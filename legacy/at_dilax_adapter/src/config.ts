/* eslint-disable @typescript-eslint/explicit-module-boundary-types */
import { IAuthTokenRetrieverConfig } from "at-realtime-common/auth";
import { IAppConfig } from "at-realtime-common/common";
import { Logger } from "at-realtime-common/logger";

/**
 * Runtime configuration. Properties are read from environment variables.
 */
export class Config implements Partial<IAppConfig> {
    public static port = process.env.PORT || "3000";
    public static isSlot = process.env.IS_SLOT === "true";
    public static appName = process.env.APP_NAME || "";
    public static readonly timezone = "Pacific/Auckland";
    public static newRelicPrefix = Config.appName;

    public static fleetApiUrl = process.env.FLEET_API_URL || "";
    public static gtfsStaticApiUrl = process.env.GTFS_STATIC_URL || "";
    public static blockMgtClientApiUrl = process.env.BLOCK_MGT_CLIENT_API_URL || "";
    public static resetCountOnTripEnded: boolean = process.env.RESET_COUNT_ON_TRIP_ENDED === "true";
    private static azureAccessTokenRetrieverConfig: IAuthTokenRetrieverConfig;
    public static APC_TTL_SECS = parseInt(process.env.APC_TTL_SECS || "3600", 10);
    public static dilaxConnectionLostThreshold = parseInt(process.env.DILAX_CONNECTION_LOST_THRESHOLD || "60", 10);

    public static secretWatcherInterval = Number(process.env.SECRET_WATCHER_INTERVAL) || 5;
    public static useConfluentKafkaConfig = process.env.IS_LOCAL !== "true";

    public static cc_static_api = {
        uri: process.env.CC_STATIC_API_HOST || "",
    };

    public static redis = {
        keyOccupancy: "trip:occupancy", // backward-compatibilty, to be removed when dependent repo (CC UI) has migrated to read from state key
        apcVehicleIdMigratedKey: "apc:vehicleIdMigratred", // to keep track of migration, to be removed in next prod version
        apcVehicleIdKey: "apc:vehicleId", // backward-compatibilty, to be removed when dependent repo (smartrak_gtfs_adapter) has migrated to read from state key
        apcVehicleTripKey: "apc:trips", // backward-compatibilty, to be removed in next prod version
        apcVehicleIdStateKey: "apc:vehicleIdState",
        keyVehicleTripInfo: "apc:vehicleTripInfo",
        vehicleLabelKey: "smartrakGtfs:vehicleLabel",
        host: process.env.REDIS_HOST || "localhost",
        port: process.env.REDIS_PORT || 6379,
        password: process.env.REDIS_PASSWORD,
        db: Number(process.env.REDIS_DB) || 0,
        lostConnectionsSet: "apc:lostConnections",
    };

    public static logger = new Logger(this, {
        prefix: process.env.IS_SLOT === "true" ? "[SLOT]" : "",
        level: process.env.LOG_LEVEL || "info",
        system: process.env.PAPERTRAIL_SYSTEM || "",
    });

    public static getAzureAccessTokenRetrieverConfig = (): IAuthTokenRetrieverConfig => {
        if (!Config.azureAccessTokenRetrieverConfig) {
            Config.azureAccessTokenRetrieverConfig = {
                clientId: process.env.APP_MANIFEST_CLIENT_ID || "",
                domain: "AucklandTransport.govt.nz",
                keyVault: {
                    host: `https://${process.env.KEY_VAULT || ""}.vault.azure.net`,
                    secretNameSystemClientSecret: process.env.KEY_VAULT_SECRET_NAME_SYSTEM_CLIENT_SECRET || "",
                },
                localDevEnv: {
                    accessToken: "",
                },
            };
        }
        return Config.azureAccessTokenRetrieverConfig;
    };
}
