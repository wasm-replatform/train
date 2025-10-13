import * as nr from "newrelic";

import { Injectable } from "@nestjs/common";
import { AuthTokenRetriever } from "at-realtime-common/auth";
import { HttpClient } from "at-realtime-common/http-client";
import { Logger } from "at-realtime-common/logger";

import { AppConfig } from "../config/app";
import { RedisConfig } from "../config/redis";
import { CacheRepository } from "../repositories/cache";

export interface VehicleInfo {
    id: string;
    label: string | undefined;
    registration: string | undefined;
    capacity: { seating?: number; standing?: number; total?: number };
    type: { type?: string };
    tag: string;
}

@Injectable()
export class FleetApiService {
    private endpoint: string;

    constructor(
        private httpClient: HttpClient,
        private cacheRepository: CacheRepository,
        private appConfig: AppConfig,
        private authTokenRetriever: AuthTokenRetriever,
        private logger: Logger,
        private redisConfig: RedisConfig,
    ) {
        this.endpoint = this.appConfig.fleetApiUrl;
    }

    public async getVehicleInformationByLabel(label: string): Promise<VehicleInfo | undefined> {
        const redisKey = `${this.redisConfig.keys.fleetKey}:label:${label}`;
        const requestUrl = `${this.endpoint}/vehicles?label=${label}`;
        return this.fetch(requestUrl, redisKey);
    }

    public async getVehicleInformationById(vehicleId: string): Promise<VehicleInfo | undefined> {
        const redisKey = `${this.redisConfig.keys.fleetKey}:vehicleId:${vehicleId}`;
        const requestUrl = `${this.endpoint}/vehicles?id=${vehicleId}`;
        return this.fetch(requestUrl, redisKey);
    }

    public async getVehicleCapacityBasedOnRoute(vehicleId: string, routeId: string): Promise<{ seating?: number; standing?: number; total?: number } | undefined> {
        const redisKey = `${this.redisConfig.keys.fleetKey}:capacityBasedOnRouteId:${vehicleId}:${routeId}`;
        const requestUrl = `${this.endpoint}/vehicles?id=${vehicleId}&route_id=${routeId}`;
        const vehicleInfo = await this.fetch(requestUrl, redisKey);

        if (vehicleInfo && vehicleInfo.capacity) {
            return vehicleInfo.capacity;
        }
    }

    private async fetch(url: string, redisKey: string): Promise<VehicleInfo | undefined> {
        let response = await this.cacheRepository
            .getAsync(redisKey)
            .then((data: string | undefined) => (data ? JSON.parse(data) : undefined))
            .catch((err: Error) => {
                this.logger.error(`Could not parse data from '${redisKey}': ${err.stack || err.message}`);
                return undefined;
            });

        if (!response) {
            response = await this.httpClient
                .get(url, { retry: true })
                .then(async (res) => {
                    if (!res.data?.length) {
                        res.data = [{}];
                    }

                    await this.cacheRepository.setexAsync(redisKey, 10 * 60, JSON.stringify(res.data[0]));
                    return res.data[0];
                })
                .catch(async (err: Error) => {
                    nr.noticeError(err);
                    this.logger.error(`Fleet API '${url}': ${err.stack || err.message}`);
                    await this.cacheRepository.setexAsync(redisKey, 60, "{}");
                });
        }

        return Object.keys(response || {}).length ? response : undefined;
    }

    public async getVehicleInfoByIdOrLabel(vehicleIdOrLabel: string): Promise<VehicleInfo | undefined> {
        let vehicleInfo: VehicleInfo | undefined;
        // Some magic to get vehicle information
        // For buses, externalId === RAPID (AT) ID. For trains (AMnnn), it's more complex.
        if (/^[A-Z]+\d+$/.test(vehicleIdOrLabel)) {
            /**
             * The Fleet API has the train label formatted the Kiwirail way, where the space between the alpha and
             * numeric parts is padded with spaces so the final string is exactly 14 characters. For example:
             * Smartrak 'AM484' becomes 'AMP        484'.
             */
            const index = vehicleIdOrLabel.search(/\d/);
            let alpha = vehicleIdOrLabel.substr(0, index);
            // This is dubious but it works - Smartrak only knows the name of the EMU pair, but GTFS only knows about the power car
            alpha = alpha === "AM" ? "AMP" : alpha;
            const num = vehicleIdOrLabel.substr(index);
            let vehicleId = alpha;
            for (let i = 0; i < 14 - (alpha.length + num.length); i++) {
                vehicleId += " ";
            }

            vehicleInfo = await this.getVehicleInformationByLabel(vehicleId + num);
        } else {
            if (this.isTrain(vehicleIdOrLabel)) {
                // handle non smartrak trains
                vehicleInfo = await this.getVehicleInformationByLabel(vehicleIdOrLabel);
            } else {
                vehicleInfo = await this.getVehicleInformationById(vehicleIdOrLabel);
            }
        }
        return vehicleInfo;
    }

    private isTrain(str: string): boolean {
        // FIXME: this is wrong way to treat trains
        return str.length === 14 && str.includes("  ");
    }
}
