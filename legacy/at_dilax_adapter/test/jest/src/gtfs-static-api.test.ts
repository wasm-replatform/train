import { HttpClient } from "at-realtime-common/http-client";
import GtfsStaticApi from "../../../src/services/gtfs-static-api";

import NodeCache = require("node-cache");

describe("Gtfs Static API", () => {
    const mockData = [
        {
            stop_code: "6567",
            route_type: 2,
            parent_stop_code: null,
        },
        {
            stop_code: "6593",
            route_type: 3,
            parent_stop_code: null,
        },
    ];

    const mockCache = {
        get: () => undefined,
        set: () => undefined,
    } as unknown as NodeCache;

    const mockHttpClient = {
        get: async () => ({ data: mockData }),
    } as unknown as HttpClient;

    const gtfsStaticApi = new GtfsStaticApi(mockCache, mockHttpClient);

    test("Should get stopTypes", async () => {
        const result = await gtfsStaticApi.getStopTypes();
        expect(result).toStrictEqual(mockData);
    });

    test("Should get stopTypes", async () => {
        const result = await gtfsStaticApi.getTrainStopTypes();
        expect(result).toStrictEqual([mockData[0]]);
    });
});
