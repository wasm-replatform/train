describe("Config", () => {

    beforeEach(() => {
        jest.resetModules();
    });

    it("should have correct properties for Kafka and returning confluent variables", () => {
        const { Config } = require("../../src/config");
        const { consumer, producer } = Config.kafka;

        expect(consumer.endpoints).toEqual(["lkc-prx9qk-6meoz4.australiaeast.azure.glb.confluent.cloud:9092"]);
        expect(consumer.topics).toEqual(["dev-realtime-r9k.v1"]);
        expect(consumer.consumerGroup).toEqual("dev-r9k-position-adapter-v2-local-3");

        expect(producer.endpoints).toEqual(["lkc-prx9qk-6meoz4.australiaeast.azure.glb.confluent.cloud:9092"]);
        expect(producer.vpTopic).toEqual("dev-realtime-r9k-to-smartrak.v1");
    });
});
