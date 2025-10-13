using Confluent.Kafka;
using Confluent.SchemaRegistry;
using DeadReckoningAdapter.Core.Helpers;
using DeadReckoningAdapter.Core.Kafka;
using DeadReckoningAdapter.Core.Services;
using DeadReckoningAdapter.Models;
using DeadReckoningAdapter.Tests.TestModels;
using Microsoft.Extensions.Logging;
using Moq;
using System.Reflection;

namespace DeadReckoningAdapter.Tests.Core.Kafka
{
    public class KafkaProducerTests
    {
        private readonly Mock<ILogger<KafkaProducer<TestMessage>>> _mockLogger;
        private readonly Mock<IKeyVaultHelper<ConfluentSecret>> _mockKeyVaultHelper;
        private readonly KafkaSettings _kafkaSettings;
        private readonly KeyVaultSettings _keyVaultSettings;
        private readonly HealthState _healthState;

        public KafkaProducerTests()
        {
            _mockLogger = new Mock<ILogger<KafkaProducer<TestMessage>>>();
            _mockKeyVaultHelper = new Mock<IKeyVaultHelper<ConfluentSecret>>();
            _kafkaSettings = new KafkaSettings
            {
                BootstrapServers = "localhost:9092",
                ConfluentEnvPrefix = "dev-",
                ProducerSettings = new ProducerSettings() { Topic = "test-topic" },
                ConsumerSettings = new ConsumerSettings() { Topics = ["test-topic1", "test-topic2"], ConsumerGroup = "test-consumerGroup", BatchSize = 200, GroupPrefix = "test-groupPrefix" }

            };
            _keyVaultSettings = new KeyVaultSettings
            {
                ConfluentSecretName = "realtime-confluent-key",
                KeyVault = "keyvault",
                SchemaEndpoint = "https://schemaendpoint.com",
                SchemaSecretName = "realtime-schema-registry-key"
            };
            _healthState = new HealthState();
        }

        [Fact]
        public void Constructor_ConfigsValidAndUpdated_ShouldSetupProducerAndRespondToSecretChanges()
        {
            // Arrange
            var secretName = _keyVaultSettings.ConfluentSecretName;
            var schemaSecretName = _keyVaultSettings.SchemaSecretName;
            var confluentSecret = new ConfluentSecret { Key = "test-key", Secret = "test-secret" };
            var schemaSecret = new ConfluentSecret { Key = "schema-key", Secret = "schema-secret" };
            _mockKeyVaultHelper.Setup(x => x.GetSecretValue(secretName)).Returns(confluentSecret);
            _mockKeyVaultHelper.Setup(x => x.GetSecretValue(schemaSecretName)).Returns(schemaSecret);

            var mockClient = new MockSchemaRegistryClient();
            var schemaString = "{\"type\":\"record\",\"name\":\"VehiclePositionMessage\",\"fields\":[{\"name\":\"exampleField\",\"type\":\"string\"}]}";
            var schema = new RegisteredSchema("dev-test-topic-value", 1, 111, schemaString, SchemaType.Json, new List<SchemaReference>());
            mockClient.AddSchema(111, schema);
            var mockNewRelicService = new Mock<INewRelicService>();
            var mockSchemaRegistry = new Mock<ISchemaRegistry>();
            mockSchemaRegistry
                .Setup(sr => sr.GetSchemaRegistry())
                .Returns(mockClient);

            // Act
            var kafkaProducer = new KafkaProducer<TestMessage>(_kafkaSettings, _keyVaultSettings, _mockLogger.Object, _mockKeyVaultHelper.Object, mockSchemaRegistry.Object, mockNewRelicService.Object, _healthState);

            // Trigger the event manually
            _mockKeyVaultHelper.Raise(m => m.SecretChanged += null, this, secretName);

            // Assert
            _mockKeyVaultHelper.Verify(x => x.GetSecretValue(secretName), Times.Exactly(2));
        }

