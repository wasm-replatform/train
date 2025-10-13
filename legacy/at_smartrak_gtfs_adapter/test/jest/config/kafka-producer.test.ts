import { ConfluentTopics, Versions, getConfluentTopic } from "at-realtime-common/kafka";
import { KeyVault } from "at-realtime-common/keyvault";
import { KafkaProducerConfig } from "../../../src/config/kafka-producer";
describe("kafka producer config test", () => {
    it("Should use confluent kafka config", () => {
        const hosts = ["lkc-prx9qk-6meoz4.australiaeast.azure.glb.confluent.cloud:9092"];
        process.env = {
            CONFLUENT_KAFKA_ENVIRONMENT: "dev",
            KAFKA_HOSTS: hosts.join(","),
        };
        const kafkaProducerConfig = new KafkaProducerConfig({} as any);
        expect(kafkaProducerConfig.brokers).toEqual(hosts);
        expect(kafkaProducerConfig.ssl).toEqual(true);
        expect(kafkaProducerConfig.sasl === undefined).toBe(false);
        const vsTopic = getConfluentTopic(ConfluentTopics.GTFS_VP, process.env.CONFLUENT_KAFKA_ENVIRONMENT || "dev", Versions.V1);
        const drTopic = getConfluentTopic(ConfluentTopics.DEAD_RECKONING, process.env.CONFLUENT_KAFKA_ENVIRONMENT || "dev", Versions.V1);
        expect(kafkaProducerConfig.vpTopic).toBe(vsTopic);
        expect(kafkaProducerConfig.drTopic).toBe(drTopic);
    });

    it("When module init, should load secret from key vault", async () => {
        process.env = {
            CONFLUENT_KAFKA_ENVIRONMENT: "dev",
        };
        const secret = { key: "test key", secret: "test secret" };
        const keyVaultConfig = {
            kvHost: "https://test.vault.azure.net",
            secretWatcherInterval: 100,
            localDevEnv: {
                "realtime-confluent-key": {
                    value: JSON.stringify(secret),
                },
            },
        };
        const keyVault = new KeyVault({} as any, keyVaultConfig);
        const kafkaProducerConfig = new KafkaProducerConfig(keyVault);
        kafkaProducerConfig.onModuleInit();
        await new Promise(process.nextTick);
        expect(kafkaProducerConfig.sasl?.mechanism).toBe("plain");
        expect(kafkaProducerConfig.sasl?.username).toBe(secret.key);
        expect(kafkaProducerConfig.sasl?.password).toBe(secret.secret);
        expect(kafkaProducerConfig.environment).toEqual("Confluent");
    });

    it("When module init and useSchemaProducer is true, should load both secrets from key vault", async () => {
        process.env = {
            USE_SCHEMA_REGISTRY_PRODUCER: "true",
            SCHEMA_REGISTRY_URL: "https://some.url",
            CONFLUENT_KAFKA_ENVIRONMENT: "dev",
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
        const keyVault = new KeyVault({} as any, keyVaultConfig);
        const kafkaProducerConfig = new KafkaProducerConfig(keyVault);
        kafkaProducerConfig.onModuleInit();
        await new Promise(process.nextTick);
        expect(kafkaProducerConfig.sasl?.mechanism).toBe("plain");
        expect(kafkaProducerConfig.sasl?.username).toBe(secret.key);
        expect(kafkaProducerConfig.sasl?.password).toBe(secret.secret);
        expect(kafkaProducerConfig.schema?.url).toBe("https://some.url");
        expect(kafkaProducerConfig.schema?.apiKey).toBe(secret.key);
        expect(kafkaProducerConfig.schema?.apiSecret).toBe(secret.secret);
        expect(kafkaProducerConfig.vpTopic).toEqual("dev-realtime-gtfs-vp.v2");
        expect(kafkaProducerConfig.drTopic).toEqual("dev-realtime-dead-reckoning.v1");
        expect(kafkaProducerConfig.environment).toEqual("Schema Registry");
    });
});
