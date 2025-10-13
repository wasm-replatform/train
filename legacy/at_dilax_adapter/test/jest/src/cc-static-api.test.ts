import { HttpClient } from "at-realtime-common/http-client";
import CcStaticApi from "../../../src/services/cc-static-api";

const mockStopInfo = [
    {
        stop_id: "100-56c57897",
        feed_id: 1,
        stop_code: "100",
        platform_code: null,
        stop_name: "Papatoetoe Train Station",
        stop_desc: null,
        stop_lat: -36.97766,
        stop_lon: 174.84925,
        zone_id: null,
        stop_url: null,
        location_type: 1,
        parent_station: null,
        stop_timezone: null,
        wheelchair_boarding: 0,
        start_date: "20220722",
        end_date: "20220913",
    },
    {
        stop_id: "9214-6ac64c01",
        feed_id: 1,
        stop_code: "9214",
        platform_code: "1",
        stop_name: "Papatoetoe Train Station 1",
        stop_desc: null,
        stop_lat: -36.97766,
        stop_lon: 174.84925,
        zone_id: null,
        stop_url: null,
        location_type: 0,
        parent_station: "100-56c57897",
        stop_timezone: null,
        wheelchair_boarding: 0,
        start_date: "20220722",
        end_date: "20220913",
    },
];

jest.mock("newrelic", () => {
    return {
        noticeError: jest.fn(),
    };
});

describe("CC Static API", () => {
    const httpClient = { get: async () => ({ data: mockStopInfo }) } as unknown as HttpClient;

    beforeEach(() => {
        jest.clearAllMocks();
    });

    test("Should find the stop ids for a vehicle based on GPS coordinates", async () => {
        const lat = "-36.97766";
        const lng = "174.84925";
        const distance = 150;
        const ccStaticApi = new CcStaticApi(httpClient);

        const result = await ccStaticApi.getStopsInfoByLongLat(lat, lng, distance);

        expect(result.length).toEqual(2);
        expect(result[0].stopCode).toEqual(mockStopInfo[0].stop_code);
        expect(result[0].stopId).toEqual(mockStopInfo[0].stop_id);
        expect(result[1].stopCode).toEqual(mockStopInfo[1].stop_code);
        expect(result[1].stopId).toEqual(mockStopInfo[1].stop_id);
    });
});
