import { ISecretLoader } from "at-realtime-common/common";
import { ConfluentTopics, IKafkaProducerConfig, SchemaConfig, Versions, getConfluentTopic } from "at-realtime-common/kafka";
import { EventEmitter } from "events";
import { SASLOptions } from "kafkajs";
import { ConfluentSecretRetriever } from "./confluent-secret-retriever";

export class KafkaProducerConfig extends EventEmitter implements IKafkaProducerConfig, ISecretLoader {
    private useSchemaRegistryProducer = process.env.USE_SCHEMA_REGISTRY_PRODUCER === "true";
    private environment = process.env.CONFLUENT_KAFKA_ENVIRONMENT || "";
    public useConfluentKafkaConfig = process.env.IS_LOCAL !== "true";

    public brokers = (process.env.KAFKA_HOSTS || "").split(",");

    public ssl = this.useConfluentKafkaConfig;

    public sasl = this.useConfluentKafkaConfig
        ? ({
              mechanism: "plain",
              ...ConfluentSecretRetriever.confluentKafkaSecret,
          } as SASLOptions)
        : undefined;

    public schema = this.useSchemaRegistryProducer
        ? ({
              url: process.env.SCHEMA_REGISTRY_URL || "https://psrc-mp377.australiaeast.azure.confluent.cloud",
              ...ConfluentSecretRetriever.schemaRegistrySecret,
          } as SchemaConfig)
        : undefined;

    public topic = getConfluentTopic(ConfluentTopics.DILAX_APC, this.environment, this.useSchemaRegistryProducer ? Versions.V2 : Versions.V1);
}
