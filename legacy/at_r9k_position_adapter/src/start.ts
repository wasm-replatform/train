require("newrelic");

import * as nr from "newrelic";

import { Main } from "./main";
import { Config } from "./config";
import { ExpressCommon, KafkaCommon } from "at-realtime-common";
import { ConfluentSecretRetriever } from "./confluent-secret-retriever";

let main: Main;

async function run() {
    if (Config.useConfluentKafkaConfig){
        await ConfluentSecretRetriever.retrieve();
    }

    const webApp = new ExpressCommon.ExpressWebAppWrapper(Config.logger, nr);
    await webApp.start(Config.port);
    Config.logger.info("Status endpoint is running");

    const kafkaConsumer = new KafkaCommon.KafkaConsumer(
        Config.logger,
        Config.kafka.consumer.endpoints,
        Config.kafka.consumer.topics,
        Config.kafka.consumer.consumerGroup,
        {
            newRelicClient: nr,
            newRelicPrefix: Config.newRelicPrefix,
            ssl: Config.useConfluentKafkaConfig,
            sasl: Config.useConfluentKafkaConfig ? {
                mechanism: "plain",
                ...ConfluentSecretRetriever.confluentKafkaSecret,
            }: undefined,
        }
    );

    const kafkaProducer = new KafkaCommon.KafkaProducer(
        Config.logger,
        Config.kafka.producer.endpoints,
        Config.kafka.producer.vpTopic,
        {
            newRelicClient: nr,
            newRelicPrefix: Config.newRelicPrefix,
            ssl: Config.useConfluentKafkaConfig,
            sasl: Config.useConfluentKafkaConfig ? {
                mechanism: "plain",
                ...ConfluentSecretRetriever.confluentKafkaSecret,
            } : undefined,
        }
    );

    main = new Main(webApp, kafkaConsumer, kafkaProducer);
    if (Config.useConfluentKafkaConfig){
        await ConfluentSecretRetriever.watch(() => {
            Config.logger.info("Restarting application to reload Kafka keys");
            // do not await stop as it could get stuck
            main.stop();
            return setTimeout(() => process.exit(1), 10 * 1000);
        });
    }

    await main.start();
}

["unhandledRejection", "uncaughtException"].forEach((errorType: never) => {
    process.on(errorType, async (err: Error) => {
        try {
            nr.noticeError(err);
            Config.logger.error(`Received '${errorType}': ${err.stack || err}`);
            if (main) {
                main.stop();
            }
        } finally {
            setTimeout(() => process.exit(1), 30000);
        }
    });
});

run();