        [Fact]
        public async Task ProduceMessage_ValidConfigs_ShouldProduceMessageCorrectly()
        {
            // Arrange
            var mockLog = new Mock<ILogger<KafkaProducer<TestMessage>>>();
            var secretName = _keyVaultSettings.ConfluentSecretName;
            var schemaSecretName = _keyVaultSettings.SchemaSecretName;
            var confluentSecret = new ConfluentSecret { Key = "test-key", Secret = "test-secret" };
            var schemaSecret = new ConfluentSecret { Key = "schema-key", Secret = "schema-secret" };
            _mockKeyVaultHelper.Setup(x => x.GetSecretValue(secretName)).Returns(confluentSecret);
            _mockKeyVaultHelper.Setup(x => x.GetSecretValue(schemaSecretName)).Returns(schemaSecret);

            var mockClient = new MockSchemaRegistryClient();
            var schemaString = "{\"type\":\"record\",\"name\":\"VehiclePositionMessage\",\"fields\":[{\"name\":\"exampleField\",\"type\":\"string\"}]}";
            var schema = new RegisteredSchema("dev-test-topic-value", 1, 111, schemaString, SchemaType.Json, new List<SchemaReference>());
            mockClient.AddSchema(111, schema);
            var mockNewRelicService = new Mock<INewRelicService>();
            var mockSchemaRegistry = new Mock<ISchemaRegistry>();
            mockSchemaRegistry
                .Setup(sr => sr.GetSchemaRegistry())
                .Returns(mockClient);

            var mockProducer = new Mock<IProducer<string, byte[]>>();
            mockProducer.Setup(p => p.ProduceAsync(It.IsAny<TopicPartition>(), It.IsAny<Message<string, byte[]>>(), default))
                        .ReturnsAsync(new DeliveryResult<string, byte[]>());

            IKafkaProducer<TestMessage> kafkaProducer = new KafkaProducer<TestMessage>(_kafkaSettings, _keyVaultSettings, mockLog.Object, _mockKeyVaultHelper.Object, mockSchemaRegistry.Object, mockNewRelicService.Object, _healthState);

            // Override _producer field using reflection
            var producerField = typeof(KafkaProducer<TestMessage>).GetField("_producer", BindingFlags.NonPublic | BindingFlags.Instance);
            producerField!.SetValue(kafkaProducer, mockProducer.Object);

            var testMessage = new TestMessage { Content = "Hello, World!" };
            var key = "test-key";

            // Act
            await kafkaProducer.ProduceMessage(key, testMessage);

            // Assert
            mockProducer.Verify(
                p => p.ProduceAsync(
                    It.Is<TopicPartition>(tp => tp.Topic == "dev-test-topic" && tp.Partition == 10),
                    It.Is<Message<string, byte[]>>(m => m.Key == key && m.Value is byte[]), default),
                    Times.Once);
        }

        [Fact]
        public async Task ProduceMessage_ValidConfigs_ShouldThrowExceptionOnProduce()
        {
            // Arrange
            var mockLog = new Mock<ILogger<KafkaProducer<TestMessage>>>();
            var secretName = _keyVaultSettings.ConfluentSecretName;
            var schemaSecretName = _keyVaultSettings.SchemaSecretName;
            var confluentSecret = new ConfluentSecret { Key = "test-key", Secret = "test-secret" };
            var schemaSecret = new ConfluentSecret { Key = "schema-key", Secret = "schema-secret" };
            _mockKeyVaultHelper.Setup(x => x.GetSecretValue(secretName)).Returns(confluentSecret);
            _mockKeyVaultHelper.Setup(x => x.GetSecretValue(schemaSecretName)).Returns(schemaSecret);

            var mockClient = new MockSchemaRegistryClient();
            var schemaString = "{\"type\":\"record\",\"name\":\"VehiclePositionMessage\",\"fields\":[{\"name\":\"exampleField\",\"type\":\"string\"}]}";
            var schema = new RegisteredSchema("dev-test-topic-value", 1, 111, schemaString, SchemaType.Json, new List<SchemaReference>());
            mockClient.AddSchema(111, schema);
            var mockNewRelicService = new Mock<INewRelicService>();
            var mockSchemaRegistry = new Mock<ISchemaRegistry>();
            mockSchemaRegistry
                .Setup(sr => sr.GetSchemaRegistry())
                .Returns(mockClient);

            var mockProducer = new Mock<IProducer<string, byte[]>>();
            mockProducer.Setup(p => p.ProduceAsync(It.IsAny<TopicPartition>(), It.IsAny<Message<string, byte[]>>(), default))
                        .ThrowsAsync(new ProduceException<string, byte[]>(new Error(ErrorCode.Local_BadMsg), new DeliveryResult<string, byte[]>()));

            var kafkaProducer = new KafkaProducer<TestMessage>(_kafkaSettings, _keyVaultSettings, mockLog.Object, _mockKeyVaultHelper.Object, mockSchemaRegistry.Object, mockNewRelicService.Object, _healthState);

            // Override _producer field using reflection
            var producerField = typeof(KafkaProducer<TestMessage>).GetField("_producer", BindingFlags.NonPublic | BindingFlags.Instance);
            producerField!.SetValue(kafkaProducer, mockProducer.Object);

            var testMessage = new TestMessage { Content = "Hello, World!" };
            var key = "test-key";

            // Act
            await kafkaProducer.ProduceMessage(key, testMessage);

            // Assert
            mockLog.Verify(
                logger => logger.Log(
                    LogLevel.Error,
                    It.IsAny<EventId>(),
                    It.Is<It.IsAnyType>((v, t) => v.ToString().Contains("Bad message format")),
                    It.IsAny<Exception>(),
                    It.IsAny<Func<It.IsAnyType, Exception, string>>()),
                Times.Once);
        }

