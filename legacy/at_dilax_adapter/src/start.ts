require("newrelic");

import { AuthTokenRetriever } from "at-realtime-common/auth";
import { HttpClient } from "at-realtime-common/http-client";
import { KafkaConsumer, KafkaProducer } from "at-realtime-common/kafka";
import { IRedisConfig, Redis } from "at-realtime-common/redis";
import { FastifyAdapter, ServerFactory } from "at-realtime-common/server";
import * as newrelic from "newrelic";
import * as NodeCache from "node-cache";
import { Config } from "./config";
import { KafkaConsumerConfig } from "./config/kafka-consumer";
import { KafkaProducerConfig } from "./config/kafka-producer";
import Main from "./main";
import { ConfluentSecretRetriever } from "./secret-retriever/confluent-secret-retriever";
import BlockMgtClientAPI from "./services/block-mgt-client-api";
import CcStaticApi from "./services/cc-static-api";
import FleetApi from "./services/fleet-api";
import GtfsStaticApi from "./services/gtfs-static-api";

const options: IRedisConfig = {
    host: Config.redis.host,
    port: Number(Config.redis.port),
    password: Config.redis.password,
    db: Config.redis.db,
    newRelicPrefix: Config.newRelicPrefix,
};
const redisClient = new Redis(Config.logger, options);
const httpClient = new HttpClient(Config.logger);
let main: Main;
let webApp: FastifyAdapter;

const run = async () => {
    if (Config.useConfluentKafkaConfig) {
        await ConfluentSecretRetriever.retrieveAndWatch(closeApp);
    }
    const tokenRetriever = new AuthTokenRetriever(Config.getAzureAccessTokenRetrieverConfig());
    await redisClient.start();
    const blockMgtClientAPI = new BlockMgtClientAPI(tokenRetriever, httpClient);

    const producerConfig = new KafkaProducerConfig();

    const kafkaProducer = new KafkaProducer(Config.logger, Config, producerConfig);
    const kafkaConsumer = new KafkaConsumer(Config.logger, Config, new KafkaConsumerConfig());

    main = new Main(
        kafkaConsumer,
        kafkaProducer,
        producerConfig.topic,
        new FleetApi(redisClient, httpClient),
        new CcStaticApi(httpClient),
        new GtfsStaticApi(new NodeCache(), httpClient),
        blockMgtClientAPI,
        redisClient,
    );

    webApp = await ServerFactory.createSimple(Config.logger);
    webApp.configureStatusCheck([redisClient, kafkaConsumer, kafkaProducer]);

    await main
        .start()
        .then(() => Config.logger.info("Dilax adapter started"))
        .catch((reason) => {
            newrelic.noticeError(reason);
            Config.logger.error(`Failed to start Dilax adapter: ${reason}`);
        });
    webApp.listen(Config.port);
};

const closeApp = () => {
    Config.logger.info("Restarting application to reload Kafka keys");
    main?.stop(); // do not await stop as it could get stuck
    return setTimeout(() => process.exit(1), 10 * 1000);
};

run();
