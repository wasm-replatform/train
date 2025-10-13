import { Injectable } from "@nestjs/common";
import { Test, TestingModule } from "@nestjs/testing";
import { Reflector } from "@nestjs/core";
import { MicroserviceOptions } from "@nestjs/microservices";
import {
    getConfluentTopic,
    Versions,
    KafkaConsumer,
    KafkaConsumerModule,
    KafkaConsumerTransporter,
    KafkaProducer,
    KafkaProducerModule,
    MockKafka,
    ConfluentTopics,
} from "at-realtime-common/kafka";
import { ILoggerConfig, Logger, LoggerModule } from "at-realtime-common/logger";
import { FastifyAdapter } from "at-realtime-common/server";
import { IAppConfig, getGlobalInterceptor, getGlobalValidationPipe } from "at-realtime-common/common";
import { RedisModule } from "at-realtime-common/redis";
import { HttpClientModule } from "at-realtime-common/http-client";
import { KeyVaultModule, KeyVault } from "at-realtime-common/keyvault";
import { AuthTokenRetrieverModule } from "at-realtime-common/auth";

import { AppConfig } from "../../../src/config/app";
import { LoggerConfig } from "../../../src/config/logger";
import { RedisConfig } from "../../../src/config/redis";
import { KafkaProducerConfig } from "../../../src/config/kafka-producer";
import { KafkaConsumerConfig } from "../../../src/config/kafka-consumer";
import { KeyVaultConfig } from "../../../src/config/keyvault-config";
import { AuthTokenRetrieverConfig } from "../../../src/config/auth-token-retriever";

import { KafkaController } from "../../../src/controller/kafka";
import { RestApiController } from "../../../src/controller/rest";
import { CacheRepository } from "../../../src/repositories/cache";

import { KafkaService } from "../../../src/services/kafka-service";
import { GodMode } from "../../../src/processors/god-mode";
import { PassengerCountProcessor } from "../../../src/processors/passenger-count";
import { SerialDataProcessor } from "../../../src/processors/serial-data";
import { LocationProcessor } from "../../../src/processors/location";
import { FleetApiService } from "../../../src/apis/fleet";
import { TripMgtApi } from "../../../src/apis/trip-mgt";
import { BlockMgtApi } from "../../../src/apis/block-mgt";

async function createTestingModule(supportApiPort: number) {
    @Injectable()
    class MockKafkaProducer extends MockKafka {
        constructor() {
            super({}, {}, {});
        }
    }

    @Injectable()
    class MockKafkaConsumer extends MockKafka {
        constructor() {
            super(
                {},
                {},
                {
                    topics: [
                        getConfluentTopic(ConfluentTopics.SMARTRAK_BUS_AVL, process.env.CONFLUENT_KAFKA_ENVIRONMENT || "dev", Versions.V1),
                        getConfluentTopic(ConfluentTopics.AIS, process.env.CONFLUENT_KAFKA_ENVIRONMENT || "dev", Versions.V1),
                        getConfluentTopic(ConfluentTopics.FERRY_TRIPS_EVENT, process.env.CONFLUENT_KAFKA_ENVIRONMENT || "dev", Versions.V1),
                        getConfluentTopic(ConfluentTopics.PASSENGER_COUNT, process.env.CONFLUENT_KAFKA_ENVIRONMENT || "dev", Versions.V1),
                        ...(process.env.USE_CAF_TOPIC === "true"
                            ? [getConfluentTopic(ConfluentTopics.CAF_AVL, process.env.CONFLUENT_KAFKA_ENVIRONMENT || "dev", Versions.V2)]
                            : []),
                    ],
                },
            );
        }
    }

    @Injectable()
    class MockRedisConfig extends RedisConfig {
        public mock = true;
    }

    @Injectable()
    class MockAppConfig extends AppConfig {
        public fleetApiUrl = "http://localhost:" + supportApiPort;
        public tripManagementUrl = "http://localhost:" + supportApiPort;
        public blockManagementUrl = "http://localhost:" + supportApiPort;
    }

    @Injectable()
    class MockKeyVault extends KeyVault {
        constructor() {
            super(new Logger({} as IAppConfig, {} as ILoggerConfig), { kvHost: "string" });
        }
        public async loadAndWatchSecret() {
            return;
        }
        public async onApplicationBootstrap() {
            return;
        }
        public async getSecret(secretName: string) {
            return `test${secretName}`;
        }
    }

    @Injectable()
    class MockAuthTokenRetrieverConfig extends AuthTokenRetrieverConfig {
        public localDevEnv = {
            accessToken: "test-key",
        };
    }

    return Test.createTestingModule({
        imports: [
            LoggerModule.register(AppConfig, LoggerConfig),
            KeyVaultModule.register(Logger, KeyVaultConfig),
            HttpClientModule.register(Logger),
            KafkaProducerModule.register(Logger, AppConfig, KafkaProducerConfig),
            KafkaConsumerModule.register(Logger, AppConfig, KafkaConsumerConfig),
            RedisModule.register(Logger, MockRedisConfig),
            AuthTokenRetrieverModule.register(MockAuthTokenRetrieverConfig),
        ],
        controllers: [KafkaController, RestApiController],
        providers: [
            AppConfig,
            KafkaConsumerConfig,
            KafkaProducerConfig,
            RedisConfig,
            KafkaService,
            CacheRepository,
            GodMode,
            PassengerCountProcessor,
            SerialDataProcessor,
            LocationProcessor,
            FleetApiService,
            TripMgtApi,
            BlockMgtApi,
        ],
    })
        .overrideProvider(AppConfig)
        .useClass(MockAppConfig)
        .overrideProvider(KafkaConsumer)
        .useClass(MockKafkaConsumer)
        .overrideProvider(KafkaProducer)
        .useClass(MockKafkaProducer)
        .overrideProvider(RedisConfig)
        .useClass(MockRedisConfig)
        .overrideProvider(KeyVault)
        .useClass(MockKeyVault)
        .overrideProvider(AuthTokenRetrieverConfig)
        .useClass(MockAuthTokenRetrieverConfig)
        .compile();
}

export async function bootstrapAppForTest(supportApiPort: number): Promise<TestingModule> {
    const module = await createTestingModule(supportApiPort);
    const app = module.createNestApplication(new FastifyAdapter());

    app.useGlobalPipes(getGlobalValidationPipe());
    app.useGlobalInterceptors(getGlobalInterceptor(app.get(Reflector)));
    app.connectMicroservice<MicroserviceOptions>({
        strategy: app.get(KafkaConsumerTransporter),
    });

    await app.init();
    await app.startAllMicroservices();
    await app.getHttpAdapter().getInstance().ready();
    return module;
}
