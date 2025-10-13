import { Injectable } from "@nestjs/common";
import { IAppConfig } from "at-realtime-common/common";

@Injectable()
export class AppConfig implements IAppConfig {
    public port = process.env.PORT || "3000";
    public appName = process.env.APP_NAME || "";
    public timezone = process.env.TIMEZONE || "Pacific/Auckland";
    public isSlot = process.env.IS_SLOT === "true";
    public newRelicPrefix = this.appName;
    public swagger = process.env.SWAGGER === "true";

    public enableGodMode = process.env.GOD_MODE === "true";
    public fleetApiUrl = process.env.FLEET_API_URL || "https://www-dev-at-fleet-api-01.azurewebsites.net";
    public tripManagementUrl = process.env.TRIP_MANAGEMENT_URL || "https://www-dev-trip-mgt-api-01.azurewebsites.net";
    public blockManagementUrl = process.env.BLOCK_MANAGEMENT_URL || "https://www-dev-block-mgt-api-01.azurewebsites.net";

    public accuracyThreshold = parseInt(process.env.ACCURACY_THRESHOLD || "0", 10);

    public tripDurationBuffer = parseInt(process.env.TRIP_DURATION_BUFFER || "3600", 10);
    public serialDataFilterThreshold = parseInt(process.env.SERIAL_DATA_FILTER_THRESHOLD || "900", 10);
    public defaultTrainTotalCapacity = parseInt(process.env.DEFAULT_TRAIN_TOTAL_CAPACITY || "373", 10);
    public defaultTrainSeatingCapacity = parseInt(process.env.DEFAULT_TRAIN_SEATING_CAPACITY || "230", 10);

    public secretWatcherInterval = Number(process.env.SECRET_WATCHER_INTERVAL) || 5;
}
