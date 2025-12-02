import { Logger } from "at-realtime-common/logger";

/**
 * Runtime configuration. Properties are read from environment variables.
 */
export default class Config {
    public static appName = process.env.APP_NAME || "at-dilax-apc-connector";
    public static port = process.env.PORT || "3000";
    public static newRelicPrefix = Config.appName;

    public static logger = new Logger(this, {
        prefix: process.env.IS_SLOT === "true" ? "[SLOT]" : "",
        level: process.env.LOG_LEVEL || "info",
        system: process.env.PAPERTRAIL_SYSTEM || "",
    });

    public static replication = {
        endpoint: process.env.REPLICATION_ENDPOINT,
    };
}
