import * as moment from "moment-timezone";

import { AuthTokenRetriever } from "at-realtime-common/auth";
import { HttpClient } from "at-realtime-common/http-client";
import { Redis } from "at-realtime-common/redis";
import * as NodeCache from "node-cache";
import { Config } from "../../../src/config";
import { DilaxLostConnectionsDetector } from "../../../src/dilax-adapter-lost-connections-detector";
import BlockMgtClientAPI from "../../../src/services/block-mgt-client-api";
import { VehicleAllocation } from "../../../src/types/vehicle-allocation";

class RedisHelperStub extends Redis {
    private entries: Map<string, string> = new Map<string, string>();

    public async getAsync(key: string): Promise<string | null> {
        return this.entries.get(key) || null;
    }

    public async smembersAsync(key: string): Promise<string[]> {
        return [];
    }

    public async saddAsync(key: string, value: string[]): Promise<number> {
        value.forEach((entry) => this.entries.set(key, entry));
        return this.entries.size;
    }

    public setValue(key: string, value: string) {
        this.entries.set(key, value);
    }

    public reset() {
        this.entries.clear();
    }
}

class BlockMgtClientApiStub extends BlockMgtClientAPI {
    private allocations: VehicleAllocation[] = [];

    public async getAllAllocations(): Promise<VehicleAllocation[]> {
        return this.allocations;
    }

    public mockAllocations(allocations: VehicleAllocation[]) {
        this.allocations = allocations;
    }

    public reset() {
        this.allocations = [];
    }
}

const createAllocation = (vehicleId: string, startTimeUnix: number, endTimeUnix: number): VehicleAllocation => ({
    operationalBlockId: "101",
    tripId: "246-850029-18540-2-M200101-cd17a4ce",
    serviceDate: moment.tz(Config.timezone).format("YYYYMMDD"),
    startTime: "05:09:00",
    vehicleId: vehicleId,
    vehicleLabel: "AMP        390",
    routeId: "EAST-201",
    directionId: null,
    referenceId: "M200",
    endTime: "06:00:00",
    delay: 0,
    startDatetime: startTimeUnix,
    endDatetime: endTimeUnix,
    isCanceled: false,
    isCopied: false,
    timezone: "Pacific/Auckland",
    creationDatetime: "2020-08-19T08:26:06.005+12:00",
});

const createVehicleTripInfo = (tripId: string, vehicleId: string, vehicleLabel: string, dilaxMessageTimestamp: number) => ({
    tripId,
    vehicleInfo: {
        vehicleId,
        label: vehicleLabel,
    },
    lastReceivedTimestamp: dilaxMessageTimestamp,
    dilaxMessage: {
        dlx_vers: "1.0",
        dlx_type: "td",
        driving: true,
        atstop: false,
        operational: true,
        distance_laststop: 2142,
        distance_start: 180644,
        departure_utc: "1597801883",
        arrival_utc: "1597801843",
        speed: 0,
        trigger: "station_left_summary",
        device: {
            operator: "AUCKLAND",
            site: "AM605",
            model: "CARM1",
            serial: "1437-03383",
        },
        wpt: { sat: "5", lat: "-36.908800", lon: "174.685600", speed: 0 },
        clock: { utc: dilaxMessageTimestamp, tz: "NZST-12NZDT,M9.5.0,M4.1.0/3" },
        pis: { line: "0", stop: "0" },
        doors: [
            { name: "M1D1", in: 0, out: 0, art: 0, st: "closed" },
            { name: "M1D2", in: 0, out: 0, art: 0, st: "open" },
            { name: "M1D3", in: 0, out: 0, art: 0, st: "closed" },
            { name: "M1D4", in: 0, out: 0, art: 0, st: "open" },
            { name: "TD1", in: 0, out: 0, art: 0, st: "closed" },
            { name: "TD2", in: 0, out: 0, art: 0, st: "open" },
            { name: "TD3", in: 0, out: 0, art: 0, st: "closed" },
            { name: "TD4", in: 0, out: 0, art: 0, st: "open" },
            { name: "M2D1", in: 0, out: 0, art: 0, st: "open" },
            { name: "M2D2", in: 0, out: 0, art: 0, st: "closed" },
            { name: "M2D3", in: 0, out: 0, art: 0, st: "open" },
            { name: "M2D4", in: 0, out: 0, art: 0, st: "closed" },
        ],
    },
});