        [Fact]
        public void Dispose_ShouldDisposeProducer()
        {
            // Arrange
            var secretName = _keyVaultSettings.ConfluentSecretName;
            var schemaSecretName = _keyVaultSettings.SchemaSecretName;
            var confluentSecret = new ConfluentSecret { Key = "test-key", Secret = "test-secret" };
            var schemaSecret = new ConfluentSecret { Key = "schema-key", Secret = "schema-secret" };
            _mockKeyVaultHelper.Setup(x => x.GetSecretValue(secretName)).Returns(confluentSecret);
            _mockKeyVaultHelper.Setup(x => x.GetSecretValue(schemaSecretName)).Returns(schemaSecret);
            var mockProducer = new Mock<IProducer<string, byte[]>>();

            var mockClient = new MockSchemaRegistryClient();
            var schemaString = "{\"type\":\"record\",\"name\":\"VehiclePositionMessage\",\"fields\":[{\"name\":\"exampleField\",\"type\":\"string\"}]}";
            var schema = new RegisteredSchema("dev-test-topic-value", 1, 111, schemaString, SchemaType.Json, new List<SchemaReference>());
            mockClient.AddSchema(111, schema);
            var mockNewRelicService = new Mock<INewRelicService>();
            var mockSchemaRegistry = new Mock<ISchemaRegistry>();
            mockSchemaRegistry
                .Setup(sr => sr.GetSchemaRegistry())
                .Returns(mockClient);

            var kafkaProducer = new KafkaProducer<TestMessage>(_kafkaSettings, _keyVaultSettings, _mockLogger.Object, _mockKeyVaultHelper.Object, mockSchemaRegistry.Object, mockNewRelicService.Object, _healthState);
            var producerField = typeof(KafkaProducer<TestMessage>).GetField("_producer", BindingFlags.NonPublic | BindingFlags.Instance);
            producerField!.SetValue(kafkaProducer, mockProducer.Object);

            // Act
            kafkaProducer.Dispose();

            // Assert
            mockProducer.Verify(p => p.Dispose(), Times.Once);
            Assert.False(_healthState.ProducerIsReady);
        }

        [Fact]
        public async Task ProduceMessage_ValidConfigs_ShouldThrowGeneralExceptionOnProduce()
        {
            // Arrange
            var mockLog = new Mock<ILogger<KafkaProducer<TestMessage>>>();
            var secretName = _keyVaultSettings.ConfluentSecretName;
            var schemaSecretName = _keyVaultSettings.SchemaSecretName;
            var confluentSecret = new ConfluentSecret { Key = "test-key", Secret = "test-secret" };
            var schemaSecret = new ConfluentSecret { Key = "schema-key", Secret = "schema-secret" };
            _mockKeyVaultHelper.Setup(x => x.GetSecretValue(secretName)).Returns(confluentSecret);
            _mockKeyVaultHelper.Setup(x => x.GetSecretValue(schemaSecretName)).Returns(schemaSecret);

            var mockClient = new MockSchemaRegistryClient();
            var schemaString = "{\"type\":\"record\",\"name\":\"VehiclePositionMessage\",\"fields\":[{\"name\":\"exampleField\",\"type\":\"string\"}]}";
            var schema = new RegisteredSchema("dev-test-topic-value", 1, 111, schemaString, SchemaType.Json, new List<SchemaReference>());
            mockClient.AddSchema(111, schema);
            var mockNewRelicService = new Mock<INewRelicService>();
            var mockSchemaRegistry = new Mock<ISchemaRegistry>();
            mockSchemaRegistry
                .Setup(sr => sr.GetSchemaRegistry())
                .Returns(mockClient);

            var mockProducer = new Mock<IProducer<string, byte[]>>();
            mockProducer.Setup(p => p.ProduceAsync(It.IsAny<TopicPartition>(), It.IsAny<Message<string, byte[]>>(), default))
                        .ThrowsAsync(new Exception("General exception"));

            var kafkaProducer = new KafkaProducer<TestMessage>(_kafkaSettings, _keyVaultSettings, mockLog.Object, _mockKeyVaultHelper.Object, mockSchemaRegistry.Object, mockNewRelicService.Object, _healthState);

            // Override _producer field using reflection
            var producerField = typeof(KafkaProducer<TestMessage>).GetField("_producer", BindingFlags.NonPublic | BindingFlags.Instance);
            producerField!.SetValue(kafkaProducer, mockProducer.Object);

            var testMessage = new TestMessage { Content = "Hello, World!" };
            var key = "test-key";

            // Act
            await kafkaProducer.ProduceMessage(key, testMessage);

            // Assert
            mockLog.Verify(
                logger => logger.Log(
                    LogLevel.Error,
                    It.IsAny<EventId>(),
                    It.Is<It.IsAnyType>((v, t) => v.ToString().Contains("General exception")),
                    It.IsAny<Exception>(),
                    It.IsAny<Func<It.IsAnyType, Exception, string>>()),
                Times.Once);
        }

