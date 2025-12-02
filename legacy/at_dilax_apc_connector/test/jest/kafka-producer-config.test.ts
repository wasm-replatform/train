import { ConfluentTopics, Versions, getConfluentTopic } from "at-realtime-common/kafka";
import { KafkaProducerConfig } from "../../src/kafka-producer-config";

describe("kafka producer config test", () => {
    const originalEnv = process.env;

    afterEach(() => {
        process.env = originalEnv;
    });

    it("Should use kafka config", () => {
        const hosts = ["lkc-prx9qk-6meoz4.australiaeast.azure.glb.confluent.cloud:9092"];
        process.env = {
            KAFKA_HOSTS: "lkc-prx9qk-6meoz4.australiaeast.azure.glb.confluent.cloud:9092",
            CONFLUENT_KAFKA_ENVIRONMENT: "test",
        };
        const kafkaProducerConfig = new KafkaProducerConfig({} as any);
        expect(kafkaProducerConfig.brokers).toEqual(hosts);
        expect(kafkaProducerConfig.ssl).toEqual(true);
        expect(kafkaProducerConfig.sasl === undefined).toBe(false);
        const topic = getConfluentTopic(ConfluentTopics.DILAX_APC, "test", Versions.V1);
        expect(kafkaProducerConfig.topic).toBe(topic);
    });
});
