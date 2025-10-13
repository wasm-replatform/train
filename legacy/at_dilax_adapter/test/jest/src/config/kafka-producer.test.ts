import { AzureTopics, ConfluentTopics, KafkaEnvConfig, getTopicByKafkaEnv } from "at-realtime-common/kafka";
import { KafkaProducerConfig } from "../../../../src/config/kafka-producer";
describe("kafka producer config test", () => {
    it("Should use confluent kafka config", () => {
        const hosts = ["lkc-prx9qk-6meoz4.australiaeast.azure.glb.confluent.cloud:9092"];
        process.env = {
            IS_LOCAL: "false",
            KAFKA_ENVIRONMENT: "dev",
            KAFKA_HOSTS: "lkc-prx9qk-6meoz4.australiaeast.azure.glb.confluent.cloud:9092",
            USE_SCHEMA_REGISTRY_PRODUCER: "true",
        };
        const envConfig: KafkaEnvConfig = {
            useSchemaRegistry: process.env.USE_SCHEMA_REGISTRY_PRODUCER === "true",
            useConfluentKafka: process.env.IS_LOCAL !== "true",
            environment: process.env.KAFKA_ENVIRONMENT || "",
        };

        const kafkaProducerConfig = new KafkaProducerConfig();
        expect(kafkaProducerConfig.brokers).toEqual(hosts);
        expect(kafkaProducerConfig.ssl).toEqual(true);
        expect(kafkaProducerConfig.sasl).toBeDefined();
        const dilaxEnrichedTopic = getTopicByKafkaEnv(ConfluentTopics.DILAX_APC_ENRICHED, AzureTopics.DILAX_APC_ENRICHED_V2, envConfig);
        expect(kafkaProducerConfig.topic).toBe(dilaxEnrichedTopic);
    });

    it("Should use confluent kafka with local config", () => {
        const hosts = ["lkc-prx9qk-6meoz4.australiaeast.azure.glb.confluent.cloud:9092"];
        process.env = {
            IS_LOCAL: "true",
            KAFKA_ENVIRONMENT: "dev",
            KAFKA_HOSTS: "lkc-prx9qk-6meoz4.australiaeast.azure.glb.confluent.cloud:9092",
            USE_SCHEMA_REGISTRY_PRODUCER: "true",
        };
        const envConfig: KafkaEnvConfig = {
            useSchemaRegistry: process.env.USE_SCHEMA_REGISTRY_PRODUCER === "true",
            useConfluentKafka: process.env.IS_LOCAL !== "true",
            environment: process.env.KAFKA_ENVIRONMENT || "",
        };
        const kafkaProducerConfig = new KafkaProducerConfig();
        expect(kafkaProducerConfig.brokers).toEqual(hosts);
        expect(kafkaProducerConfig.ssl).toEqual(false);
        expect(kafkaProducerConfig.sasl).toBeUndefined();
        const dilaxEnrichedTopic = getTopicByKafkaEnv(ConfluentTopics.DILAX_APC_ENRICHED, AzureTopics.DILAX_APC_ENRICHED_V2, envConfig);
        expect(kafkaProducerConfig.topic).toBe(dilaxEnrichedTopic);
    });

    it("When module init and useSchemaProducer is true, schema should not be empty", async () => {
        process.env = { USE_SCHEMA_REGISTRY_PRODUCER: "true", SCHEMA_REGISTRY_URL: "https://some.url", KAFKA_ENVIRONMENT: "dev" };
        const kafkaProducerConfig = new KafkaProducerConfig();
        await new Promise(process.nextTick);
        expect(kafkaProducerConfig.sasl).toBeDefined();
        expect(kafkaProducerConfig.schema).toBeDefined();
        expect(kafkaProducerConfig.schema?.url).toBe("https://some.url");
        expect(kafkaProducerConfig.topic).toEqual("dev-realtime-dilax-apc-enriched.v2");
    });
});
