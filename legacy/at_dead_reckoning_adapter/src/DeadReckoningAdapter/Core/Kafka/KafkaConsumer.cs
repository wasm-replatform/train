using Confluent.Kafka;
using DeadReckoningAdapter.Core.Helpers;
using DeadReckoningAdapter.Core.Processors;
using DeadReckoningAdapter.Core.Services;
using DeadReckoningAdapter.Models;
using Microsoft.Extensions.Options;

namespace DeadReckoningAdapter.Core.Kafka
{
    public class KafkaConsumer : BackgroundService, IDisposable
    {
        private readonly ILogger<KafkaConsumer> _logger;
        private readonly KafkaSettings _kafkaSettings;
        private readonly IKeyVaultHelper<ConfluentSecret> _keyVaultHelper;
        private readonly string[] _topics;
        private readonly string _confluentSecretName;
        private readonly string _schemaSecretName;
        private readonly ISchemaRegistry? _schemaRegistry;
        private CancellationTokenSource _consumerCancellationToken;
        private readonly IServiceScopeFactory _scopeFactory;
        private IConsumer<string, object>? _consumer;
        private readonly INewRelicService _newRelicService;
        private readonly HealthState _healthState;

        public KafkaConsumer(
            IOptions<KafkaSettings> kafkaSettings,
            IOptions<KeyVaultSettings> keyVaultSettings,
            ILogger<KafkaConsumer> logger,
            IKeyVaultHelper<ConfluentSecret> keyVaultHelper,
            IServiceScopeFactory scopeFactory,
            ISchemaRegistry schemaRegistry,
            INewRelicService newRelicService,
            HealthState healthState)
        {
            _logger = logger;
            _keyVaultHelper = keyVaultHelper;
            _kafkaSettings = kafkaSettings.Value;
            _topics = _kafkaSettings.ConsumerSettings.Topics.Select(x => SetTopicName(x, _kafkaSettings.ConfluentEnvPrefix)).ToArray();

            _confluentSecretName = keyVaultSettings.Value.ConfluentSecretName;
            _schemaSecretName = keyVaultSettings.Value.SchemaSecretName;
            _schemaRegistry = schemaRegistry;
            _newRelicService = newRelicService;
            _healthState = healthState;
            _consumerCancellationToken = new CancellationTokenSource();

            keyVaultHelper.SecretChanged += (sender, secretName) =>
            {
                if (secretName == _confluentSecretName || secretName == _schemaSecretName)
                {
                    _schemaRegistry.SetSchemaRegistry();
                    BuildConsumer();
                }
            };
            _scopeFactory = scopeFactory;
        }

        private static string SetTopicName(string topic, string env)
        {
            return $"{env}{topic}";
        }

        private void BuildConsumer()
        {
            DisposeConsumer();

            if (_schemaRegistry == null)
            {
                _logger.LogError("Schema registry is null");
                throw new InvalidOperationException("Schema registry is null.");
            }

            var secret = _keyVaultHelper.GetSecretValue(_confluentSecretName);

            var config = new ConsumerConfig
            {
                BootstrapServers = _kafkaSettings.BootstrapServers,
                SecurityProtocol = SecurityProtocol.SaslSsl,
                SaslMechanism = SaslMechanism.Plain,
                SaslUsername = secret.Key,
                SaslPassword = secret.Secret,
                GroupId = _kafkaSettings.ConsumerSettings.GroupPrefix + _kafkaSettings.ConsumerSettings.ConsumerGroup,
                AutoOffsetReset = AutoOffsetReset.Latest,
                EnableAutoCommit = false,
            };

            _consumer = new ConsumerBuilder<string, object>(config)
                .SetValueDeserializer(new CustomJsonDeserializer(_schemaRegistry))
                .Build();
            _logger.LogInformation("Starting Consumer...");
            _consumer.Subscribe(_topics);
            _logger.LogInformation($"Subscribed to Kafka topics: {string.Join(", ", _topics)}");

        }

        protected override Task ExecuteAsync(CancellationToken stoppingToken)
        {
            BuildConsumer();

            return Task.Run(() => ProcessQueue(stoppingToken), stoppingToken);
        }

        private async Task ProcessQueue(CancellationToken stoppingToken)
        {
            using var scope = _scopeFactory.CreateScope();
            var messageProcessor = scope.ServiceProvider.GetRequiredService<IMessageProcessor>();
            _healthState.ConsumerIsReady = true;
            var batchSize = _kafkaSettings.ConsumerSettings.BatchSize;
            var messagesSinceLastCommit = 0;

            while (!stoppingToken.IsCancellationRequested)
            {
                if (_consumer == null)
                {
                    _logger.LogWarning("Consumer is null; waiting before retry...");
                    await Task.Delay(1000, stoppingToken);
                    continue;
                }

                try
                {
                    var consumeResult = _consumer.Consume(_consumerCancellationToken.Token);

                    if (consumeResult?.Message.Value == null)
                    {
                        continue;
                    }

                    _newRelicService.IncrementConsumeTopicMetric(consumeResult.Topic);
                    
                    if (consumeResult.Message.Value is VehiclePositionMessage vpMessageRef)
                    {
                        await messageProcessor.ProcessVPMessage(vpMessageRef);
                    }
                    else if (consumeResult.Message.Value is DeadReckoningMessage drMessageRef)
                    {
                        await messageProcessor.ProcessDRMessage(drMessageRef);
                    }

                    messagesSinceLastCommit++;

                    if (messagesSinceLastCommit >= batchSize)
                    {
                        _consumer.Commit();
                        messagesSinceLastCommit = 0;
                    }
                }
                catch (OperationCanceledException) when (_consumerCancellationToken.Token.IsCancellationRequested)
                {
                    _logger.LogInformation("Consumer cancellation requested.");
                    _consumerCancellationToken = new CancellationTokenSource();
                    BuildConsumer();
                }
                catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested)
                {
                    _logger.LogInformation("Stopping Kafka consumer service.");
                    break;
                }
                catch (ConsumeException e)
                {
                    _logger.LogError($"Error when consuming : {e.Error.Reason}");
                }
                catch (Exception ex)
                {
                    _logger.LogError($"Error when consuming : {ex.Message}");
                }
            }

            _logger.LogInformation("Exiting Kafka consumer service.");
            DisposeConsumer();
        }

        private void DisposeConsumer()
        {
            _healthState.ConsumerIsReady = false;
            _consumerCancellationToken.Cancel();
            _consumerCancellationToken.Dispose();
            _consumerCancellationToken = new CancellationTokenSource();

            if (_consumer != null)
            {
                _logger.LogInformation("Disposing Kafka consumer.");
                _consumer.Close();
                _consumer.Dispose();
                _consumer = null;
            }
        }

        public override void Dispose()
        {
            DisposeConsumer();
            base.Dispose();
        }
    }
}
