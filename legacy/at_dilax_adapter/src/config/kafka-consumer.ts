import { ISecretLoader } from "at-realtime-common/common";
import { ConfluentTopics, IKafkaConsumerConfig, SchemaConfig, Versions, getConfluentConsumerGroup, getConfluentTopic } from "at-realtime-common/kafka";
import { EventEmitter } from "events";
import { SASLOptions } from "kafkajs";
import { ConfluentSecretRetriever } from "../secret-retriever/confluent-secret-retriever";

export class KafkaConsumerConfig extends EventEmitter implements IKafkaConsumerConfig, ISecretLoader {
    constructor() {
        super();
    }
    private kafkaEnv = process.env.KAFKA_ENVIRONMENT || "dev";
    private useConfluentKafkaConfig = process.env.IS_LOCAL !== "true";

    public brokers = (process.env.KAFKA_HOSTS || "").split(",");
    public consumerGroup = getConfluentConsumerGroup(this.kafkaEnv, process.env.KAFKA_CONSUMER_GROUP || "");
    public topics = [getConfluentTopic(ConfluentTopics.DILAX_APC, this.kafkaEnv, Versions.V1), getConfluentTopic(ConfluentTopics.DILAX_APC, this.kafkaEnv, Versions.V2)];

    public ssl = this.useConfluentKafkaConfig;
    public sasl = this.useConfluentKafkaConfig
        ? ({
              mechanism: "plain",
              ...ConfluentSecretRetriever.confluentKafkaSecret,
          } as SASLOptions)
        : undefined;

    public schema = {
        url: process.env.SCHEMA_REGISTRY_URL || "https://psrc-mp377.australiaeast.azure.confluent.cloud",
        ...ConfluentSecretRetriever.schemaRegistrySecret,
    } as SchemaConfig;
}
