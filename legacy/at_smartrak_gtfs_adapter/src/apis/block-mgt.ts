import * as nr from "newrelic";

import { Injectable } from "@nestjs/common";
import { AuthTokenRetriever } from "at-realtime-common/auth";
import { HttpClient } from "at-realtime-common/http-client";
import { Logger } from "at-realtime-common/logger";

import { AppConfig } from "../config/app";
import { RedisConfig } from "../config/redis";
import { CacheRepository } from "../repositories/cache";
import { Allocation } from "../model/vehicle-allocation";

enum BlockState {
    OK,
    ERROR,
}

export class BlockInstance {
    public readonly tripId: string;
    public readonly startTime: string;
    public readonly serviceDate: string;
    public readonly vehicleIds: string[];
    public readonly error: boolean;

    private state: BlockState;

    constructor(config: Partial<BlockInstance> = {}) {
        this.tripId = config.tripId || "";
        this.startTime = config.startTime || "";
        this.serviceDate = config.serviceDate || "";
        this.vehicleIds = config.vehicleIds || [];
        this.error = config.error || false;

        this.state = BlockState.OK;

        if (this.error) {
            this.state = BlockState.ERROR;
        }
    }

    public static from(config?: Partial<BlockInstance>): BlockInstance {
        return new BlockInstance(config);
    }

    public hasError(): boolean {
        return this.state === BlockState.ERROR;
    }

    // Javascript method which is called automatically during JSON stringify
    public toJSON(): Partial<BlockInstance> {
        return {
            tripId: this.tripId,
            startTime: this.startTime,
            serviceDate: this.serviceDate,
            vehicleIds: this.vehicleIds,
            error: this.error,
        };
    }
}

@Injectable()
export class BlockMgtApi {
    private vehicleAllocationUrl: string;

    constructor(
        private httpClient: HttpClient,
        private cacheRepository: CacheRepository,
        private appConfig: AppConfig,
        private authTokenRetriever: AuthTokenRetriever,
        private logger: Logger,
        private redisConfig: RedisConfig,
    ) {
        this.vehicleAllocationUrl = `${appConfig.blockManagementUrl}/allocations/vehicles`;
    }

    public async getAllocationByVehicleId(vehicleId: string, timestamp: number): Promise<BlockInstance | undefined> {
        const blockManagementKey = `${this.redisConfig.keys.blockManagementKey}:${vehicleId}`;

        let response = await this.cacheRepository
            .getAsync(blockManagementKey)
            .then((data: string | undefined) => (data ? BlockInstance.from(JSON.parse(data)) : undefined))
            .catch((err: Error) => {
                this.logger.error(`Could not parse blockInstance from '${blockManagementKey}': ${err.stack || err.message}`);
                return undefined;
            });

        if (!response) {
            const url = `${this.vehicleAllocationUrl}/${vehicleId}?currentTrip=true&siblings=true&nowUnixTimeSeconds=${timestamp}`;
            const headers = { Authorization: `Bearer ${await this.authTokenRetriever.getToken()}` };

            response = await this.httpClient
                .get(url, { headers, retry: true })
                .then(async (res) => {
                    let blockInstance: BlockInstance | undefined;

                    if (res.data?.current?.length) {
                        blockInstance = BlockInstance.from({
                            tripId: res.data.current[0].tripId,
                            startTime: res.data.current[0].startTime,
                            serviceDate: res.data.current[0].serviceDate,
                            vehicleIds: res.data.current.map((instance: Allocation) => instance.vehicleId),
                        });
                    }

                    await this.cacheRepository.setexAsync(blockManagementKey, 20, JSON.stringify(blockInstance || {}));
                    return blockInstance;
                })
                .catch(async (err: Error) => {
                    nr.noticeError(err);

                    const errorBlockInstance = BlockInstance.from({ error: true });
                    this.logger.error(`Calling Block Management API '${url}': ${err.stack || err.message}`);

                    // Cache fail for 10s
                    await this.cacheRepository.setexAsync(blockManagementKey, 10, JSON.stringify(errorBlockInstance));
                    return errorBlockInstance;
                });
        }

        return response && (response.tripId || response.hasError()) ? response : undefined;
    }
}
