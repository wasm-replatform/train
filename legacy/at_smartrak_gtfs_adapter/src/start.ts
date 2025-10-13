import { MicroserviceOptions } from "@nestjs/microservices";
import { Container } from "./container";

import { onException, onExitSignal } from "at-realtime-common/common";
import { KafkaConsumer, KafkaConsumerTransporter, KafkaProducer } from "at-realtime-common/kafka";
import { Logger } from "at-realtime-common/logger";
import { Redis } from "at-realtime-common/redis";
import { INestApplicationExtended, ServerFactory } from "at-realtime-common/server";

import { AppConfig } from "./config/app";

let app: INestApplicationExtended;
let logger: Logger;

async function bootstrap() {
    app = await ServerFactory.create(Logger, AppConfig, Container);
    logger = app.get(Logger);
    logger.info("Starting AT Smartrak GTFS Adapter...");

    app.connectMicroservice<MicroserviceOptions>({ strategy: app.get(KafkaConsumerTransporter) });

    app.configureStatusCheck([app.get(KafkaConsumer), app.get(KafkaProducer), app.get(Redis)]);

    await app.startAllMicroservices();
    await app.listen(app.get(AppConfig).port);
}

["SIGINT", "SIGTERM"].forEach((signalType) => {
    process.on(signalType, () => onExitSignal(app, logger, signalType));
});

["unhandledRejection", "uncaughtException"].forEach((errorType) => {
    process.on(errorType, (err) => onException(app, logger, errorType, err));
});

bootstrap();
