import { Injectable } from "@nestjs/common";
import { ISecretLoader } from "at-realtime-common/common";
import { ConfluentTopics, IKafkaProducerConfig, KafkaEnvConfig, SchemaConfig, Versions, getConfluentTopic } from "at-realtime-common/kafka";
import { KeyVault } from "at-realtime-common/keyvault";
import { EventEmitter } from "events";

@Injectable()
export class KafkaProducerConfig extends EventEmitter implements IKafkaProducerConfig, ISecretLoader {
    constructor(private keyVault: KeyVault) {
        super();
    }
    private confluentSecretName = process.env.KEY_VAULT_SECRET_NAME_CONFLUENT_KEY || "realtime-confluent-key";
    private schemaSecretName = process.env.KEY_VAULT_SECRET_NAME_SCHEMA_REGISTRY_KEY || "realtime-schema-registry-key";
    private useSchemaProducer = process.env.USE_SCHEMA_REGISTRY_PRODUCER === "true";
    private useConfluentKafkaConfig = process.env.IS_LOCAL !== "true";

    public brokers = (process.env.KAFKA_HOSTS || "").split(",");
    public ssl = this.useConfluentKafkaConfig;
    public sasl = this.useConfluentKafkaConfig
        ? {
              mechanism: "plain" as never,
              // loading values from the key vault
              username: "",
              password: "",
          }
        : undefined;
    public schema = this.useSchemaProducer
        ? ({
              url: process.env.SCHEMA_REGISTRY_URL || "",
              apiKey: "",
              apiSecret: "",
          } as SchemaConfig)
        : undefined;

    private envConfig: KafkaEnvConfig = {
        useSchemaRegistry: this.useSchemaProducer,
        useConfluentKafka: this.useConfluentKafkaConfig,
        environment: process.env.CONFLUENT_KAFKA_ENVIRONMENT || "",
    };

    public vpTopic = getConfluentTopic(ConfluentTopics.GTFS_VP, this.envConfig.environment, this.envConfig.useSchemaRegistry ? Versions.V2 : Versions.V1);

    public drTopic = getConfluentTopic(ConfluentTopics.DEAD_RECKONING, this.envConfig.environment, Versions.V1);

    public async onModuleInit() {
        if (this.useConfluentKafkaConfig) {
            await this.keyVault.loadAndWatchSecret(this.confluentSecretName, (secretValue) => {
                if (!secretValue) {
                    return;
                }
                const { key, secret } = JSON.parse(secretValue);
                this.sasl = {
                    mechanism: "plain" as never,
                    username: key,
                    password: secret,
                };
                this.emit("configUpdate");
            });

            if (this.useSchemaProducer) {
                await this.keyVault.loadAndWatchSecret(this.schemaSecretName, (secretValue) => {
                    if (!secretValue) {
                        return;
                    }
                    const { key, secret } = JSON.parse(secretValue);
                    this.schema = {
                        url: process.env.SCHEMA_REGISTRY_URL || "",
                        apiKey: key,
                        apiSecret: secret,
                    };
                    this.emit("configUpdate");
                });
            }
        }
    }

    public get environment(): string {
        if (this.useSchemaProducer) {
            return "Schema Registry";
        }

        return "Confluent";
    }
}
