import * as _ from "lodash";
import * as newrelic from "newrelic";
import { AuthTokenRetriever } from "at-realtime-common/auth";
import { Config } from "../config";
import { HttpClient } from "at-realtime-common/http-client";
import { Allocation } from "../types/vehicle-allocation";

export default class BlockMgtClientAPI {
    private log = Config.logger;

    constructor(private tokenRetriever: AuthTokenRetriever, private httpClient: HttpClient) {}

    public async getAllocationByVehicleId(vehicleId: string) {
        try {
            const accessToken = await this.tokenRetriever.getToken();
            const response = await this.httpClient.get(`${Config.blockMgtClientApiUrl}/allocations/vehicles/${vehicleId}?currentTrip=true`, {
                headers: {
                    Authorization: `Bearer ${accessToken}`,
                },
            });
            return _.get(response.data, "current.[0]");
        } catch (err) {
            newrelic.noticeError(err);
            this.log.error(`Error occurred while fetching allocations by vehicle id [${vehicleId}]`, err.message);
            return null;
        }
    }

    public async getAllAllocations(): Promise<Allocation[]> {
        try {
            const accessToken = await this.tokenRetriever.getToken();
            const response = await this.httpClient.get(`${Config.blockMgtClientApiUrl}/allocations`, {
                headers: {
                    Authorization: `Bearer ${accessToken}`,
                },
            });
            return response.data.all;
        } catch (err) {
            newrelic.noticeError(err);
            this.log.error("Error occurred while fetching all allocations", err.stack || err.message);
            return [];
        }
    }
}
