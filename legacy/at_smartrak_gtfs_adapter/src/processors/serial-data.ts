import * as nr from "newrelic";
import * as moment from "moment-timezone";
import { Injectable } from "@nestjs/common";

import { Logger } from "at-realtime-common/logger";
import { DecodedSerialData, SmarTrakEvent } from "at-realtime-common/model";

import { AppConfig } from "../config/app";
import { RedisConfig } from "../config/redis";
import { CacheRepository } from "../repositories/cache";
import { TripMgtApi, TripInstance } from "../apis/trip-mgt";

const AsyncLock = require("async-lock");

@Injectable()
export class SerialDataProcessor {
    private lock = new AsyncLock({ maxPending: 10000 });
    private vehicleToTimestamp: { [key: string]: number } = {};

    constructor(private logger: Logger, private appConfig: AppConfig, private redisConfig: RedisConfig, private cacheRepository: CacheRepository, private tripMgtApi: TripMgtApi) {}

    public async process(event: SmarTrakEvent) {
        this.logger.debug(`SerialData event: ${JSON.stringify(event)}`);
        if (!this.isValid(event)) {
            return;
        }

        const vehicleId = event.remoteData.externalId;
        const decodedEvent = event.serialData.decodedSerialData;

        return this.lock
            .acquire(`serialData:${vehicleId}`, async () => {
                if (this.isOld(event)) {
                    return;
                }

                const eventTimestamp = moment.utc(event.messageData.timestamp).unix();
                return await this.allocateVehicleToTrip(vehicleId, decodedEvent, eventTimestamp);
            })
            .catch((err: Error) => {
                this.logger.error(`Could not process '${JSON.stringify(event)}', key "serialData:${vehicleId}": ${err.stack || err.message}`);
            });
    }

    private async allocateVehicleToTrip(vehicleId: string, decodedEvent: DecodedSerialData | undefined, eventTimestamp: number) {
        const tripVehicleKey = `${this.redisConfig.keys.tripKey}:${vehicleId}`;
        const signOnVehicleTimeKey = `${this.redisConfig.keys.vehicleSOTimeKey}:${vehicleId}`;

        const tripId = (decodedEvent as unknown as { tripId: string | undefined }).tripId;
        if (!tripId) {
            // if no tripNumber provided just remove descriptor for that vehicle
            await this.cacheRepository.deleteAsync(signOnVehicleTimeKey);
            return await this.cacheRepository.deleteAsync(tripVehicleKey);
        }
        const prevTripInstance: TripInstance | undefined = await this.cacheRepository
            .getAsync(tripVehicleKey)
            .then((data: string | undefined) => (data ? TripInstance.from(JSON.parse(data)) : undefined))
            .catch((err: Error) => {
                // we can not do anything if cache fails
                this.logger.error(`Could not parse prevTripInstance from '${tripVehicleKey}': ${err.stack || err.message}`);
                return undefined;
            });

        let newTripInstance: TripInstance | undefined;
        if (prevTripInstance) {
            // we have old trip attached and it is the same as new one
            // no need to check time to make sure we are always attached to
            // the same copy trip
            if (prevTripInstance.tripId === tripId) {
                return;
            }

            // if it is not the same the try to get new trip
            newTripInstance = await this.tripMgtApi.getNearestTripInstance(tripId, eventTimestamp);
            if (!newTripInstance || newTripInstance.hasError()) {
                // if trip ids do not match and we got an error or empty response from trip mgt then remove prevTripInstance from cache
                await this.cacheRepository.deleteAsync(signOnVehicleTimeKey);
                await this.cacheRepository.deleteAsync(tripVehicleKey);
                return;
            }
        } else {
            // vehicle did not have trip attached find nearest trip for this vehicle
            newTripInstance = await this.tripMgtApi.getNearestTripInstance(tripId, eventTimestamp);
        }
        // no trip or different trip attached to this vehicle set new in progress trip
        if (newTripInstance && !newTripInstance.hasError()) {
            nr.incrementMetric(`${this.appConfig.newRelicPrefix}/bus/trip_descriptors`);
            await this.cacheRepository.setexAsync(signOnVehicleTimeKey, 24 * 60 * 60, JSON.stringify(eventTimestamp));
            await this.cacheRepository.setexAsync(tripVehicleKey, 4 * 60 * 60, JSON.stringify(newTripInstance));
        }
    }

    private isValid(event: SmarTrakEvent): boolean {
        if (!event.remoteData || !event.remoteData.externalId || !event.serialData || !event.serialData.decodedSerialData) {
            this.logger.debug(`Cannot process Smartrak Event ${JSON.stringify(event)}`);
            nr.incrementMetric(`${this.appConfig.newRelicPrefix}/serial_data/invalid_message`, 1);
            return false;
        }

        const eventTimestamp = moment.utc(event.messageData.timestamp).unix();

        if (eventTimestamp - moment().unix() > this.appConfig.serialDataFilterThreshold) {
            nr.incrementMetric(`${this.appConfig.newRelicPrefix}/serial_data/future_events`, 1);
            this.logger.warn(`Received serialData event with future time: ${JSON.stringify(event)}`);
            return false;
        }

        return true;
    }

    private isOld(event: SmarTrakEvent): boolean {
        const eventTimestamp = moment.utc(event.messageData.timestamp).unix();

        if (this.vehicleToTimestamp[event.remoteData.externalId] > eventTimestamp) {
            nr.incrementMetric(`${this.appConfig.newRelicPrefix}/serial_data/old_events`, 1);
            this.logger.warn(`Received older serialData event latest timestamp is ${this.vehicleToTimestamp[event.remoteData.externalId]} received: ${JSON.stringify(event)}`);
            return true;
        }

        this.vehicleToTimestamp[event.remoteData.externalId] = eventTimestamp;

        return false;
    }
}
