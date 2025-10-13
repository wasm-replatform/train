using Confluent.SchemaRegistry;
using DeadReckoningAdapter.Core.Helpers;
using DeadReckoningAdapter.Models;

namespace DeadReckoningAdapter.Core.Kafka
{
    public interface ISchemaRegistry
    {
        void SetSchemaRegistry();
        ISchemaRegistryClient? GetSchemaRegistry();
    }

    public class SchemaRegistry : ISchemaRegistry {
        private readonly IKeyVaultHelper<ConfluentSecret> _keyVaultHelper;
        private readonly string _schemaSecretName;
        private readonly string _schemaEndpoint;
        private ISchemaRegistryClient? _schemaRegistry;

        public SchemaRegistry(IKeyVaultHelper<ConfluentSecret> keyVaultHelper, KeyVaultSettings keyVaultSettings) {
            _keyVaultHelper = keyVaultHelper;
            _schemaSecretName = keyVaultSettings.SchemaSecretName;
            _schemaEndpoint = keyVaultSettings.SchemaEndpoint;
        }

        public void SetSchemaRegistry() {
            var secret = _keyVaultHelper.GetSecretValue(_schemaSecretName);
            var schemaRegistryConfig = new SchemaRegistryConfig {
                Url = _schemaEndpoint,
                BasicAuthCredentialsSource = AuthCredentialsSource.UserInfo,
                BasicAuthUserInfo = $"{secret.Key}:{secret.Secret}",
            };
            _schemaRegistry = new CachedSchemaRegistryClient(schemaRegistryConfig);
        }

        public ISchemaRegistryClient GetSchemaRegistry() {
            if (_schemaRegistry == null) {
                throw new Exception("Schema Registry is not set");
            }

            return _schemaRegistry;
        }
    }
}