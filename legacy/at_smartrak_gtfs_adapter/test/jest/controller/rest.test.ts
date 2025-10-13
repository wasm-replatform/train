import { Test } from "@nestjs/testing";
import { HttpException, Injectable } from "@nestjs/common";
import * as moment from "moment-timezone";
import { Request as ExpressRequest } from "express";

import { Redis, RedisModule } from "at-realtime-common/redis";
import { Logger, LoggerModule } from "at-realtime-common/logger";

import { AppConfig } from "../../../src/config/app";
import { LoggerConfig } from "../../../src/config/logger";
import { RedisConfig } from "../../../src/config/redis";
import { RestApiController } from "../../../src/controller/rest";
import { GodMode } from "../../../src/processors/god-mode";

describe("RestApiController", () => {
    let restApiController: RestApiController;
    let godMode: GodMode;
    let redisClient: Redis;
    let redisConfig: RedisConfig;
    let appConfig: AppConfig;

    @Injectable()
    class MockRedisConfig extends RedisConfig {
        public mock = true;
    }

    beforeEach(async () => {
        const moduleRef = await Test.createTestingModule({
            imports: [LoggerModule.register(AppConfig, LoggerConfig), RedisModule.register(Logger, MockRedisConfig)],
            controllers: [RestApiController],
            providers: [AppConfig, RedisConfig, GodMode],
        })
            .overrideProvider(RedisConfig)
            .useClass(MockRedisConfig)
            .compile();

        appConfig = moduleRef.get<AppConfig>(AppConfig);
        redisClient = moduleRef.get<Redis>(Redis);
        redisConfig = moduleRef.get<RedisConfig>(RedisConfig);
        godMode = moduleRef.get<GodMode>(GodMode);
        restApiController = moduleRef.get<RestApiController>(RestApiController);
        redisClient.start();
    });

    afterEach(async () => {
        (
            redisClient as unknown as {
                redis: {
                    flushall: () => {
                        /** noop */
                    };
                };
            }
        ).redis.flushall();
    });

    it("should return OK from default route", async () => {
        const mockRequest = {
            headers: {
                "user-agent": "Local-Test-Browser",
            },
        } as ExpressRequest;
        const result = restApiController.index(mockRequest);
        expect(result).toEqual("OK");
    });

    it("should return a vehicle info if exists", async () => {
        const now = JSON.stringify(moment.utc().unix());
        const vehicleId = "12345";
        redisClient.setAsync(`${redisConfig.keys.tripKey}:${vehicleId}`, JSON.stringify({ tripId: "abcdefg" }));
        redisClient.setAsync(`${redisConfig.keys.fleetKey}:${vehicleId}`, JSON.stringify({ label: "ABC123" }));
        redisClient.setAsync(`${redisConfig.keys.vehicleSOTimeKey}:${vehicleId}`, now);

        const result = await restApiController.getVehicleInfoById(vehicleId);
        expect(result.tripInfo?.tripId).toEqual("abcdefg");
        expect(result.fleetInfo?.label).toEqual("ABC123");
        expect(result.signOnTime).toEqual(now);
        expect(result.vehicleId).toEqual("12345");
    });

    it("should return Ok from setVehicleToTrip when god mode is on", async () => {
        appConfig.enableGodMode = true;
        const mockRequest = {
            params: {
                vehicleId: "12345",
                tripId: "abcdefg",
            },
        } as unknown as ExpressRequest;
        const result = await restApiController.setVehicleToTrip(mockRequest);
        expect(result.message).toEqual("Ok");
    });

    it("should throw 404 error from setVehicleToTrip when god mode is off", async () => {
        appConfig.enableGodMode = false;
        const mockRequest = {
            params: {
                vehicleId: "12345",
                tripId: "abcdefg",
            },
        } as unknown as ExpressRequest;
        expect(async () => await restApiController.setVehicleToTrip(mockRequest)).rejects.toThrow(HttpException);
    });

    it("should reset all in resetVehicle when vehicleId is all", async () => {
        appConfig.enableGodMode = true;
        const mockRequest = {
            params: {
                vehicleId: "all",
            },
        } as unknown as ExpressRequest;
        const fn = jest.spyOn(godMode, "resetAll");
        const result = await restApiController.resetVehicle(mockRequest);
        expect(fn).toBeCalled();
        expect(result.message).toEqual("Ok");
    });

    it("should reset one in resetVehicle when vehicleId is not all", async () => {
        appConfig.enableGodMode = true;
        const mockRequest = {
            params: {
                vehicleId: "12345",
            },
        } as unknown as ExpressRequest;
        const fn = jest.spyOn(godMode, "resetVehicle");
        const result = await restApiController.resetVehicle(mockRequest);
        expect(fn).toBeCalled();
        expect(result.message).toEqual("Ok");
    });

    it("should throw 404 error from resetVehicle when god mode is off", async () => {
        appConfig.enableGodMode = false;
        const mockRequest = {
            params: {
                vehicleId: "all",
            },
        } as unknown as ExpressRequest;
        expect(async () => await restApiController.resetVehicle(mockRequest)).rejects.toThrow(HttpException);
    });
});