describe("DilaxLostConnectionsDetector", () => {
    let dilaxLostConnectionsDetector: DilaxLostConnectionsDetector;
    const redisHelperMock = new RedisHelperStub(Config.logger, { newRelicPrefix: "" });
    const blockManagementClientApiMock = new BlockMgtClientApiStub({} as unknown as AuthTokenRetriever, {} as unknown as HttpClient);
    const sleep = (ms: number) => new Promise((rx) => setTimeout(rx, ms));

    beforeEach(() => {
        redisHelperMock.reset();
        blockManagementClientApiMock.reset();
    });

    afterEach(() => {
        dilaxLostConnectionsDetector.stop();
    });

    test("If no allocation is in the cache nothing is detected.", async () => {
        dilaxLostConnectionsDetector = new DilaxLostConnectionsDetector(redisHelperMock, blockManagementClientApiMock);

        const detected = await dilaxLostConnectionsDetector.detectCandidates();

        expect(detected.length).toEqual(0);
    });

    test("Start detecting lost connection", async () => {
        const allocation = createAllocation(
            "59390",
            moment()
                .subtract(Config.dilaxConnectionLostThreshold + 5, "minute")
                .unix(),
            moment().add(5, "minute").unix(),
        );
        blockManagementClientApiMock.mockAllocations([allocation]);
        dilaxLostConnectionsDetector = new DilaxLostConnectionsDetector(redisHelperMock, blockManagementClientApiMock);
        await dilaxLostConnectionsDetector.init();

        const spy = jest.spyOn(Config.logger, "warn");
        await dilaxLostConnectionsDetector.startDetectingLostConnections();
        await sleep(500);
        expect(spy).toBeCalledWith(expect.stringMatching("Dilax Connection Lost: Vehicle"));

        dilaxLostConnectionsDetector.stopDetectingLostConnections();
    });

    describe("No VehicleTripInfo is found", () => {
        test("In service vehicle where (start time + threshold) >= now is not reported as lost", async () => {
            const allocation = createAllocation("59390", moment().subtract(5, "minute").unix(), moment().add(5, "minute").unix());
            blockManagementClientApiMock.mockAllocations([allocation]);
            dilaxLostConnectionsDetector = new DilaxLostConnectionsDetector(redisHelperMock, blockManagementClientApiMock);
            await dilaxLostConnectionsDetector.init();

            const detected = await dilaxLostConnectionsDetector.detectCandidates();

            expect(detected.length).toEqual(0);
        });

        test("In service vehicle where (start time + threshold) < now is reported as lost", async () => {
            const allocation = createAllocation(
                "59390",
                moment()
                    .subtract(Config.dilaxConnectionLostThreshold + 5, "minute")
                    .unix(),
                moment().add(5, "minute").unix(),
            );
            blockManagementClientApiMock.mockAllocations([allocation]);
            dilaxLostConnectionsDetector = new DilaxLostConnectionsDetector(redisHelperMock, blockManagementClientApiMock);
            await dilaxLostConnectionsDetector.init();

            const detected = await dilaxLostConnectionsDetector.detectCandidates();

            expect(detected.length).toEqual(1);
            expect(detected[0].vehicleTripInfo.dilaxMessage).toBeUndefined();
            expect(detected[0].vehicleTripInfo.stopId).toBeUndefined();
            expect(detected[0].vehicleTripInfo.lastReceivedTimestamp).toBeUndefined();
            expect(detected[0].vehicleTripInfo.tripId).toEqual(allocation.tripId);
            expect(detected[0].vehicleTripInfo.vehicleInfo.vehicleId).toEqual(allocation.vehicleId);
            expect(detected[0].vehicleTripInfo.vehicleInfo.label).toEqual(allocation.vehicleLabel);
        });
    });

    describe("VehicleTripInfo is found", () => {
        test("Should fall back to start time comparison if VehicleTripInfo is for a different trip (start time is within threshold)", async () => {
            const allocation = createAllocation("59390", moment().subtract("minutes").unix(), moment().add(5, "minute").unix());
            const dilaxMessageTimestamp = moment()
                .subtract(Config.dilaxConnectionLostThreshold + 5, "minutes")
                .unix();
            const vehicleTripInfo = createVehicleTripInfo("A different trip ID", allocation.vehicleId, allocation.vehicleLabel, dilaxMessageTimestamp);
            redisHelperMock.setValue(`${Config.redis.keyVehicleTripInfo}:${vehicleTripInfo.vehicleInfo.vehicleId}`, JSON.stringify(vehicleTripInfo));
            blockManagementClientApiMock.mockAllocations([allocation]);
            dilaxLostConnectionsDetector = new DilaxLostConnectionsDetector(redisHelperMock, blockManagementClientApiMock);
            await dilaxLostConnectionsDetector.init();

            const detected = await dilaxLostConnectionsDetector.detectCandidates();

            expect(detected.length).toEqual(0);
        });

        test("Should fall back to start time comparison if VehicleTripInfo is for a different trip (start time is NOT within threshold)", async () => {
            const dilaxMessageTimestamp = moment()
                .subtract(Config.dilaxConnectionLostThreshold + 5, "minutes")
                .unix();
            const allocation = createAllocation("59390", dilaxMessageTimestamp, moment().add(5, "minute").unix());
            const vehicleTripInfo = createVehicleTripInfo("A different trip ID", allocation.vehicleId, allocation.vehicleLabel, dilaxMessageTimestamp);
            redisHelperMock.setValue(`${Config.redis.keyVehicleTripInfo}:${vehicleTripInfo.vehicleInfo.vehicleId}`, JSON.stringify(vehicleTripInfo));
            blockManagementClientApiMock.mockAllocations([allocation]);
            dilaxLostConnectionsDetector = new DilaxLostConnectionsDetector(redisHelperMock, blockManagementClientApiMock);
            await dilaxLostConnectionsDetector.init();

            const detected = await dilaxLostConnectionsDetector.detectCandidates();

            expect(detected.length).toEqual(1);
            expect(detected[0].vehicleTripInfo.dilaxMessage).not.toBeUndefined();
            expect(detected[0].vehicleTripInfo.stopId).toBeUndefined();
            expect(detected[0].vehicleTripInfo.lastReceivedTimestamp).not.toBeUndefined();
            expect(detected[0].vehicleTripInfo.tripId).not.toEqual(allocation.tripId);
            expect(detected[0].vehicleTripInfo.vehicleInfo.vehicleId).toEqual(allocation.vehicleId);
            expect(detected[0].vehicleTripInfo.vehicleInfo.label).toEqual(allocation.vehicleLabel);
        });

        test("Should detectCandidates as lost when tripIds are equal and (lastReceivedTimestamp + threshold) < now", async () => {
            const dilaxMessageTimestamp = moment()
                .subtract(Config.dilaxConnectionLostThreshold + 5, "minutes")
                .unix();
            const allocation = createAllocation("59390", dilaxMessageTimestamp, moment().add(5, "minute").unix());
            const vehicleTripInfo = createVehicleTripInfo(allocation.tripId, allocation.vehicleId, allocation.vehicleLabel, dilaxMessageTimestamp);
            redisHelperMock.setValue(`${Config.redis.keyVehicleTripInfo}:${allocation.vehicleId}`, JSON.stringify(vehicleTripInfo));
            blockManagementClientApiMock.mockAllocations([allocation]);
            dilaxLostConnectionsDetector = new DilaxLostConnectionsDetector(redisHelperMock, blockManagementClientApiMock);
            await dilaxLostConnectionsDetector.init();

            const detected = await dilaxLostConnectionsDetector.detectCandidates();

            expect(detected.length).toEqual(1);
            expect(detected[0].vehicleTripInfo.dilaxMessage).toEqual(vehicleTripInfo.dilaxMessage);
            expect(detected[0].vehicleTripInfo.stopId).toBeUndefined();
            expect(detected[0].vehicleTripInfo.lastReceivedTimestamp).toEqual(vehicleTripInfo.lastReceivedTimestamp);
            expect(detected[0].vehicleTripInfo.tripId).toEqual(allocation.tripId);
            expect(detected[0].vehicleTripInfo.vehicleInfo.vehicleId).toEqual(allocation.vehicleId);
            expect(detected[0].vehicleTripInfo.vehicleInfo.label).toEqual(allocation.vehicleLabel);
        });

        test("Should NOT detectCandidates as lost when tripIds are equal and (lastReceivedTimestamp + threshold) >= now", async () => {
            const allocation = createAllocation(
                "59390",
                moment()
                    .subtract(Config.dilaxConnectionLostThreshold + 5, "minute")
                    .unix(),
                moment().add(5, "minute").unix(),
            );
            redisHelperMock.setValue(
                `${Config.redis.keyVehicleTripInfo}:${allocation.vehicleId}`,
                JSON.stringify(createVehicleTripInfo(allocation.tripId, allocation.vehicleId, allocation.vehicleLabel, moment().subtract(5, "minutes").unix())),
            );
            blockManagementClientApiMock.mockAllocations([allocation]);
            dilaxLostConnectionsDetector = new DilaxLostConnectionsDetector(redisHelperMock, blockManagementClientApiMock);
            await dilaxLostConnectionsDetector.init();

            const detected = await dilaxLostConnectionsDetector.detectCandidates();

            expect(detected.length).toEqual(0);
        });
    });

    describe("Filter running vehicles", () => {
        test("Should filter out Diesel Trains (ADL)", async () => {
            const nodeCache = new NodeCache();
            const allocation = createAllocation(
                "59390",
                moment()
                    .subtract(Config.dilaxConnectionLostThreshold + 5, "minute")
                    .unix(),
                moment().add(5, "minute").unix(),
            );
            allocation.vehicleLabel = "ADL     810";
            blockManagementClientApiMock.mockAllocations([allocation]);
            dilaxLostConnectionsDetector = new DilaxLostConnectionsDetector(redisHelperMock, blockManagementClientApiMock, nodeCache);
            await dilaxLostConnectionsDetector.init();

            const cachedTrips = nodeCache.get<VehicleAllocation[]>(dilaxLostConnectionsDetector.allocationsCacheKey);

            expect(cachedTrips?.length).toEqual(0);
        });
    });
});
