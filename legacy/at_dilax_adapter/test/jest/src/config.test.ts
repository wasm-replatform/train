import { Config } from "../../../src/config";
import { KeyVault } from "../../../src/secret-retriever/key-vault";

describe("Tests getKafkaConfig in Config", () => {
    const KAFKA_HOSTS = "lkc-prx9qk-6meoz4.australiaeast.azure.glb.confluent.cloud:9092";
    const KAFKA_PRODUCER_TOPIC = "dev-realtime-dilax-adapter-apc-enriched.v1";
    const KAFKA_CONSUMER_TOPIC = "dev-realtime-dilax-adapter-apc.v1";
    const KAFKA_CONSUMER_GROUP = "dev-at-dilax-adapter-adapter";
    const KAFKA_SSL = true;
    const mockSecret = { key: "my-user", secret: "my-secret" };

    it("Check static values", () => {
        const azureAccessConfig = Config.getAzureAccessTokenRetrieverConfig();

        // since these are static params, the env vars will be empty when they are calculated
        // default values were also removed, so they should be empty where applicable
        expect(azureAccessConfig.clientId).toEqual("");
        expect(azureAccessConfig.keyVault.host).toEqual("https://.vault.azure.net");
        expect(azureAccessConfig.keyVault.secretNameSystemClientSecret).toEqual("");
    });

    test.each([
        {
            useConfluentKafkaConfig: true,
            kafkaHosts: KAFKA_HOSTS,
            kafkaProducerTopic: KAFKA_PRODUCER_TOPIC,
            kafkaConsumerTopic: KAFKA_CONSUMER_TOPIC,
            kafkaConsumerGroup: KAFKA_CONSUMER_GROUP,
            ssl: KAFKA_SSL,
            sasl: {
                mechanism: "plain",
                username: mockSecret.key,
                password: mockSecret.secret,
            },
        },
        {
            useConfluentKafkaConfig: false,
            kafkaHosts: KAFKA_HOSTS,
            kafkaProducerTopic: KAFKA_PRODUCER_TOPIC,
            kafkaConsumerTopic: KAFKA_CONSUMER_TOPIC,
            kafkaConsumerGroup: KAFKA_CONSUMER_GROUP,
            ssl: false,
            sasl: undefined,
        },
    ])(
        "Check correct configuration for Confluent kafka with local and confluent configs",
        async ({ useConfluentKafkaConfig, kafkaHosts, kafkaProducerTopic, kafkaConsumerTopic, kafkaConsumerGroup, ssl, sasl }) => {
            const mockGetSecret = jest.spyOn(KeyVault.prototype, "getSecret").mockReturnValue(Promise.resolve(JSON.stringify(mockSecret)));
        },
    );
});
