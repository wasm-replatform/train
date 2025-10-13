import * as nr from "newrelic";
import * as moment from "moment-timezone";
import { Injectable } from "@nestjs/common";

import { SmarTrakEvent } from "at-realtime-common/model";
import { Logger } from "at-realtime-common/logger";
import { transit_realtime } from "at-realtime-common/gtfs-realtime";

import { AppConfig } from "../config/app";
import { RedisConfig } from "../config/redis";
import { CacheRepository } from "../repositories/cache";
import { TripMgtApi, TripInstance } from "../apis/trip-mgt";
import { BlockMgtApi, BlockInstance } from "../apis/block-mgt";
import { VehicleInfo } from "../apis/fleet";
import { v4 as uuid } from "uuid";

import Position = transit_realtime.Position;
import FeedEntity = transit_realtime.FeedEntity;
import VehiclePosition = transit_realtime.VehiclePosition;
import OccupancyStatus = transit_realtime.VehiclePosition.OccupancyStatus;
import VehicleDescriptor = transit_realtime.VehicleDescriptor;
import TripDescriptor = transit_realtime.TripDescriptor;
import { PassengerCountEvent } from "./passenger-count";
import { DeadReckoningMessage, PositionDR, VehicleDR } from "../model/dead-reckoning";

const AsyncLock = require("async-lock");

@Injectable()
export class LocationProcessor {
    private lock = new AsyncLock({ maxPending: 30000 });

    constructor(
        private logger: Logger,
        private appConfig: AppConfig,
        private redisConfig: RedisConfig,
        private cacheRepository: CacheRepository,
        private tripMgtApi: TripMgtApi,
        private blockMgtApi: BlockMgtApi,
    ) {}

    public async process(event: SmarTrakEvent, vehicleInfo: VehicleInfo): Promise<FeedEntity | DeadReckoningMessage | undefined> {
        this.logger.debug(`Location event: ${JSON.stringify(event)}`);
        if (!this.isValid(event)) {
            return;
        }

        const vehicleIdOrLabel = event.remoteData.externalId || event.remoteData.remoteName || "";

        return this.lock
            .acquire(`location:${vehicleIdOrLabel}`, async () => {
                const eventTimestamp = moment.utc(event.messageData.timestamp).unix();
                const vehicleDescriptor = new VehicleDescriptor({
                    id: vehicleInfo.id,
                    label: vehicleInfo.label,
                    licensePlate: vehicleInfo.registration,
                });

                if (vehicleInfo.type.type === "Train") {
                    const blockInstance: BlockInstance | undefined = await this.blockMgtApi.getAllocationByVehicleId(vehicleInfo.id, eventTimestamp);
                    await this.assignTrainToTrip(blockInstance, vehicleInfo, eventTimestamp);
                }

                const tripInstance = await this.getTripInstance(vehicleDescriptor, eventTimestamp);
                const tripDescriptor = tripInstance?.toTripDescriptor();

                const odometer = event.locationData.odometer ?? event.eventData.odometer ?? null;
                if ((!event.locationData?.latitude || !event.locationData?.longitude) && odometer && tripDescriptor) {
                    return new DeadReckoningMessage(uuid(), eventTimestamp, new PositionDR(odometer), tripDescriptor, new VehicleDR(vehicleInfo.id));
                }

                return new FeedEntity({
                    id: vehicleDescriptor.id,
                    vehicle: new VehiclePosition({
                        position: new Position({
                            latitude: event.locationData.latitude,
                            longitude: event.locationData.longitude,
                            bearing: event.locationData.heading,
                            speed: (event.locationData.speed * 1000) / 3600,
                            odometer,
                        }),
                        trip: tripDescriptor,
                        vehicle: vehicleDescriptor,
                        occupancyStatus: tripDescriptor ? await this.getOccupancyStatus(vehicleInfo.id, tripDescriptor) : undefined,
                        timestamp: eventTimestamp,
                    }),
                });
            })
            .catch((err: Error) => {
                this.logger.error(`Could not process '${JSON.stringify(event)}', key "location:${vehicleIdOrLabel}": ${err.stack || err.message}`);
            });
    }

