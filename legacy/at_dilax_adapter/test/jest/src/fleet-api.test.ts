import { HttpClient } from "at-realtime-common/http-client";
import { Redis } from "at-realtime-common/redis";
import FleetApi from "../../../src/services/fleet-api";

jest.mock("newrelic", () => ({
    noticeError: jest.fn(),
}));

describe("Fleet API", () => {
    const mockData = [
        {
            id: "50801",
            label: "ADL        801",
            agency: {
                agencyId: "AM",
                agencyName: "AT Metro",
                depot: {
                    name: "21",
                },
            },
            attributes: {
                loweringFloor: true,
            },
            capacity: {
                total: 373,
                seating: 232,
                standing: 141,
            },
            type: {
                type: "Train",
            },
            eod: {
                generated: {},
                activated: {},
                beId: 201,
                vehicleId: "801",
            },
            vehicle: "50801",
        },
    ];

    const mockRedis = {
        // cache: "",
        getAsync: async function (value: string) {
            if (value.includes("TRAIN")) {
                return "";
            }
            return undefined;
        },
        setexAsync: async function (key: string, timeout: number, value: string) {
            value = "";
        },
    } as unknown as Redis;

    const mockHttpClient = {
        get: async (label: string) => {
            if (label.includes("TRAIN")) {
                return { data: mockData };
            }
            throw new Error("no data");
        },
    } as unknown as HttpClient;

    const fleetApi = new FleetApi(mockRedis, mockHttpClient);

    test("Should get fleet list from httpClient", async () => {
        const result = await fleetApi.trainByLabel("TRAIN");
        expect(result).toStrictEqual(mockData[0]);
    });

    test("Should get fleet list from cache", async () => {
        const result = await fleetApi.trainByLabel("TRAIN");
        expect(result).toStrictEqual(mockData[0]);
    });

    test("Should handle no response", async () => {
        const result = await fleetApi.trainByLabel("OTHER");
        expect(result).toStrictEqual(null);
    });
});
