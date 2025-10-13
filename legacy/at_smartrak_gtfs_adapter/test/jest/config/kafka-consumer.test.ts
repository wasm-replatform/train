import { ConfluentTopics, Versions, getConfluentTopic } from "at-realtime-common/kafka";
import { KafkaConsumerConfig } from "../../../src/config/kafka-consumer";
import { KeyVault } from "at-realtime-common/keyvault";
import { ILogger } from "at-realtime-common/logger";
describe("kafka consumer config test", () => {
    const OLD_ENV = process.env;
    beforeEach(() => {
        process.env = { ...OLD_ENV };
    });

    it("Should use confluent kafka config without CAF topic", async () => {
        const hosts = ["lkc-prx9qk-6meoz4.australiaeast.azure.glb.confluent.cloud:9092"];
        const consumerGroup = "dev-smartrak-gtfs-adapter";
        process.env = {
            CONFLUENT_KAFKA_ENVIRONMENT: "dev",
            KAFKA_CONSUMER_GROUP: "smartrak-gtfs-adapter",
            KAFKA_HOSTS: hosts.join(","),
            SCHEMA_REGISTRY_URL: "https://some.url",
            USE_CAF_TOPIC: "false",
        };
        const secret = { key: "test key", secret: "test secret" };
        const keyVaultConfig = {
            kvHost: "https://test.vault.azure.net",
            secretWatcherInterval: 100,
            localDevEnv: {
                "realtime-confluent-key": {
                    value: JSON.stringify(secret),
                },
                "realtime-schema-registry-key": {
                    value: JSON.stringify(secret),
                },
            },
        };
        const keyVault = new KeyVault({} as ILogger, keyVaultConfig);
        const kafkaConsumerConfig = new KafkaConsumerConfig(keyVault);
        await kafkaConsumerConfig.onModuleInit();

        expect(kafkaConsumerConfig.brokers).toEqual(hosts);
        expect(kafkaConsumerConfig.ssl).toEqual(true);
        expect(kafkaConsumerConfig.consumerGroup).toEqual(consumerGroup);
        expect(kafkaConsumerConfig.sasl).toBeDefined();
        expect(kafkaConsumerConfig.sasl?.mechanism).toEqual("plain");
        expect(kafkaConsumerConfig.sasl?.username).toBe(secret.key);
        expect(kafkaConsumerConfig.sasl?.password).toBe(secret.secret);
        expect(kafkaConsumerConfig.schema?.url).toBe("https://some.url");
        expect(kafkaConsumerConfig.schema?.apiKey).toBe(secret.key);
        expect(kafkaConsumerConfig.schema?.apiSecret).toBe(secret.secret);
        const confluentTopic = getConfluentTopic(ConfluentTopics.SMARTRAK_BUS_AVL, "dev", Versions.V1);
        const schemaTopic = getConfluentTopic(ConfluentTopics.SMARTRAK_BUS_AVL, "dev", Versions.V1);
        const cafTopic = getConfluentTopic(ConfluentTopics.CAF_AVL, "dev", Versions.V2);
        expect(kafkaConsumerConfig.topics).toContain(confluentTopic);
        expect(kafkaConsumerConfig.topics).toContain(schemaTopic);
        expect(kafkaConsumerConfig.topics).not.toContain(cafTopic);
    });

    it("Should use confluent kafka config with CAF topic", async () => {
        const hosts = ["lkc-prx9qk-6meoz4.australiaeast.azure.glb.confluent.cloud:9092"];
        const consumerGroup = "dev-smartrak-gtfs-adapter";
        process.env = {
            CONFLUENT_KAFKA_ENVIRONMENT: "dev",
            KAFKA_CONSUMER_GROUP: "smartrak-gtfs-adapter",
            KAFKA_HOSTS: hosts.join(","),
            SCHEMA_REGISTRY_URL: "https://some.url",
            USE_CAF_TOPIC: "true",
        };
        const secret = { key: "test key", secret: "test secret" };
        const keyVaultConfig = {
            kvHost: "https://test.vault.azure.net",
            secretWatcherInterval: 100,
            localDevEnv: {
                "realtime-confluent-key": {
                    value: JSON.stringify(secret),
                },
                "realtime-schema-registry-key": {
                    value: JSON.stringify(secret),
                },
            },
        };
        const keyVault = new KeyVault({} as ILogger, keyVaultConfig);
        const kafkaConsumerConfig = new KafkaConsumerConfig(keyVault);
        await kafkaConsumerConfig.onModuleInit();

        expect(kafkaConsumerConfig.brokers).toEqual(hosts);
        expect(kafkaConsumerConfig.ssl).toEqual(true);
        expect(kafkaConsumerConfig.consumerGroup).toEqual(consumerGroup);
        expect(kafkaConsumerConfig.sasl).toBeDefined();
        expect(kafkaConsumerConfig.sasl?.mechanism).toEqual("plain");
        expect(kafkaConsumerConfig.sasl?.username).toBe(secret.key);
        expect(kafkaConsumerConfig.sasl?.password).toBe(secret.secret);
        expect(kafkaConsumerConfig.schema?.url).toBe("https://some.url");
        expect(kafkaConsumerConfig.schema?.apiKey).toBe(secret.key);
        expect(kafkaConsumerConfig.schema?.apiSecret).toBe(secret.secret);
        const confluentTopic = getConfluentTopic(ConfluentTopics.SMARTRAK_BUS_AVL, "dev", Versions.V1);
        const schemaTopic = getConfluentTopic(ConfluentTopics.SMARTRAK_BUS_AVL, "dev", Versions.V1);
        const cafTopic = getConfluentTopic(ConfluentTopics.CAF_AVL, "dev", Versions.V2);
        expect(kafkaConsumerConfig.topics).toContain(confluentTopic);
        expect(kafkaConsumerConfig.topics).toContain(schemaTopic);
        expect(kafkaConsumerConfig.topics).toContain(cafTopic);
    });
});
