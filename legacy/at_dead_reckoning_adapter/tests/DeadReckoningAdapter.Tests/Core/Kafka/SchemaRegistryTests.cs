using Confluent.SchemaRegistry;
using DeadReckoningAdapter.Core.Helpers;
using DeadReckoningAdapter.Core.Kafka;
using DeadReckoningAdapter.Models;
using Moq;

namespace DeadReckoningAdapter.Tests.Core.Kafka {
// test schemaRegistry class
    public class SchemaRegistryTests
    {
        private readonly Mock<IKeyVaultHelper<ConfluentSecret>> _mockKeyVaultHelper;
        private readonly Mock<ISchemaRegistryClient> _mockSchemaRegistryClient;
        private readonly Mock<KeyVaultSettings> _mockKeyVaultSettings;
        private readonly SchemaRegistry _schemaRegistry;

        public SchemaRegistryTests()
        {
            _mockKeyVaultHelper = new Mock<IKeyVaultHelper<ConfluentSecret>>();
            _mockSchemaRegistryClient = new Mock<ISchemaRegistryClient>();
            _mockKeyVaultSettings = new Mock<KeyVaultSettings>();
            _schemaRegistry = new SchemaRegistry(_mockKeyVaultHelper.Object, _mockKeyVaultSettings.Object);
        }

        [Fact]
        public void GetSchemaRegistry_ShouldReturnSchemaRegistry()
        {
            // Arrange
            _schemaRegistry.GetType().GetField("_schemaRegistry", System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance).SetValue(_schemaRegistry, _mockSchemaRegistryClient.Object);
            

            // Act
            var result = _schemaRegistry.GetSchemaRegistry();

            // Assert
            Assert.NotNull(result);
        }

        [Fact]
        public void GetSchemaRegistry_WhenSchemaRegistryNotSet_ShouldThrowException()
        {
            // Act
            var exception = Assert.Throws<Exception>(() => _schemaRegistry.GetSchemaRegistry());

            // Assert
            Assert.Equal("Schema Registry is not set", exception.Message);
        }
    }

}