import * as newrelic from "newrelic";
import { HttpClient } from "at-realtime-common/http-client";
import { Config } from "../config";

const stopByLongLat = "gtfs/stops/geosearch";

const headers = {
    Accept: "application/json; charset=utf-8",
    "Content-Type": "application/json; charset=utf-8",
};

/**
 * Interface to request information from CC Static API.
 */
export default class CcStaticApi {
    private log = Config.logger;

    constructor(private httpClient: HttpClient) {}

    /**
     * Find the stop ids from CC Static API for a vehicle based on GPS coordinates.
     */
    public async getStopsInfoByLongLat(lat: string, lng: string, distance: number): Promise<any> {
        let stopsInfo;
        try {
            const stops = await this.httpClient
                .get(`${Config.cc_static_api.uri}/${stopByLongLat}?lat=${lat}&lng=${lng}&distance=${distance}`, { headers })
                .then((response) => response.data);
            this.log.debug(`stops [${JSON.stringify(stops)}]`);
            if (stops && stops.length > 0) {
                if (stops.length > 1) {
                    const stopIds = stops.map((stop: any) => stop.stop_id).join(",");
                    const stopNames = stops.map((stop: any) => stop.stop_name).join(",");
                    this.log.debug(`Found [${stops.length}] stops [${stopIds}] stopNames [${stopNames}] ` + `lat lng [${lat} ${lng}] stops [${JSON.stringify(stops)}]`);
                }
                stopsInfo = stops.map((stop: any) => {
                    return {
                        stopId: stop.stop_id,
                        stopCode: stop.stop_code,
                    };
                });
            }
        } catch (err) {
            newrelic.noticeError(err);
            this.log.error(`Failed to get stop info by lat [${lat}] lng [${lng}]: ${err.message}`);
        }
        return stopsInfo;
    }
}
