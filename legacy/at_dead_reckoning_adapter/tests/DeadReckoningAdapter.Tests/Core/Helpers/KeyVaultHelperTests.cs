using Azure;
using Azure.Identity;
using Azure.Security.KeyVault.Secrets;
using DeadReckoningAdapter.Core.Helpers;
using DeadReckoningAdapter.Models;
using Moq;
using System.Reflection;

namespace DeadReckoningAdapter.Tests.Core.Helpers
{
    public class KeyVaultHelperTests
    {
        private readonly Mock<SecretClient> _mockSecretClient;
        private readonly KeyVaultHelper<ConfluentSecret> _keyVaultHelper;
        private readonly IEnumerable<string> _secretNames;
        private readonly Dictionary<string, KeyVaultSecret> _secrets;
        private readonly KeyVaultSettings _settings;

        public KeyVaultHelperTests()
        {
            _mockSecretClient = new Mock<SecretClient>(new Uri("https://dummy.vault.azure.net"), new DefaultAzureCredential());
            _secretNames = new List<string> { "confluent-key", "schema-key" };
            _secrets = new Dictionary<string, KeyVaultSecret>
            {
                { "confluent-key", new KeyVaultSecret("confluent-key", @"{  ""key"": ""key1"",  ""secret"": ""secret1""}")},
                { "schema-key", new KeyVaultSecret("schema-key", @"{  ""key"": ""key2"",  ""secret"": ""secret2""}")}
            };

            _settings = new KeyVaultSettings
            {
                KeyVault = "dummy",
                ConfluentSecretName = "confluent-key",
                SchemaEndpoint = "https://schemaendpoint.com",
                SchemaSecretName = "realtime-schema-registry-key"
            };

            _keyVaultHelper = new KeyVaultHelper<ConfluentSecret>(_settings, _secretNames);

            // Set private SecretClient property using reflection
            var secretClientField = typeof(KeyVaultHelper<ConfluentSecret>).GetField("_client", BindingFlags.NonPublic | BindingFlags.Instance);
            secretClientField?.SetValue(_keyVaultHelper, _mockSecretClient.Object);
        }

        [Fact]
        public async Task InitializeAsync_LoadsSecrets()
        {
            // Arrange
            SetupGetSecretAsync();

            // Act
            await _keyVaultHelper.InitializeAsync();

            // Assert
            Assert.Equal("secret1", _keyVaultHelper.GetSecretValue("confluent-key").Secret);
            Assert.Equal("secret2", _keyVaultHelper.GetSecretValue("schema-key").Secret);
        }

        [Fact]
        public async Task HaveSecretsChangedAsync_DetectsChanges()
        {
            // Arrange
            var expectedResults = new Dictionary<string, bool>
            {
                { "confluent-key", true },
                { "schema-key", false }
            };
            SetupGetSecretAsync();
            await _keyVaultHelper.InitializeAsync();

            // Change version of one secret
            var newVersionSecret = new KeyVaultSecret("confluent-key", @"{  ""key"": ""key3"",  ""secret"": ""secret3""}");
            _mockSecretClient.Setup(client => client.GetSecretAsync("confluent-key", It.IsAny<string>(), default)).ReturnsAsync(Response.FromValue(newVersionSecret, null!));

            // Act
            var result = await _keyVaultHelper.HaveSecretsChangedAsync();

            // Assert
            Assert.Equal(expectedResults.Count, result.Count);
            Assert.True(result.ContainsKey("confluent-key"));
            Assert.True(result.ContainsKey("schema-key"));
            Assert.Equal(expectedResults["confluent-key"], result["confluent-key"]);
            Assert.Equal(expectedResults["schema-key"], result["schema-key"]);
        }

        [Fact]
        public async Task GetSecretValue_WhenSecretNotFound_ThrowsException()
        {
            // Arrange
            SetupGetSecretAsync();
            await _keyVaultHelper.InitializeAsync();

            // Act & Assert
            Assert.Throws<KeyNotFoundException>(() => _keyVaultHelper.GetSecretValue("nonexistent_secret"));
        }

        private void SetupGetSecretAsync()
        {
            foreach (var secret in _secrets)
            {
                var secretValue = secret.Value;
                _mockSecretClient
                    .Setup(client => client.GetSecretAsync(secret.Key, It.IsAny<string>(), default))
                    .ReturnsAsync(Response.FromValue(secretValue, null!));
            }
        }
    }
}
