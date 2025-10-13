import { Injectable } from "@nestjs/common";
import { ILoggerConfig } from "at-realtime-common/logger";

@Injectable()
export class LoggerConfig implements ILoggerConfig {
    public prefix = process.env.IS_SLOT === "true" ? "[SLOT]" : "";
    public level = process.env.LOG_LEVEL || "info";
    public system = process.env.PAPERTRAIL_SYSTEM || "";
}
