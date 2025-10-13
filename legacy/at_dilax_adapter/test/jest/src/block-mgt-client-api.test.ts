import { AuthTokenRetriever } from "at-realtime-common/auth";
import { HttpClient } from "at-realtime-common/http-client";
import BlockMgtClientAPI from "../../../src/services/block-mgt-client-api";

describe("Block Management API", () => {
    const mockData = {
        all: [
            {
                operationalBlockId: "501",
                tripId: "247-810163-18720-2-7101501-e8549b1d",
                serviceDate: "20230712",
                startTime: "05:12:00",
                vehicleId: "50806",
                vehicleLabel: "ADL        806",
                routeId: "WEST-201",
                directionId: null,
                referenceId: "7101",
                endTime: "05:59:00",
                delay: 0,
                startDatetime: 1689095520,
                endDatetime: 1689098340,
                isCanceled: false,
                isCompleted: false,
                isCopied: false,
                timezone: "Pacific/Auckland",
                creationDatetime: "2023-07-12T15:07:06.781+12:00",
            },
        ],
        current: [],
        next: [],
    };

    const mockAzureTokenRetriever = {
        getToken: async () => "token",
    } as unknown as AuthTokenRetriever;

    const mockHttpClient = {
        get: async () => ({ data: mockData }),
    } as unknown as HttpClient;

    const blockMgtApi = new BlockMgtClientAPI(mockAzureTokenRetriever, mockHttpClient);

    test("Get all block allocations", async () => {
        const result = await blockMgtApi.getAllAllocations();
        expect(result).toStrictEqual(mockData.all);
    });

    test("Get vehicle block allocations", async () => {
        const result = await blockMgtApi.getAllocationByVehicleId("NB5202");
        expect(result).toStrictEqual(undefined);
    });
});
