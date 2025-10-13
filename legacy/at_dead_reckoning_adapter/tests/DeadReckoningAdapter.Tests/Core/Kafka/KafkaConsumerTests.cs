using Confluent.Kafka;
using DeadReckoningAdapter.Core.Helpers;
using DeadReckoningAdapter.Core.Kafka;
using DeadReckoningAdapter.Core.Processors;
using DeadReckoningAdapter.Core.Services;
using DeadReckoningAdapter.Models;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using Moq;
using System.Reflection;

namespace DeadReckoningAdapter.Tests.Core.Kafka
{
    public class KafkaConsumerTests
    {
        private readonly Mock<ILogger<KafkaConsumer>> _mockLogger;
        private readonly Mock<IKeyVaultHelper<ConfluentSecret>> _mockKeyVaultHelper;
        private readonly KafkaSettings _kafkaSettings;
        private readonly KeyVaultSettings _keyVaultSettings;
        private readonly Mock<IMessageProcessor> _mockMessageProcessor;
        private readonly Mock<IServiceScopeFactory> _mockScopeFactory;
        private readonly IOptions<KafkaSettings> _kafkaSettingsOptions;
        private readonly IOptions<KeyVaultSettings> _keyVaultSettingsOptions;
        private readonly HealthState _healthState;


        public KafkaConsumerTests()
        {
            _mockLogger = new Mock<ILogger<KafkaConsumer>>();
            _mockKeyVaultHelper = new Mock<IKeyVaultHelper<ConfluentSecret>>();
            _kafkaSettings = new KafkaSettings
            {
                BootstrapServers = "localhost:9092",
                ConfluentEnvPrefix = "dev-",
                ProducerSettings = new ProducerSettings() { Topic = "test-topic" },
                ConsumerSettings = new ConsumerSettings() { Topics = ["test-topic1", "test-topic2"], ConsumerGroup = "test-consumerGroup", BatchSize = 200, GroupPrefix = "test-groupPrefix" }

            };
            _kafkaSettingsOptions = Options.Create(_kafkaSettings);
            _keyVaultSettings = new KeyVaultSettings
            {
                ConfluentSecretName = "realtime-confluent-key",
                KeyVault = "keyvault",
                SchemaEndpoint = "https://schemaendpoint.com",
                SchemaSecretName = "realtime-schema-registry-key"
            };
            _keyVaultSettingsOptions = Options.Create(_keyVaultSettings);

            _mockScopeFactory = new Mock<IServiceScopeFactory>();
            var mockServiceScope = new Mock<IServiceScope>();
            var mockServiceProvider = new Mock<IServiceProvider>();
            _mockMessageProcessor = new Mock<IMessageProcessor>();

            mockServiceProvider.Setup(sp => sp.GetService(typeof(IMessageProcessor)))
                               .Returns(_mockMessageProcessor.Object);

            mockServiceScope.Setup(s => s.ServiceProvider)
                            .Returns(mockServiceProvider.Object);

            _mockScopeFactory.Setup(f => f.CreateScope())
                                .Returns(mockServiceScope.Object);

            _healthState = new HealthState();
        }

        [Fact]
        public void Constructor_ConfigsValidAndUpdated_ShouldSetupConsumerAndRespondToSecretChanges()
        {
            // Arrange
            var secretName = _keyVaultSettings.ConfluentSecretName;
            var schemaSecretName = _keyVaultSettings.SchemaSecretName;
            var confluentSecret = new ConfluentSecret { Key = "test-key", Secret = "test-secret" };
            var schemaSecret = new ConfluentSecret { Key = "schema-key", Secret = "schema-secret" };
            _mockKeyVaultHelper.Setup(x => x.GetSecretValue(secretName)).Returns(confluentSecret);
            _mockKeyVaultHelper.Setup(x => x.GetSecretValue(schemaSecretName)).Returns(schemaSecret);
            var mockSchemaRegistry = new Mock<ISchemaRegistry>();
            var mockNewRelicService = new Mock<INewRelicService>();

            // Act
            var kafkaConsumer = new KafkaConsumer(_kafkaSettingsOptions, _keyVaultSettingsOptions, _mockLogger.Object, _mockKeyVaultHelper.Object, _mockScopeFactory.Object, mockSchemaRegistry.Object, mockNewRelicService.Object, _healthState);

            // Trigger the event manually
            _mockKeyVaultHelper.Raise(m => m.SecretChanged += null, this, secretName);

            // Assert
            _mockKeyVaultHelper.Verify(x => x.GetSecretValue(secretName), Times.Exactly(1));
        }

        [Fact]
        public void Dispose_ShouldDisposeConsumer()
        {
            // Arrange
            var secretName = _keyVaultSettings.ConfluentSecretName;
            var schemaSecretName = _keyVaultSettings.SchemaSecretName;
            var confluentSecret = new ConfluentSecret { Key = "test-key", Secret = "test-secret" };
            var schemaSecret = new ConfluentSecret { Key = "schema-key", Secret = "schema-secret" };
            _mockKeyVaultHelper.Setup(x => x.GetSecretValue(secretName)).Returns(confluentSecret);
            _mockKeyVaultHelper.Setup(x => x.GetSecretValue(schemaSecretName)).Returns(schemaSecret);
            var mockConsumer = new Mock<IConsumer<string, object>>();
            var mockSchemaRegistry = new Mock<ISchemaRegistry>();
            var mockNewRelicService = new Mock<INewRelicService>();

            // Act
            var kafkaConsumer = new KafkaConsumer(_kafkaSettingsOptions, _keyVaultSettingsOptions, _mockLogger.Object, _mockKeyVaultHelper.Object, _mockScopeFactory.Object, mockSchemaRegistry.Object, mockNewRelicService.Object, _healthState);
            var consumerField = typeof(KafkaConsumer).GetField("_consumer", BindingFlags.NonPublic | BindingFlags.Instance);
            consumerField!.SetValue(kafkaConsumer, mockConsumer.Object);
            kafkaConsumer.Dispose();

            // Assert
            mockConsumer.Verify(c => c.Close(), Times.Once);
            mockConsumer.Verify(c => c.Dispose(), Times.Once);
        }
    }
}
