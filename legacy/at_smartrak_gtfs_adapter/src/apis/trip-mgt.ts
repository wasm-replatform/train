import * as nr from "newrelic";
import * as moment from "moment-timezone";

import { Injectable } from "@nestjs/common";
import { AuthTokenRetriever } from "at-realtime-common/auth";
import { HttpClient } from "at-realtime-common/http-client";
import { Logger } from "at-realtime-common/logger";
import { transit_realtime } from "at-realtime-common/gtfs-realtime";

import { AppConfig } from "../config/app";
import { RedisConfig } from "../config/redis";
import { CacheRepository } from "../repositories/cache";

import TripDescriptor = transit_realtime.TripDescriptor;

enum TripState {
    OK,
    ERROR,
}

export class TripInstance {
    public readonly tripId: string;
    public readonly routeId: string;
    public readonly serviceDate: string;
    public readonly startTime: string;
    public readonly endTime: string;
    public readonly directionId: number;
    public readonly isAddedTrip: boolean;
    public readonly error: boolean;

    private state: TripState;

    constructor(config: Partial<TripInstance> = {}) {
        this.tripId = config.tripId || "";
        this.endTime = config.endTime || "";
        this.routeId = config.routeId || "";
        this.serviceDate = config.serviceDate || "";
        this.startTime = config.startTime || "";
        this.directionId = config.directionId || 0;
        this.isAddedTrip = config.isAddedTrip || false;
        this.error = config.error || false;

        this.state = TripState.OK;

        if (this.error) {
            this.state = TripState.ERROR;
        }
    }

    public static from(config?: Partial<TripInstance>): TripInstance {
        return new TripInstance(config);
    }

    public hasError(): boolean {
        return this.state === TripState.ERROR;
    }

    public toTripDescriptor(): TripDescriptor {
        const { tripId, routeId, serviceDate, startTime, directionId, isAddedTrip } = this;
        return TripDescriptor.create({
            tripId,
            routeId,
            startTime,
            directionId,
            startDate: serviceDate,
            scheduleRelationship: isAddedTrip ? TripDescriptor.ScheduleRelationship.ADDED : TripDescriptor.ScheduleRelationship.SCHEDULED,
        });
    }

    // Javascript method which is called automatically during JSON stringify
    public toJSON(): Partial<TripInstance> {
        return {
            tripId: this.tripId,
            routeId: this.routeId,
            serviceDate: this.serviceDate,
            endTime: this.endTime,
            startTime: this.startTime,
            directionId: this.directionId,
            isAddedTrip: this.isAddedTrip,
            error: this.error,
        };
    }

    public remap(tripId: string, routeId: string): TripInstance {
        return new TripInstance({
            ...this,
            tripId,
            routeId,
        });
    }
}

@Injectable()
export class TripMgtApi {
    private tripInstancesUrl: string;

    constructor(
        private httpClient: HttpClient,
        private cacheRepository: CacheRepository,
        private appConfig: AppConfig,
        private authTokenRetriever: AuthTokenRetriever,
        private logger: Logger,
        private redisConfig: RedisConfig,
    ) {
        this.tripInstancesUrl = `${this.appConfig.tripManagementUrl}/tripinstances`;
    }

    public async getTripInstance(tripId: string, serviceDate: string, startTime: string): Promise<TripInstance | undefined> {
        const tripInstances: TripInstance[] = await this.getTrips(tripId, serviceDate);
        const tripInstance = tripInstances.find((instance: TripInstance) => instance.startTime === startTime);

        if (!tripInstance && tripInstances[0] && tripInstances[0].hasError()) {
            return tripInstances[0];
        }

        return tripInstance;
    }

    public async getNearestTripInstance(tripId: string, eventTimestamp: number): Promise<TripInstance | undefined> {
        const momentWithTimeZone = moment.unix(eventTimestamp).tz(this.appConfig.timezone);

        const currentHours = momentWithTimeZone.format("HH");
        const currentServiceDate = momentWithTimeZone.format("YYYYMMDD");

        let tripInstances = await this.getTrips(tripId, currentServiceDate);

        if (tripInstances[0] && tripInstances[0].hasError()) {
            return tripInstances[0];
        }

        if (Number(currentHours) < 4) {
            const previousDayTripInstances = await this.getTrips(tripId, momentWithTimeZone.subtract(1, "day").format("YYYYMMDD"));
            if (previousDayTripInstances[0] && previousDayTripInstances[0].hasError()) {
                return previousDayTripInstances[0];
            }
            tripInstances = tripInstances.concat(previousDayTripInstances);
        }

        if (!tripInstances.length) {
            return;
        }

        tripInstances.sort((left: TripInstance, right: TripInstance) => {
            // find nearest trip to event time
            // we need to pass time separately to work correctly with time over 24 hours
            const leftTime = moment
                .tz(`${left.serviceDate}`, "YYYYMMDD", this.appConfig.timezone)
                .hours(Number(left.startTime.split(":")[0]))
                .minutes(Number(left.startTime.split(":")[1]))
                .seconds(Number(left.startTime.split(":")[2]))
                .unix();

            const rightTime = moment
                .tz(`${right.serviceDate}`, "YYYYMMDD", this.appConfig.timezone)
                .hours(Number(right.startTime.split(":")[0]))
                .minutes(Number(right.startTime.split(":")[1]))
                .seconds(Number(right.startTime.split(":")[2]))
                .unix();

            return Math.abs(eventTimestamp - leftTime) - Math.abs(eventTimestamp - rightTime);
        });

        return tripInstances[0];
    }

    private async getTrips(tripId: string, serviceDate: string): Promise<TripInstance[]> {
        const tripManagementKey = `${this.redisConfig.keys.tripManagementKey}:${tripId}:${serviceDate}`;

        let response: TripInstance[] | undefined = await this.cacheRepository
            .getAsync(tripManagementKey)
            .then((data: string | undefined) => (data ? JSON.parse(data).map((trip: TripInstance) => TripInstance.from(trip)) : undefined))
            .catch((err: Error) => {
                this.logger.error(`Could not parse tripInstances from '${tripManagementKey}': ${err.stack || err.message}`);
                return undefined;
            });

        if (!response) {
            const body = { tripIds: [tripId], serviceDate: serviceDate };
            const headers = { Authorization: `Bearer ${await this.authTokenRetriever.getToken()}` };

            response = await this.httpClient
                .post(this.tripInstancesUrl, body, { headers, retry: true })
                .then(async (res) => {
                    let tripInstances: TripInstance[] = [];

                    if (res.data?.tripInstances?.length) {
                        tripInstances = res.data.tripInstances.map((trip: TripInstance) => TripInstance.from(trip));
                    }

                    await this.cacheRepository.setexAsync(tripManagementKey, 20, JSON.stringify(tripInstances));
                    return tripInstances;
                })
                .catch(async (err: Error) => {
                    nr.noticeError(err);

                    const errorTripInstances = [TripInstance.from({ error: true })];
                    this.logger.error(`Calling Trip Management API '${this.tripInstancesUrl} ${JSON.stringify(body)}': ${err.stack || err.message}`);

                    await this.cacheRepository.setexAsync(tripManagementKey, 10, JSON.stringify(errorTripInstances));
                    return errorTripInstances;
                });
        }

        return response;
    }
}
