import { Injectable } from "@nestjs/common";
import { ISecretLoader } from "at-realtime-common/common";
import { ConfluentTopics, IKafkaConsumerConfig, SchemaConfig, Versions, getConfluentConsumerGroup, getConfluentTopic } from "at-realtime-common/kafka";
import { KeyVault } from "at-realtime-common/keyvault";
import { EventEmitter } from "events";

@Injectable()
export class KafkaConsumerConfig extends EventEmitter implements IKafkaConsumerConfig, ISecretLoader {
    constructor(private keyVault: KeyVault) {
        super();
    }
    private useConfluentKafkaConfig = process.env.IS_LOCAL !== "true";
    private confluentEnv = process.env.CONFLUENT_KAFKA_ENVIRONMENT || "";
    private confluentSecretName = process.env.KEY_VAULT_SECRET_NAME_CONFLUENT_KEY || "realtime-confluent-key";
    private schemaSecretName = process.env.KEY_VAULT_SECRET_NAME_SCHEMA_REGISTRY_KEY || "realtime-schema-registry-key";
    private useCafTopic = process.env.USE_CAF_TOPIC === "true";

    public brokers = (process.env.KAFKA_HOSTS || "").split(",");
    public consumerGroup = getConfluentConsumerGroup(this.confluentEnv, process.env.KAFKA_CONSUMER_GROUP || "");
    public topics = [
        getConfluentTopic(ConfluentTopics.SMARTRAK_BUS_AVL, this.confluentEnv, Versions.V1),
        getConfluentTopic(ConfluentTopics.SMARTRAK_BUS_AVL, this.confluentEnv, Versions.V2),
        getConfluentTopic(ConfluentTopics.SMARTRAK_TRAIN_AVL, this.confluentEnv, Versions.V1),
        getConfluentTopic(ConfluentTopics.R9K_TO_SMARTRAK, this.confluentEnv, Versions.V1),
        getConfluentTopic(ConfluentTopics.PASSENGER_COUNT, this.confluentEnv, Versions.V1),
        ...(this.useCafTopic ? [getConfluentTopic(ConfluentTopics.CAF_AVL, this.confluentEnv, Versions.V2)] : []),
    ];

    public ssl = this.useConfluentKafkaConfig;
    public sasl = this.useConfluentKafkaConfig
        ? {
              mechanism: "plain" as never,
              // loading values from the key vault
              username: "",
              password: "",
          }
        : undefined;
    public schema = this.useConfluentKafkaConfig
        ? ({
              url: process.env.SCHEMA_REGISTRY_URL || "",
              apiKey: "",
              apiSecret: "",
          } as SchemaConfig)
        : undefined;

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
