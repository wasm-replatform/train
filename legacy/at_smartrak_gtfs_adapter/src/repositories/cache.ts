import * as NodeCache from "node-cache";
import { Injectable } from "@nestjs/common";

import { Redis } from "at-realtime-common/redis";
import { Logger } from "at-realtime-common/logger";

// we use node cache as short time cache
// which should reduce redis calls
@Injectable()
export class CacheRepository {
    private cache = new NodeCache({ stdTTL: 5, checkperiod: 5 });
    private nodeCacheTTL = 5;

    constructor(private redisClient: Redis, private logger: Logger) {}

    public async getAsync(key: string): Promise<string | undefined> {
        let value = this.cache.get(key);

        if (!value) {
            value = await this.redisClient.getAsync(key).catch((err: Error) => {
                this.logger.error(`Could not get '${key}': ${err.stack || err.message}`);
            });

            if (value && value !== "null") {
                // if we have actual value then short cache it for 5s
                this.cache.set(key, value, this.nodeCacheTTL);
            }
        }

        return value as string | undefined;
    }

    public async deleteAsync(key: string): Promise<void> {
        await this.redisClient.delAsync(key).catch((err: Error) => {
            this.logger.error(`Could not delete '${key}': ${err.stack || err.message}`);
        });
        this.cache.del(key);
    }

    public async setexAsync(key: string, ttl: number, value: string): Promise<void> {
        await this.redisClient.setexAsync(key, ttl, value).catch((err: Error) => this.logger.error(`Could not set '${key}': ${err.stack || err.message}`));
        this.cache.set(key, value, ttl < this.nodeCacheTTL ? ttl : this.nodeCacheTTL);
    }

    public async flushNodeCache() {
        this.cache.flushAll();
    }
}
