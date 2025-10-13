import { KeyVault } from "./key-vault";

export class ConfluentSecretRetriever {
    private static confluentKeyName = process.env.KEY_VAULT_SECRET_NAME_CONFLUENT_KEY || "realtime-confluent-key";
    private static schemaKeyName = process.env.KEY_VAULT_SECRET_NAME_SCHEMA_REGISTRY_KEY || "realtime-schema-registry-key";
    private static keyVaultUrl = "https://" + process.env.KEY_VAULT + ".vault.azure.net";
    private static confluentSecret = "";
    private static schemaSecret = "";
    private static secretWatcherInterval = Number(process.env.SECRET_WATCHER_INTERVAL) || 5;

    public static confluentKafkaSecret = {
        username: "",
        password: "",
    };

    public static schemaRegistrySecret = {
        apiKey: "",
        apiSecret: "",
    };

    public static async retrieveAndWatch(closeApp: () => void): Promise<void> {
        if (this.confluentKafkaSecret.password && this.schemaRegistrySecret.apiSecret) {
            return;
        }

        const kv = new KeyVault(this.keyVaultUrl);

        this.confluentSecret = await kv.getSecret(this.confluentKeyName);
        if (!this.confluentSecret) {
            throw new Error("No valid kafka keys");
        }
        const confluentValues = JSON.parse(this.confluentSecret);
        this.confluentKafkaSecret.username = confluentValues.key;
        this.confluentKafkaSecret.password = confluentValues.secret;

        this.schemaSecret = await kv.getSecret(this.schemaKeyName);
        if (!this.schemaSecret) {
            throw new Error("No valid schema registry keys");
        }
        const schemaValues = JSON.parse(this.schemaSecret);
        this.schemaRegistrySecret.apiKey = schemaValues.key;
        this.schemaRegistrySecret.apiSecret = schemaValues.secret;

        // we use simple restart mechanism as legacy consumer does not support rebuilding out of the box
        // and restart of the application should pick up latest version of the key without the problem
        const reloadConfigs = async () => {
            const kafkaLatestSecret = await kv.getSecret(this.confluentKeyName);
            const schemaLatestSecret = await kv.getSecret(this.schemaKeyName);
            if ((kafkaLatestSecret && kafkaLatestSecret !== this.confluentSecret) || (schemaLatestSecret && schemaLatestSecret !== this.schemaSecret)) {
                closeApp();
            }

            setTimeout(() => reloadConfigs(), this.secretWatcherInterval * 60 * 1000);
        };

        await reloadConfigs();
    }
}
