import { Injectable } from "@nestjs/common";
import { Logger } from "at-realtime-common/logger";

import { CacheRepository } from "../repositories/cache";
import { RedisConfig } from "../config/redis";

export interface PassengerCountEvent {
    occupancyStatus?: string;
    vehicle: { id: string };
    trip: {
        tripId: string;
        routeId: string;
        startDate: string;
        startTime: string;
    };
    timestamp: number;
}

@Injectable()
export class PassengerCountProcessor {
    constructor(private logger: Logger, private cacheRepository: CacheRepository, private redisConfig: RedisConfig) {}

    public async process(event: PassengerCountEvent): Promise<void> {
        const redisKey = `${this.redisConfig.keys.passengerCountKey}:${event.vehicle.id}:${event.trip.tripId}:${event.trip.startDate}:${event.trip.startTime}`;
        this.logger.debug(`Set the occupancy status of ${redisKey} to ${event.occupancyStatus}`);
        await this.cacheRepository.setexAsync(redisKey, 3 * 60 * 60, JSON.stringify(event));
    }
}
