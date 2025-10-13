using System.Text;
using Confluent.Kafka;
using Confluent.SchemaRegistry;
using Confluent.SchemaRegistry.Serdes;
using DeadReckoningAdapter.Core.Helpers;
using DeadReckoningAdapter.Core.Services;
using DeadReckoningAdapter.Models;
using Newtonsoft.Json;
using NJsonSchema.Validation;

namespace DeadReckoningAdapter.Core.Kafka
{
    public interface IKafkaProducer<in T>
    {
        Task ProduceMessage(string key, T value);
    }

    public class KafkaProducer<T> : IKafkaProducer<T>, IDisposable where T : class
    {
        private IProducer<string, byte[]>? _producer;
        private readonly string _topic;
        private readonly ILogger<KafkaProducer<T>> _logger;
        private readonly IKeyVaultHelper<ConfluentSecret> _keyVaultHelper;
        private readonly string _confluentSecretName;
        private readonly KafkaSettings _kafkaSettings;
        private readonly string _schemaSecretName;
        private readonly HealthState _healthState;
        private readonly ISchemaRegistryClient? _schemaRegistry;
        private Schema? _schema;
        private readonly ConfluentPartitioner _partitioner = new();
        private readonly INewRelicService _newRelicService;

        public KafkaProducer(
            KafkaSettings settings,
            KeyVaultSettings keyVaultSettings,
            ILogger<KafkaProducer<T>> logger,
            IKeyVaultHelper<ConfluentSecret> keyVaultHelper,
            ISchemaRegistry schemaRegistry,
            INewRelicService newRelicService,
            HealthState healthState)
        {
            _logger = logger;
            _keyVaultHelper = keyVaultHelper;
            _kafkaSettings = settings;
            _confluentSecretName = keyVaultSettings.ConfluentSecretName;
            _schemaSecretName = keyVaultSettings.SchemaSecretName;
            _topic = SetTopicName(_kafkaSettings.ProducerSettings.Topic, _kafkaSettings.ConfluentEnvPrefix);
            _schemaRegistry = schemaRegistry.GetSchemaRegistry();
            _newRelicService = newRelicService;
            _healthState = healthState;
            keyVaultHelper.SecretChanged += (sender, secretName) =>
            {
                if (secretName == _confluentSecretName || secretName == _schemaSecretName)
                {
                    BuildProducer(_kafkaSettings);
                }
            };

            BuildProducer(settings);
        }

        public async Task InitialiseSchemaRegistry()
        {
            if (_schemaRegistry == null)
            {
                throw new Exception("Schema Registry is not set");
            }

            _schema = await _schemaRegistry.GetLatestSchemaAsync(_topic + "-value");
        }

        public async Task<JsonSerializer<T>> FetchSerializer()
        {
            await InitialiseSchemaRegistry();

            var jsonSerializerConfig = new JsonSerializerConfig
            {
                AutoRegisterSchemas = false,
                UseLatestVersion = true
            };

            return new JsonSerializer<T>(_schemaRegistry, jsonSerializerConfig);
        }

        private static string SetTopicName(string topic, string env)
        {
            return $"{env}{topic}";
        }

        private async void BuildProducer(KafkaSettings settings)
        {
            var secret = _keyVaultHelper.GetSecretValue(_confluentSecretName);

            // commented out until Confluent provide answer why this is failing when trying to produce messages
            // var serializer = await FetchSerializer();
            await InitialiseSchemaRegistry();

            var config = new ProducerConfig
            {
                BootstrapServers = settings.BootstrapServers,
                SecurityProtocol = SecurityProtocol.SaslSsl,
                SaslMechanism = SaslMechanism.Plain,
                SaslUsername = secret.Key,
                SaslPassword = secret.Secret
            };

            _producer = new ProducerBuilder<string, byte[]>(config)
            .Build();

            _logger.LogInformation("Starting Producer...");
            _healthState.ProducerIsReady = true;
        }

        public async Task ProduceMessage(string key, T value)
        {
            if (_producer == null)
            {
                throw new Exception("Producer has not been initialised");
            }
            else if (_schemaRegistry == null)
            {
                throw new Exception("Schema Registry has not been initialised");
            }

            try
            {
                var message = await SerializeAndValidateMessage(value);

                var partition = _partitioner.FetchPartition(key);

                await _producer.ProduceAsync(new TopicPartition(_topic, partition), new Message<string, byte[]> { Key = key, Value = message });
                _producer.Flush(TimeSpan.FromSeconds(10));
                
                _newRelicService.IncrementProduceTopicMetric(_topic);

            }
            catch (ProduceException<string, byte[]> e)
            {
                _logger.LogError($"Error when producing : {e.Error.Reason}");
            }
            catch (Exception ex)
            {
                _logger.LogError($"Error when producing : {ex.Message}");
            }
        }

        // This method can be removed when Confluent provid a resoulution to use their own serializer
        private async Task<byte[]> SerializeAndValidateMessage(T message)
        {
            if (_schema == null)
            {
                throw new Exception("Schema has not been initialised");
            }

            string jsonPayload = JsonConvert.SerializeObject(message);

            JsonSchemaValidator validator = new JsonSchemaValidator();
            var njsonSchema = await NJsonSchema.JsonSchema.FromJsonAsync(_schema.SchemaString);
            ICollection<ValidationError> collection = validator.Validate(jsonPayload, njsonSchema);

            if (collection.Count > 0)
            {
                throw new Exception("Message does not match schema. Json validation failed", new AggregateException(collection.Select(x => new Exception(x.ToString()))));
            }

            byte[] jsonBytes = Encoding.UTF8.GetBytes(jsonPayload);

            using (var memoryStream = new MemoryStream())
            {
                // Write magic byte (1 byte)
                memoryStream.WriteByte(0x0);

                // Write schema ID (4 bytes, big-endian)
                byte[] schemaIdBytes = BitConverter.GetBytes(_schema.Id);
                if (BitConverter.IsLittleEndian)
                {
                    Array.Reverse(schemaIdBytes);
                }
                memoryStream.Write(schemaIdBytes, 0, schemaIdBytes.Length);

                // Write the JSON payload
                memoryStream.Write(jsonBytes, 0, jsonBytes.Length);

                // Return the serialized message as a byte array
                return memoryStream.ToArray();
            }
        }

        public void Dispose()
        {
            _healthState.ProducerIsReady = false;
            _producer?.Dispose();
        }
    }
}