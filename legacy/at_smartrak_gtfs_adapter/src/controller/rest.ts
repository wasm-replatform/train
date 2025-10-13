import { Request, Get, Param, HttpException, HttpStatus } from "@nestjs/common";
import { CommonController } from "at-realtime-common/common";
import { Redis } from "at-realtime-common/redis";
import { Logger } from "at-realtime-common/logger";
import { Request as ExpressRequest } from "express";

import { GodMode } from "../processors/god-mode";
import { RedisConfig } from "../config/redis";
import { AppConfig } from "../config/app";
import { TripInstance } from "../apis/trip-mgt";
import { VehicleInfo } from "../apis/fleet";

interface VehicleInfoResponse {
    pid: number;
    vehicleId: string;
    signOnTime: string | null;
    tripInfo: TripInstance | undefined;
    fleetInfo: VehicleInfo | undefined;
}

interface ApiResponse {
    message: string;
    process: number;
}

@CommonController()
export class RestApiController {
    constructor(private logger: Logger, private redisClient: Redis, private redisConfig: RedisConfig, private appConfig: AppConfig, private godMode: GodMode) {}

    @Get("/")
    public index(@Request() req: ExpressRequest): string {
        const userAgent = req.headers["user-agent"];
        this.logger.info(`"/" called with UserAgent: ${userAgent}`);
        return "OK";
    }

    @Get("/info/:vehicleId")
    public async getVehicleInfoById(@Param("vehicleId") vehicleId: string): Promise<VehicleInfoResponse> {
        try {
            const tripInfo = await this.redisClient.getAsync(`${this.redisConfig.keys.tripKey}:${vehicleId}`).then((data) => (data ? JSON.parse(data) : undefined));
            const fleetInfo = await this.redisClient.getAsync(`${this.redisConfig.keys.fleetKey}:${vehicleId}`).then((data) => (data ? JSON.parse(data) : undefined));
            const signOnTime = await this.redisClient.getAsync(`${this.redisConfig.keys.vehicleSOTimeKey}:${vehicleId}`);

            return { pid: process.pid, vehicleId, signOnTime, tripInfo, fleetInfo };
        } catch (err) {
            throw new HttpException(
                {
                    message: "Could not process request",
                },
                HttpStatus.INTERNAL_SERVER_ERROR,
            );
        }
    }

    @Get("/god-mode/set-trip/:vehicleId/:tripId")
    public setVehicleToTrip(@Request() req: ExpressRequest): ApiResponse {
        if (this.godMode && this.appConfig.enableGodMode) {
            this.godMode.setVehicleToTrip(req.params.vehicleId, req.params.tripId);
            return { message: "Ok", process: process.pid };
        }

        throw new HttpException(
            {
                message: "Ops...",
            },
            HttpStatus.NOT_FOUND,
        );
    }

    @Get("/god-mode/reset/:vehicleId")
    public resetVehicle(@Request() req: ExpressRequest): ApiResponse {
        if (this.godMode && this.appConfig.enableGodMode) {
            if (req.params.vehicleId === "all") {
                this.godMode.resetAll();
            } else {
                this.godMode.resetVehicle(req.params.vehicleId);
            }
            return { message: "Ok", process: process.pid };
        }

        throw new HttpException(
            {
                message: "Ops...",
            },
            HttpStatus.NOT_FOUND,
        );
    }
}
