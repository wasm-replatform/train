namespace DeadReckoningAdapter.Models
{
    public class KeyVaultSettings
    {
        public required string KeyVault { get; set; }
        public required string ConfluentSecretName { get; set; }
        public required string SchemaEndpoint { get; set; }
        public required string SchemaSecretName { get; set; }
    }
}