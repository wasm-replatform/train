import * as newrelic from "newrelic";
import { HttpClient } from "at-realtime-common/http-client";
import { Redis } from "at-realtime-common/redis";
import { Config } from "../config";

/**
 * NOTE: This class was copied from at_smartrak_gtfs_adapter, code can be viewed in that repo:
 * git checkout 0800be6
 * TODO: The class has evolved over time, so we should refactor this class to use the latest
 * copy or move the class into a common repository.
 */
export default class FleetApi {
    private log = Config.logger;

    constructor(private redisClient: Redis, private httpClient: HttpClient) {}

    public async trainByLabel(label: string) {
        const redisKey = `${Config.redis.vehicleLabelKey}:${label}`;
        try {
            const vehicleString = await this.redisClient.getAsync(redisKey);
            if (vehicleString) {
                return JSON.parse(vehicleString);
            }
        } catch (err) {
            newrelic.noticeError(err);
            this.log.error(`Failed to get cached vehicle details: ${err.message}`);
        }
        let nullTimeout = 24 * 60 * 60;
        const response = await this.httpClient
            .get(`${Config.fleetApiUrl}/vehicles?label=${label}`)
            .then((result) => result.data)
            .catch((error) => {
                // Cache failure for a short time
                newrelic.noticeError(error);
                nullTimeout = 3 * 60;
                this.log.error(`Error reading Fleet API: ${error.message}`);
            });
        if (response) {
            // There should only be one vehicle with this label but who knows, maybe a weird ferry or bus
            const vehicle = <NonNullable<unknown>[]>response.find((e: any) => e.type?.type?.toLowerCase() === "train") || null;
            await this.redisClient.setexAsync(redisKey, 24 * 60 * 60, JSON.stringify(vehicle)).catch((err) => {
                newrelic.noticeError(err);
                this.log.error(err);
            });
            return vehicle;
        } else {
            await this.redisClient.setexAsync(redisKey, nullTimeout, "null").catch((err) => {
                newrelic.noticeError(err);
                this.log.error(err);
            });
            return null;
        }
    }
}
