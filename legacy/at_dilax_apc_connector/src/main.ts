import { EventStore } from "at-connector-common";
import { KafkaProducer } from "at-realtime-common/kafka";
import { FastifyAdapter, ServerFactory } from "at-realtime-common/server";
import * as nr from "newrelic";
import "reflect-metadata";
import Config from "./config";
import { KafkaProducerConfig } from "./kafka-producer-config";

export class Main {
    private publisher: KafkaProducer;
    private webApp: FastifyAdapter;
    private kafkaProducerConfig: KafkaProducerConfig;

    /**
     * Starts the web service.
     */
    public async start(): Promise<void> {
        Config.logger.info("Starting DILAX APC connector");

        this.kafkaProducerConfig = new KafkaProducerConfig();
        this.publisher = new KafkaProducer(
            Config.logger,
            {
                newRelicPrefix: Config.newRelicPrefix,
            },
            this.kafkaProducerConfig,
        );

        this.webApp = await ServerFactory.createSimple(Config.logger);
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        this.webApp.post("/api/apc", async (req: any, res: any) => {
            nr.incrementMetric(Config.newRelicPrefix + "/message_count", 1);
            try {
                const reqString = JSON.stringify(req.body);
                Config.logger.debug(`Received from DILAX APC: ${reqString}`);
                const eventStore = new EventStore(Config.logger, <string>Config.replication.endpoint, "Dilax");
                // Store the original message for replication
                eventStore.put(reqString);
                await this.publisher.publish(this.kafkaProducerConfig.topic, { value: JSON.stringify(req.body), key: req.body.device ? req.body.device.site : undefined }).then(
                    () => {
                        res.status(200).send("OK");
                    },
                    (err) => {
                        Config.logger.error(`Error publishing to Kafka:\n${err}`);
                        res.status(500).send(err);
                    },
                );
            } catch (err) {
                nr.noticeError(err);
                Config.logger.error(`Error processing POST request:\n${err}`);
            }
        });

        this.publisher.start();
        this.webApp.configureStatusCheck([this.publisher]);
        this.webApp.listen(Config.port);
    }

    /**
     * Stop the web service
     */
    public async close(): Promise<void> {
        Config.logger.info("Stopping DILAX APC connector");
        await this.publisher.stop();
        await this.webApp.close();
    }
}
