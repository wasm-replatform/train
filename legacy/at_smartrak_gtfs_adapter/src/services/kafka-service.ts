import { Injectable } from "@nestjs/common";
import * as moment from "moment-timezone";
import * as nr from "newrelic";

import { Ctx, Payload } from "@nestjs/microservices";
import { ConfluentTopics, KafkaMessage, KafkaProducer } from "at-realtime-common/kafka";
import { Logger } from "at-realtime-common/logger";
import { SmarTrakEvent } from "at-realtime-common/model";
import { FleetApiService, VehicleInfo } from "../apis/fleet";
import { AppConfig } from "../config/app";
import { KafkaProducerConfig } from "../config/kafka-producer";
import { Tags } from "../constant";
import { GodMode } from "../processors/god-mode";
import { LocationProcessor } from "../processors/location";
import { PassengerCountEvent, PassengerCountProcessor } from "../processors/passenger-count";
import { SerialDataProcessor } from "../processors/serial-data";
import { transit_realtime } from "at-realtime-common/gtfs-realtime";
import { DeadReckoningMessage } from "../model/dead-reckoning";

import FeedEntity = transit_realtime.FeedEntity;

@Injectable()
export class KafkaService {
    constructor(
        private logger: Logger,
        private appConfig: AppConfig,
        private kafkaProducer: KafkaProducer,
        private kafkaProducerConfig: KafkaProducerConfig,
        private passengerCountProcessor: PassengerCountProcessor,
        private fleetApiService: FleetApiService,
        private godMode: GodMode,
        private serialDataProcessor: SerialDataProcessor,
        private locationProcessor: LocationProcessor,
    ) {}

    public async onApplicationBootstrap() {
        this.logger.info(`Kafka producer started in ${this.kafkaProducerConfig.environment} configuration`);
    }

    public async process(@Payload() event: SmarTrakEvent | PassengerCountEvent, @Ctx() message: KafkaMessage) {
        if (message.topic.includes(ConfluentTopics.PASSENGER_COUNT)) {
            await nr.startBackgroundTransaction(`${this.appConfig.newRelicPrefix}/passenger_count`, async () => {
                return await this.passengerCountProcessor.process(event as PassengerCountEvent);
            });
        } else {
            const smartrakEvent = event as SmarTrakEvent;
            const vehicleIdOrLabel = smartrakEvent.remoteData?.externalId || smartrakEvent.remoteData?.remoteName || "";
            const vehicleInfo = await this.fleetApiService.getVehicleInfoByIdOrLabel(vehicleIdOrLabel);
            if (!vehicleInfo) {
                this.logger.debug(`Skip processing the event from ${message.topic} topic as the vehicle ${vehicleIdOrLabel} is not found.`);
                nr.incrementMetric(`${this.appConfig.newRelicPrefix}/${message.topic}/skipped_message_counter`, 1);
                return;
            }

            if (message.topic.includes(ConfluentTopics.CAF_AVL)) {
                if (this.isMatchingTag(vehicleInfo, Tags.CAF)) {
                    await this.processSmartrakEvent(smartrakEvent, vehicleInfo);
                    nr.incrementMetric(`${this.appConfig.newRelicPrefix}/${message.topic}/processed_message_counter`, 1);
                } else {
                    this.logger.debug("Skip processing the event from caf topic as the vehicle tag doesn't match.");
                    nr.incrementMetric(`${this.appConfig.newRelicPrefix}/${message.topic}/skipped_message_counter`, 1);
                }
                return;
            }

            if (this.isMatchingTag(vehicleInfo, Tags.Smartrak) || message.topic.includes("realtime-r9k-to-smartrak")) {
                await this.processSmartrakEvent(smartrakEvent, vehicleInfo);
                nr.incrementMetric(`${this.appConfig.newRelicPrefix}/${message.topic}/processed_message_counter`, 1);
            } else {
                this.logger.debug("Skip processing the event from smartrak topics as the vehicle tag doesn't match.");
                nr.incrementMetric(`${this.appConfig.newRelicPrefix}/${message.topic}/skipped_message_counter`, 1);
            }
        }
    }

    private async processSmartrakEvent(event: SmarTrakEvent, vehicleInfo: VehicleInfo) {
        if (this.godMode && this.appConfig.enableGodMode) {
            this.godMode.preprocess(event);
        }

        if (event.eventType === "SerialData") {
            await nr.startBackgroundTransaction(`${this.appConfig.newRelicPrefix}/serial_data`, async () => {
                return await this.serialDataProcessor.process(event);
            });
            nr.recordMetric(`${this.appConfig.newRelicPrefix}/serial_data/processing_lag`, moment().unix() - moment.utc(event.messageData.timestamp).unix());
        }

        if (event.eventType === "Location") {
            const vehiclePosition = await nr.startBackgroundTransaction(`${this.appConfig.newRelicPrefix}/location`, async () => {
                return await this.locationProcessor.process(event, vehicleInfo);
            });
            nr.recordMetric(`${this.appConfig.newRelicPrefix}/location/processing_lag`, moment().unix() - moment.utc(event.messageData.timestamp).unix());

            if (vehiclePosition && vehiclePosition instanceof DeadReckoningMessage) {
                this.kafkaProducer
                    .publish(this.kafkaProducerConfig.drTopic, {
                        value: JSON.stringify(vehiclePosition),
                        key: vehiclePosition.vehicle.id,
                    })
                    .catch((err) => {
                        this.logger.error(`Failed to publish dr message, error: ${JSON.stringify(err)}`);
                    });
            }

            if (vehiclePosition && vehiclePosition instanceof FeedEntity) {
                this.kafkaProducer
                    .publish(this.kafkaProducerConfig.vpTopic, {
                        value: JSON.stringify(vehiclePosition),
                        key: vehiclePosition.id,
                    })
                    .catch((err) => {
                        this.logger.error(`Failed to publish vp message, error: ${JSON.stringify(err)}`);
                    });
            }
        }
    }

    private isMatchingTag(vehicleInfo: VehicleInfo, expectedTag: string): boolean {
        return vehicleInfo.tag?.toLowerCase() === expectedTag.toLowerCase();
    }
}
