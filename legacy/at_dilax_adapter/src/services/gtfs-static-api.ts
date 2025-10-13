import { Config } from "../config";
import STOP_TYPE from "../types/stop-types";
import * as Cache from "node-cache";
import * as newrelic from "newrelic";
import { HttpClient } from "at-realtime-common/http-client";

/**
 * Interface to request information from GTFS Static API.
 */
export default class GtfsStaticApi {
    private log = Config.logger;

    constructor(private gtfsCache: Cache, private httpClient: HttpClient) {}

    public async getStopTypes(): Promise<any> {
        const value = <string[] | null>this.gtfsCache.get("stops");
        if (value) {
            return value;
        }

        return await this.httpClient
            .get(`${Config.gtfsStaticApiUrl}/stopstypes/`)
            .then((body: any) => {
                const stops = body?.data || [];
                // Cache failure for a minute, success for a day
                this.gtfsCache.set("stops", stops, body?.data ? 60 * 60 * 24 : 60);
                return stops;
            })
            .catch((err) => {
                newrelic.noticeError(err);
                this.log.error(`Failed to get stop types: ${err.message}`);
                return err;
            });
    }

    public async getTrainStopTypes(): Promise<any> {
        const value = <string[] | null>this.gtfsCache.get("trainStops");
        if (value) {
            return value;
        }
        const stopTypes = await this.getStopTypes();
        const trainStopTypes = stopTypes.filter((type: any) => type.route_type === STOP_TYPE.TRAINSTOP);
        this.gtfsCache.set("trainStops", trainStopTypes, 60 * 60 * 24);
        return trainStopTypes;
    }
}