    private async getTripInstance(vehicleDescriptor: VehicleDescriptor, eventTimestamp: number): Promise<TripInstance | undefined> {
        const tripVehicleKey = `${this.redisConfig.keys.tripKey}:${vehicleDescriptor.id}`;
        const signOnVehicleTimeKey = `${this.redisConfig.keys.vehicleSOTimeKey}:${vehicleDescriptor.id}`;

        const tripInstance: TripInstance | undefined = await this.cacheRepository
            .getAsync(tripVehicleKey)
            .then((data: string) => (data ? TripInstance.from(JSON.parse(data)) : undefined))
            .catch((err: Error) => {
                this.logger.error(`Could not parse tripInstance from '${tripVehicleKey}': ${err.stack || err.message}`);
                return undefined;
            });

        if (tripInstance) {
            const signOnEventSec = await this.cacheRepository.getAsync(signOnVehicleTimeKey);
            if (tripInstance.startTime && tripInstance.endTime && signOnEventSec) {
                const endTimeSplit = tripInstance.endTime.split(":");
                const startTimeSplit = tripInstance.startTime.split(":");

                // passing hours, minutes, and seconds separately is necessary for 24+ hours trips
                const tripEndTimeUnix = moment
                    .tz(`${tripInstance.serviceDate}`, "YYYYMMDD", this.appConfig.timezone)
                    .hours(Number(endTimeSplit[0]))
                    .minutes(Number(endTimeSplit[1]))
                    .seconds(Number(endTimeSplit[2]))
                    .unix();

                const tripStartTimeUnix = moment
                    .tz(`${tripInstance.serviceDate}`, "YYYYMMDD", this.appConfig.timezone)
                    .hours(Number(startTimeSplit[0]))
                    .minutes(Number(startTimeSplit[1]))
                    .seconds(Number(startTimeSplit[2]))
                    .unix();

                const tripDuration = tripEndTimeUnix - tripStartTimeUnix + this.appConfig.tripDurationBuffer;

                // force remove trip descriptor if it is out of trip duration range
                if (eventTimestamp - tripDuration > Number(signOnEventSec)) {
                    nr.incrementMetric(`${this.appConfig.newRelicPrefix}/location/passed_trip_duration`, 1);
                    return;
                }
            }

            return tripInstance;
        }
    }

    private async assignTrainToTrip(blockInstance: BlockInstance | undefined, vehicleInfo: VehicleInfo, eventTimestamp: number) {
        const tripVehicleKey = `${this.redisConfig.keys.tripKey}:${vehicleInfo.id}`;
        const signOnVehicleTimeKey = `${this.redisConfig.keys.vehicleSOTimeKey}:${vehicleInfo.id}`;

        // got an error ignore processing this event
        if (blockInstance && blockInstance.hasError()) {
            return;
        }

        // no allocation for this vehicle or it is not main cargo
        if (!blockInstance || (blockInstance && blockInstance.vehicleIds[0] !== vehicleInfo.id)) {
            await this.cacheRepository.deleteAsync(signOnVehicleTimeKey);
            await this.cacheRepository.deleteAsync(tripVehicleKey);
            return;
        }

        const prevTripInstance: TripInstance | undefined = await this.cacheRepository
            .getAsync(tripVehicleKey)
            .then((data: string) => (data ? TripInstance.from(JSON.parse(data)) : undefined))
            .catch((err: Error) => {
                this.logger.error(`Could not parse prevTripInstance from '${tripVehicleKey}': ${err.stack || err.message}`);
                return undefined;
            });

        if (prevTripInstance) {
            const isExactlyTheSame =
                prevTripInstance.tripId === blockInstance.tripId &&
                prevTripInstance.startTime === blockInstance.startTime &&
                prevTripInstance.serviceDate === blockInstance.serviceDate;
            // allocated trip and new trip are exactly the same do nothing
            if (isExactlyTheSame) {
                return;
            }
        }

        const newTripInstance: TripInstance | undefined = await this.tripMgtApi.getTripInstance(blockInstance.tripId, blockInstance.serviceDate, blockInstance.startTime);

        // no trip found
        if (!newTripInstance) {
            await this.cacheRepository.deleteAsync(signOnVehicleTimeKey);
            await this.cacheRepository.deleteAsync(tripVehicleKey);
            return;
        }

        // allocate new trip to this vehicle if it does not have error
        if (!newTripInstance.hasError()) {
            nr.incrementMetric(`${this.appConfig.newRelicPrefix}/train/trip_descriptors`);
            await this.cacheRepository.setexAsync(signOnVehicleTimeKey, 24 * 60 * 60, JSON.stringify(eventTimestamp));
            await this.cacheRepository.setexAsync(tripVehicleKey, 3 * 60 * 60, JSON.stringify(newTripInstance));
        }
    }

    private async getOccupancyStatus(vehicleId: string, trip: TripDescriptor): Promise<OccupancyStatus | undefined> {
        const passengerCountKey = `${this.redisConfig.keys.passengerCountKey}:${vehicleId}:${trip.tripId}:${trip.startDate}:${trip.startTime}`;
        const passengerCount: PassengerCountEvent | undefined = await this.cacheRepository
            .getAsync(passengerCountKey)
            .then((data: string) => (data ? JSON.parse(data) : undefined))
            .catch((err: Error) => {
                this.logger.error(`Could not get passenger count from '${passengerCountKey}': ${err.stack || err.message}`);
                return undefined;
            });
        if (passengerCount) {
            return OccupancyStatus[passengerCount.occupancyStatus as keyof typeof OccupancyStatus];
        }
    }

    private isValid(event: SmarTrakEvent): boolean {
        if (!event.remoteData) {
            nr.incrementMetric(`${this.appConfig.newRelicPrefix}/location/invalid_message`, 1);
            return false;
        }

        if (event.locationData.gpsAccuracy < this.appConfig.accuracyThreshold) {
            nr.incrementMetric(`${this.appConfig.newRelicPrefix}/rejected_bad_gps`, 1);
            return false;
        }

        return true;
    }
}
