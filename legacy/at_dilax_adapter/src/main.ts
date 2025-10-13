import { KafkaConsumer, KafkaMessage, KafkaProducer, Topics } from "at-realtime-common/kafka";
import { Redis } from "at-realtime-common/redis";
import DilaxProcessor from "./dilax-processor";

import * as nr from "newrelic";
import * as cron from "node-cron";
import { Config } from "./config";
import { DilaxLostConnectionsDetector } from "./dilax-lost-connections-detector";
import BlockMgtClientAPI from "./services/block-mgt-client-api";
import CcStaticApi from "./services/cc-static-api";
import FleetApi from "./services/fleet-api";
import GtfsStaticApi from "./services/gtfs-static-api";

/**
 * Main class - converting Dilax JSON to enriched Dilax Json.
 */
export default class Main {
    private dilaxProcessor: DilaxProcessor;
    private dilaxConnectionLostDetector: DilaxLostConnectionsDetector;
    private gtfsApi: GtfsStaticApi;
    private isKafkaStarted = false;

    constructor(
        private consumer: KafkaConsumer,
        private producer: KafkaProducer,
        private producerTopic: Topics,
        fleetApi: FleetApi,
        ccStaticApi: CcStaticApi,
        gtfsApi: GtfsStaticApi,
        blockMgtClientApi: BlockMgtClientAPI,
        redisClient: Redis,
    ) {
        this.gtfsApi = gtfsApi;
        this.dilaxConnectionLostDetector = new DilaxLostConnectionsDetector(redisClient, blockMgtClientApi);
        this.dilaxProcessor = new DilaxProcessor(redisClient, fleetApi, ccStaticApi, gtfsApi, blockMgtClientApi);
    }

    public async onPause(): Promise<void> {
        await this.pause();
    }

    public async onResume(): Promise<void> {
        await this.resume();
    }

    private async startStopTypeCaching(gtfsApi: GtfsStaticApi): Promise<void> {
        await gtfsApi.getTrainStopTypes();

        const minutestPastMidnight = Math.floor(Math.random() * 15) + 1;
        cron.schedule(`${minutestPastMidnight} 0 * * *`, () => gtfsApi.getTrainStopTypes(), { timezone: "Pacific/Auckland" });
    }

    private handleMessage = (message: KafkaMessage, next: () => void) => {
        Config.logger.debug(`received kafka message [${message.value}]`);
        nr.startBackgroundTransaction(`${Config.newRelicPrefix}/on_message`, async () => {
            this.process(message).catch((err) => {
                nr.noticeError(err);
                Config.logger.error(err);
            });
        });
        next();
    };

    public async start(): Promise<void> {
        Config.logger.info("Starting the Dilax adapter...");

        await this.startStopTypeCaching(this.gtfsApi);
        await this.dilaxConnectionLostDetector.init();

        this.consumer.onMessage(this.handleMessage);

        if (!Config.isSlot) {
            await this.resume();
        }
    }

    public async pause(): Promise<void> {
        this.dilaxConnectionLostDetector.stopDetectingLostConnections();
        if (this.isKafkaStarted) {
            await this.producer.stop();
            await this.consumer.stop();
            this.isKafkaStarted = false;
        } else {
            Config.logger.warn("kafka has already stopped");
        }
    }

    public async resume(): Promise<void> {
        await this.dilaxConnectionLostDetector.startDetectingLostConnections();
        if (!this.isKafkaStarted) {
            await this.producer.start();
            await this.consumer.start();
            this.isKafkaStarted = true;
        } else {
            Config.logger.warn("kafka has already started");
        }
    }

    public async stop(): Promise<void> {
        Config.logger.info("Stopping the Dilax adapter...");
        await this.pause();
    }

    private async process(message: KafkaMessage) {
        const start = Date.now();
        const event = JSON.parse(message.value as string);

        const dilaxEventEnriched = await this.dilaxProcessor.process(event);
        try {
            await this.producer.publish(this.producerTopic, { value: JSON.stringify(dilaxEventEnriched), key: dilaxEventEnriched.trip_id });
        } catch (err) {
            nr.noticeError(err);
            Config.logger.error(`Error publishing to Kafka: ${err.message}`);
        }
        nr.recordMetric(`${Config.newRelicPrefix}/processing_time`, Date.now() - start);
    }
}
