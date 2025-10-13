import { Injectable } from "@nestjs/common";
import { IRedisConfig } from "at-realtime-common/redis";
@Injectable()
export class RedisConfig implements IRedisConfig {
    public host = process.env.REDIS_HOST || "";
    public port = Number(process.env.REDIS_PORT || 6379);
    public password = process.env.REDIS_PASSWORD || "";
    public db = Number(process.env.REDIS_DB || 0);
    public keys = {
        tripKey: "smartrakGtfs:trip:vehicle",
        fleetKey: "smartrakGtfs:fleet",
        vehicleSOTimeKey: "smartrakGtfs:vehicle:signOn",
        tripManagementKey: "smartrakGtfs:tripManagement",
        blockManagementKey: "smartrakGtfs:blockManagement",
        allocatedVehicleKey: "smartrakGtfs:trip:allocatedVehicle",
        vehicleBlacklistKey: "smartrakGtfs:vehicleBlacklist",
        passengerCountKey: "smartrakGtfs:passengerCountEvent",
    };
}
