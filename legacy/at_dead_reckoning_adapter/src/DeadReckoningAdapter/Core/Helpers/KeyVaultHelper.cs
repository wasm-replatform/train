using System.Text.Json;
using Azure.Identity;
using Azure.Security.KeyVault.Secrets;
using DeadReckoningAdapter.Models;

namespace DeadReckoningAdapter.Core.Helpers
{
    public interface IKeyVaultHelper<T>
    {
        event EventHandler<string>? SecretChanged;
        Task InitializeAsync();
        T GetSecretValue(string secretName);
        Task<Dictionary<string, bool>> HaveSecretsChangedAsync();
    }

    public class KeyVaultHelper<T> : IKeyVaultHelper<T>
    {
        private readonly Dictionary<string, KeyVaultSecret> _secrets;
        private readonly SecretClient _client;
        private readonly IEnumerable<string> _secretNames;

        public event EventHandler<string>? SecretChanged;

        public KeyVaultHelper(KeyVaultSettings settings, IEnumerable<string> secretNames)
        {
            var keyVaultUrl = $"https://{settings.KeyVault}.vault.azure.net";
            _secretNames = secretNames;
            _secrets = new Dictionary<string, KeyVaultSecret>();
            _client = new SecretClient(new Uri(keyVaultUrl), new DefaultAzureCredential());
        }

        public async Task InitializeAsync()
        {
            foreach (var secretName in _secretNames)
            {
                var secret = (await _client.GetSecretAsync(secretName)).Value;
                _secrets[secretName] = secret;
            }
        }

        public T GetSecretValue(string secretName)
        {
            if (_secrets.TryGetValue(secretName, out var secret) && secret.Value != null)
            {
                var secretValue = JsonSerializer.Deserialize<T>(secret.Value);
                if (!EqualityComparer<T>.Default.Equals(secretValue, default))
                    return secretValue!;
            }

            throw new KeyNotFoundException($"Secret with name '{secretName}' not found.");
        }

        public async Task<Dictionary<string, bool>> HaveSecretsChangedAsync()
        {
            Dictionary<string, bool> secretsChanged = new Dictionary<string, bool>();

            foreach (var secretName in _secretNames)
            {

                var currentSecret = (await _client.GetSecretAsync(secretName)).Value;
                var cachedSecret = _secrets[secretName];

                bool hasChanged = currentSecret.Value != cachedSecret.Value;
                if (hasChanged)
                {
                    _secrets[secretName] = currentSecret;
                    OnSecretChanged(secretName);
                }
                secretsChanged.Add(secretName, hasChanged);
            }

            return secretsChanged;
        }

        protected virtual void OnSecretChanged(string secretName)
        {
            SecretChanged?.Invoke(this, secretName);
        }
    }
}