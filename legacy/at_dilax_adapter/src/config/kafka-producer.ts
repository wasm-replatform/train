import { ISecretLoader } from "at-realtime-common/common";
import { AzureTopics, ConfluentTopics, IKafkaProducerConfig, KafkaEnvConfig, SchemaConfig, getTopicByKafkaEnv } from "at-realtime-common/kafka";
import { EventEmitter } from "events";
import { SASLOptions } from "kafkajs";
import { ConfluentSecretRetriever } from "../secret-retriever/confluent-secret-retriever";

export class KafkaProducerConfig extends EventEmitter implements IKafkaProducerConfig, ISecretLoader {
    private useSchemaProducer = process.env.USE_SCHEMA_REGISTRY_PRODUCER === "true";
    private useConfluentKafkaConfig = process.env.IS_LOCAL !== "true";

    public brokers = (process.env.KAFKA_HOSTS || "lkc-prx9qk-6meoz4.australiaeast.azure.glb.confluent.cloud:9092").split(",");

    public ssl = this.useConfluentKafkaConfig;

    public sasl = this.useConfluentKafkaConfig
        ? ({
              mechanism: "plain",
              ...ConfluentSecretRetriever.confluentKafkaSecret,
          } as SASLOptions)
        : undefined;

    public schema = this.useSchemaProducer
        ? ({
              url: process.env.SCHEMA_REGISTRY_URL || "https://psrc-mp377.australiaeast.azure.confluent.cloud",
              ...ConfluentSecretRetriever.schemaRegistrySecret,
          } as SchemaConfig)
        : undefined;

    private envConfig: KafkaEnvConfig = {
        useSchemaRegistry: this.useSchemaProducer,
        useConfluentKafka: this.useConfluentKafkaConfig,
        environment: process.env.KAFKA_ENVIRONMENT || "dev",
    };
    public topic = getTopicByKafkaEnv(ConfluentTopics.DILAX_APC_ENRICHED, AzureTopics.DILAX_APC_ENRICHED_V2, this.envConfig);
}
