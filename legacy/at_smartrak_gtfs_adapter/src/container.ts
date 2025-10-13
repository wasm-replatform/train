import { Module } from "@nestjs/common";

import { AuthTokenRetrieverModule } from "at-realtime-common/auth";
import { HttpClientModule } from "at-realtime-common/http-client";
import { KafkaConsumerModule, KafkaProducerModule } from "at-realtime-common/kafka";
import { KeyVaultModule } from "at-realtime-common/keyvault";
import { Logger, LoggerModule } from "at-realtime-common/logger";
import { RedisModule } from "at-realtime-common/redis";

import { AppConfig } from "./config/app";
import { AuthTokenRetrieverConfig } from "./config/auth-token-retriever";
import { KafkaConsumerConfig } from "./config/kafka-consumer";
import { KafkaProducerConfig } from "./config/kafka-producer";
import { KeyVaultConfig } from "./config/keyvault-config";
import { LoggerConfig } from "./config/logger";
import { RedisConfig } from "./config/redis";

import { KafkaController } from "./controller/kafka";
import { RestApiController } from "./controller/rest";
import { CacheRepository } from "./repositories/cache";

import { BlockMgtApi } from "./apis/block-mgt";
import { FleetApiService } from "./apis/fleet";
import { TripMgtApi } from "./apis/trip-mgt";
import { GodMode } from "./processors/god-mode";
import { LocationProcessor } from "./processors/location";
import { PassengerCountProcessor } from "./processors/passenger-count";
import { SerialDataProcessor } from "./processors/serial-data";
import { KafkaService } from "./services/kafka-service";

@Module({
    imports: [
        LoggerModule.register(AppConfig, LoggerConfig),
        KeyVaultModule.register(Logger, KeyVaultConfig),
        HttpClientModule.register(Logger),
        KafkaProducerModule.register(Logger, AppConfig, KafkaProducerConfig),
        KafkaConsumerModule.register(Logger, AppConfig, KafkaConsumerConfig),
        RedisModule.register(Logger, RedisConfig),
        AuthTokenRetrieverModule.register(AuthTokenRetrieverConfig),
    ],
    controllers: [KafkaController, RestApiController],
    providers: [
        AppConfig,
        BlockMgtApi,
        CacheRepository,
        FleetApiService,
        GodMode,
        KafkaConsumerConfig,
        KafkaProducerConfig,
        KafkaService,
        LocationProcessor,
        PassengerCountProcessor,
        RedisConfig,
        SerialDataProcessor,
        TripMgtApi,
    ],
})
export class Container {}