        [Fact]
        public async Task ProduceMessage_ShouldThrowExceptionWhenProducerIsNull()
        {
            // Arrange
            var mockLog = new Mock<ILogger<KafkaProducer<TestMessage>>>();
            var secretName = _keyVaultSettings.ConfluentSecretName;
            var schemaSecretName = _keyVaultSettings.SchemaSecretName;
            var confluentSecret = new ConfluentSecret { Key = "test-key", Secret = "test-secret" };
            var schemaSecret = new ConfluentSecret { Key = "schema-key", Secret = "schema-secret" };
            _mockKeyVaultHelper.Setup(x => x.GetSecretValue(secretName)).Returns(confluentSecret);
            _mockKeyVaultHelper.Setup(x => x.GetSecretValue(schemaSecretName)).Returns(schemaSecret);

            var mockClient = new MockSchemaRegistryClient();
            var schemaString = "{\"type\":\"record\",\"name\":\"VehiclePositionMessage\",\"fields\":[{\"name\":\"exampleField\",\"type\":\"string\"}]}";
            var schema = new RegisteredSchema("dev-test-topic-value", 1, 111, schemaString, SchemaType.Json, new List<SchemaReference>());
            mockClient.AddSchema(111, schema);
            var mockNewRelicService = new Mock<INewRelicService>();
            var mockSchemaRegistry = new Mock<ISchemaRegistry>();
            mockSchemaRegistry
                .Setup(sr => sr.GetSchemaRegistry())
                .Returns(mockClient);

            var kafkaProducer = new KafkaProducer<TestMessage>(_kafkaSettings, _keyVaultSettings, mockLog.Object, _mockKeyVaultHelper.Object, mockSchemaRegistry.Object, mockNewRelicService.Object, _healthState);

            // Override _producer field using reflection
            var producerField = typeof(KafkaProducer<TestMessage>).GetField("_producer", BindingFlags.NonPublic | BindingFlags.Instance);
            producerField!.SetValue(kafkaProducer, null);

            var testMessage = new TestMessage { Content = "Hello, World!" };
            var key = "test-key";

            // Act
            await Assert.ThrowsAsync<Exception>(() => kafkaProducer.ProduceMessage(key, testMessage));
        }

        [Fact]
        public async Task ProduceMessage_ShouldThrowExceptionWhenSchemaRegistryIsNull()
        {
            // Arrange
            var mockLog = new Mock<ILogger<KafkaProducer<TestMessage>>>();
            var secretName = _keyVaultSettings.ConfluentSecretName;
            var schemaSecretName = _keyVaultSettings.SchemaSecretName;
            var confluentSecret = new ConfluentSecret { Key = "test-key", Secret = "test-secret" };
            var schemaSecret = new ConfluentSecret { Key = "schema-key", Secret = "schema-secret" };
            _mockKeyVaultHelper.Setup(x => x.GetSecretValue(secretName)).Returns(confluentSecret);
            _mockKeyVaultHelper.Setup(x => x.GetSecretValue(schemaSecretName)).Returns(schemaSecret);
            var mockNewRelicService = new Mock<INewRelicService>();

            var mockSchemaRegistry = new Mock<ISchemaRegistry>();
            mockSchemaRegistry
                .Setup(sr => sr.GetSchemaRegistry())
                .Returns<ISchemaRegistryClient>(null);

            var mockProducer = new Mock<IProducer<string, byte[]>>();
            mockProducer.Setup(p => p.ProduceAsync(It.IsAny<string>(), It.IsAny<Message<string, byte[]>>(), default))
                        .ThrowsAsync(new ProduceException<string, byte[]>(new Error(ErrorCode.Local_BadMsg), new DeliveryResult<string, byte[]>()));

            var kafkaProducer = new KafkaProducer<TestMessage>(_kafkaSettings, _keyVaultSettings, mockLog.Object, _mockKeyVaultHelper.Object, mockSchemaRegistry.Object, mockNewRelicService.Object, _healthState);

            // Override _producer field using reflection
            var producerField = typeof(KafkaProducer<TestMessage>).GetField("_producer", BindingFlags.NonPublic | BindingFlags.Instance);
            producerField!.SetValue(kafkaProducer, mockProducer.Object);

            var testMessage = new TestMessage { Content = "Hello, World!" };
            var key = "test-key";

            // Act
            await Assert.ThrowsAsync<Exception>(() => kafkaProducer.ProduceMessage(key, testMessage));

        }
    }
}
