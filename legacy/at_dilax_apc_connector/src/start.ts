import Config from "./config";
import { ConfluentSecretRetriever } from "./confluent-secret-retriever";
import { onException } from "at-realtime-common/common";
import { Main } from "./main";
import { KafkaProducerConfig } from "./kafka-producer-config";

let main: Main;
async function start() {
    main = new Main();

    const kafkaProducerConfig = new KafkaProducerConfig();
    if (kafkaProducerConfig.useConfluentKafkaConfig) {
        await ConfluentSecretRetriever.retrieve(closeApp);
    }

    main.start()
        .then(() => Config.logger.info("DILAX APC connector started"))
        .catch((reason) => Config.logger.error(`Failed to start DILAX APC connector: ${reason}`));
}

const closeApp = () => {
    Config.logger.info("Restarting application to reload Kafka keys");
    // do not await stop as it could get stuck
    main?.close();
    return setTimeout(() => process.exit(1), 10 * 1000);
};

["unhandledRejection", "uncaughtException"].forEach((errorType) => {
    process.on(errorType, (err) => onException(main, Config.logger, errorType, err));
});

start();
