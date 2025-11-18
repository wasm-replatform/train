import * as _ from "lodash";
import * as AsyncLock from "async-lock";
import * as nr from "newrelic";
import { Redis } from "at-realtime-common/redis";
import { DilaxEvent, VehicleTripInfo } from "./types/vehicle-trip-info";
import { Config } from "./config";
import BlockMgtClientApi from "./services/block-mgt-client-api";
import FleetApi from "./services/fleet-api";
import GtfsStaticApi from "./services/gtfs-static-api";
import CcStaticApi from "./services/cc-static-api";
import { OccupancyStatus } from "./types/occupancy";
import STOP_TYPE from "./types/stop-types";

export interface DilaxState {
    count: number;
    token: number;
    lastTripId: string | null;
    occupancyStatus: string | null;
}
export default class DilaxProcessor {
    private static REDIS_TTL = 3 * 30 * 60;
    private log = Config.logger;
    private lock = new AsyncLock({ maxPending: 30000 });

    constructor(
        private redisClient: Redis,
        private fleetApi: FleetApi,
        private ccStaticApi: CcStaticApi,
        private gtfsStaticApi: GtfsStaticApi,
        private blockMgtClientApi: BlockMgtClientApi,
    ) {}

    private updateRunningCount(dilaxEvent: DilaxEvent, dilaxState: DilaxState, vehicleId: string, skipOut: boolean): void {
        let inTotal = 0;
        let outTotal = 0;
        let outTotalNoSkip = 0;
        let currentCount = 0;
        dilaxEvent.doors.forEach((door: any) => {
            inTotal += door.in;
            if (!skipOut) {
                outTotal += door.out;
            }
            outTotalNoSkip += door.out;
        });
        this.log.info(`vehicleId [${vehicleId}] Running count: inTotal [${inTotal}] outTotal [${outTotal}] outTotalNoSkip [${outTotalNoSkip}] skipOut [${skipOut}]`);
        const previousCount = dilaxState.count;
        currentCount = Math.max(previousCount - outTotal, 0) + inTotal;
        if (currentCount < 0) {
            this.log.warn(`vehicleId [${vehicleId}] has -ve currentCount [${currentCount}]`);
        }
        dilaxState.count = currentCount;
        this.log.info(`vehicleId [${vehicleId}] dilaxState.count [${dilaxState.count}]`);
    }

    private updateOccupancyStatus(dilaxState: DilaxState, vehicleId: string, vehicleSeatingSpace: number, vehicleTotalSpace: number): void {
        let occupancyStatus: OccupancyStatus | null = null;
        if (dilaxState.count < Math.trunc(vehicleSeatingSpace * 0.05)) {
            occupancyStatus = OccupancyStatus.EMPTY;
        } else if (dilaxState.count < Math.trunc(vehicleSeatingSpace * 0.4)) {
            occupancyStatus = OccupancyStatus.MANY_SEATS_AVAILABLE;
        } else if (dilaxState.count < Math.trunc(vehicleSeatingSpace * 0.9)) {
            occupancyStatus = OccupancyStatus.FEW_SEATS_AVAILABLE;
        } else if (dilaxState.count < Math.trunc(vehicleTotalSpace * 0.9)) {
            occupancyStatus = OccupancyStatus.STANDING_ROOM_ONLY;
        } else {
            occupancyStatus = OccupancyStatus.FULL;
        }
        this.log.info(`vehicleId [${vehicleId}] occupancyStatus [${occupancyStatus}]`);
        dilaxState.occupancyStatus = occupancyStatus.toString();
    }

    private isTrainStation(trainStops: any, stopCode: any): boolean {
        const isFound = _.find(trainStops, (stopType) => stopType.parent_stop_code === stopCode);
        if (isFound) {
            return isFound.route_type === STOP_TYPE.TRAINSTOP;
        }
        return false;
    }

    private getVehicleLabel(dilaxEvent: any): string | null {
        // This should be revisited when supporting all vehicle models.
        const vehicleLabelMap = {
            AM: "AMP",
            AD: "ADL",
        };
        const site = _.get(dilaxEvent, "device.site");
        if (!site) {
            return null;
        }

        const labelParts: string[] = site.match(/\D+|\d+/g);
        if (!labelParts.length) {
            return null;
        }

        const [alpha, ...rest] = labelParts;
        const num = rest.join("");
        const model = _.get(vehicleLabelMap, alpha, alpha);

        return `${model}${_.repeat(" ", Math.max(0, 14 - model.length - num.length))}${num}`;
    }

    public async getTrainStopId(vehicleId: any, dilaxEvent: DilaxEvent): Promise<string | null> {
        if (!dilaxEvent.wpt) {
            this.log.warn(`vehicleId [${vehicleId}] No lon and lat in event, not finding stopId`);
        } else {
            const { lon, lat } = dilaxEvent.wpt;
            this.log.info(`vehicleId [${vehicleId}] Stop coordinate: lat [${lat} lon ${lon}]`);
            const stopsInfo = await this.ccStaticApi.getStopsInfoByLongLat(lat, lon, 150);

            if (stopsInfo) {
                const trainStopTypes = await this.gtfsStaticApi.getTrainStopTypes();
                if (trainStopTypes.message) {
                    this.log.error(`vehicleId [${vehicleId}] Failed to get train stop types cannot determine if stop is a station [${trainStopTypes.message}]`);
                } else {
                    for (const stopInfo of stopsInfo) {
                        this.log.debug(`vehicleId [${vehicleId}] stopInfo [${JSON.stringify(stopInfo)}]`);
                        const foundTransStation = this.isTrainStation(trainStopTypes, stopInfo.stopCode);
                        if (foundTransStation) {
                            this.log.info(`vehicleId [${vehicleId}] Found a matching train stop: stopId [${stopInfo.stopId}] stopCode [${stopInfo.stopCode}]`);
                            return stopInfo.stopId;
                        }
                    }
                }
            }
        }
        return null;
    }

    public async process(dilaxEvent: DilaxEvent): Promise<any> {
        let tripId: null | string = null;
        let stopId: null | string = null;
        let vehicleId: null | string = null;
        let startDate: null | string = null;
        let startTime: null | string = null;
        let vehicleSeatingSpace = 0;
        let vehicleTotalSpace = 0;
        const vehicleLabel = this.getVehicleLabel(dilaxEvent);
        if (!vehicleLabel) {
            this.log.warn("Could not get a valid vehicle label from the dilax-adapter event, skipping...", dilaxEvent);
        } else {
            const vehicleInfo = await this.fleetApi.trainByLabel(vehicleLabel);
            if (!vehicleInfo) {
                this.log.warn(`Failed to get vehicleId from vehicleLabel [${vehicleLabel}]`);
            } else {
                this.log.info(`vehicleInfo [${JSON.stringify(vehicleInfo)}]`);
                vehicleId = vehicleInfo.id as string;
                this.log.info(`vehicleId [${vehicleId}] vehicleLabel [${vehicleLabel}]`);
                const capacity: { seating: number; standing: number; total: number } = vehicleInfo.capacity;
                if (!capacity) {
                    this.log.warn(`vehicleId [${vehicleId}] Could not get vehicle capcacity for vehicleLabel [${vehicleLabel}], skipping...`, dilaxEvent);
                    return;
                }
                vehicleSeatingSpace = capacity.seating;
                vehicleTotalSpace = capacity.total;
                this.log.info(`vehicleId [${vehicleId}] vehicleTotalSpace [${vehicleTotalSpace}] vehicleSeatingSpace [${vehicleSeatingSpace}]`);

                const vehicleAllocation = await this.blockMgtClientApi.getAllocationByVehicleId(vehicleId);
                if (!vehicleAllocation) {
                    this.log.warn(`vehicleId [${vehicleId}] Failed to find allocated trip`);
                } else {
                    this.log.info(`vehicleId [${vehicleId}] vehicleAllocation [${JSON.stringify(vehicleAllocation)}]`);
                    tripId = vehicleAllocation.tripId;
                    startDate = vehicleAllocation.serviceDate;
                    startTime = vehicleAllocation.startTime;
                }
            }
        }

        stopId = await this.getTrainStopId(vehicleId, dilaxEvent);
        if (!stopId) {
            this.log.warn(`vehicleId [${vehicleId}] Failed to find a stop with the dilax-adapter event`, dilaxEvent);
        }

        this.log.info(`vehicleId [${vehicleId}] tripId [${tripId}] stopId [${stopId}]`);

        const dilaxEventEnriched = { ...dilaxEvent, stop_id: stopId, trip_id: tripId, start_date: startDate, start_time: startTime };

        if (!vehicleId) {
            this.log.warn(`vehicleId [${vehicleId}] Failed to find a vehicleId to process passenger count. skipping...`);
            return dilaxEventEnriched;
        }

        await this.lock.acquire(`vehicleId-${vehicleId}`, async () => {
            try {
                const dilaxStateKey = `${Config.redis.apcVehicleIdStateKey}:${vehicleId}`;
                const dilaxStatePrevStr = await this.redisClient.getAsync(dilaxStateKey);
                let dilaxState: DilaxState;
                if (!dilaxStatePrevStr) {
                    dilaxState = {
                        count: 0,
                        token: 0,
                        lastTripId: null,
                        occupancyStatus: null,
                    };
                    // backward-compatibilty, to be removed in next prod version
                    const keysMigrated = await this.redisClient.getAsync(`${Config.redis.apcVehicleIdMigratedKey}:${vehicleId}`);
                    if (!keysMigrated) {
                        const tripIdOldVersion = await this.redisClient.getAsync(`${Config.redis.apcVehicleTripKey}:${vehicleId}`);
                        const countOldVersion = await this.redisClient.getAsync(`${Config.redis.apcVehicleIdKey}:${vehicleId}`);
                        if (tripIdOldVersion) {
                            this.log.warn(`vehicleId [${vehicleId}] migrating previous version tripIdOldVersion [${tripIdOldVersion}]`);
                            dilaxState.lastTripId = tripIdOldVersion;
                        }
                        if (countOldVersion) {
                            this.log.warn(`vehicleId [${vehicleId}] migrating previous version countOldVersion [${countOldVersion}]`);
                            dilaxState.count = parseInt(countOldVersion, 10);
                        }
                        await this.redisClient.setAsync(`${Config.redis.apcVehicleIdMigratedKey}:${vehicleId}`, "true");
                    }
                } else {
                    dilaxState = JSON.parse(dilaxStatePrevStr);
                }
                this.log.info(`vehicleId [${vehicleId}] dilaxState [${JSON.stringify(dilaxState)}]`);
                const token = parseInt(dilaxEvent.clock.utc, 10);
                if (token <= dilaxState.token) {
                    this.log.warn(`vehicleId [${vehicleId}] token [${token}] < dilaxState.token [${dilaxState.token}], skipping dup msg`);
                } else {
                    dilaxState.token = token;
                    // if found tripId, we want to track changes to it
                    let hasTripIdChanged = false;
                    if (tripId) {
                        const lastTripId = dilaxState.lastTripId;
                        if (!lastTripId) {
                            // if lastTripId is null eg on initial release or redis getting cleared externally, we want to prevent a reset count
                            dilaxState.lastTripId = tripId;
                        } else {
                            if (lastTripId !== tripId) {
                                hasTripIdChanged = true;
                                dilaxState.lastTripId = tripId;
                            }
                        }
                        this.log.info(`vehicleId [${vehicleId}] lastTripId [${lastTripId}] tripId [${tripId}] hasTripIdChanged [${hasTripIdChanged}]`);
                    }
                    // rest trip count if we cannot find tripId or when tripId has changed
                    if (!tripId || hasTripIdChanged) {
                        dilaxState.count = 0;
                        this.log.warn(`vehicleId [${vehicleId}] Reset running count for tripId [${tripId}]`);
                        // ATR-2379 we keep the IN and ignore the OUT on trip id change
                        this.updateRunningCount(dilaxEvent, dilaxState, vehicleId as string, true);
                    } else {
                        this.updateRunningCount(dilaxEvent, dilaxState, vehicleId as string, false);
                    }

                    this.updateOccupancyStatus(dilaxState, vehicleId as string, vehicleSeatingSpace, vehicleTotalSpace);

                    const lastValue = await this.redisClient.getsetAsync(dilaxStateKey, JSON.stringify(dilaxState));
                    await this.redisClient.expireAsync(dilaxStateKey, Config.APC_TTL_SECS);
                    if (lastValue !== dilaxStatePrevStr) {
                        const dirtyWriteMsg = `vehicleId [${vehicleId}] overwritten by another process \
                            during this update! dilaxStatePrevStr [${dilaxStatePrevStr}] lastValue [${lastValue}] dilaxState [${JSON.stringify(dilaxState)}]`;
                        this.log.warn(`${dirtyWriteMsg}`);
                    }
                    // backward-compatibilty, to be removed when dependent repo (CC UI) has migrated to read from state key
                    try {
                        if (dilaxState.occupancyStatus) {
                            await this.redisClient.setexAsync(`${Config.redis.keyOccupancy}:${vehicleId}`, DilaxProcessor.REDIS_TTL, dilaxState.occupancyStatus);
                        }
                    } catch (err) {
                        nr.noticeError(err);
                        this.log.error(`vehicleId [${vehicleId}] Failed to cache the occupancy status: ${err}`);
                    }
                    // backward-compatibilty, to be removed when dependent repo (smartrak_gtfs_adapter) has migrated to read from state key
                    try {
                        await this.redisClient.setexAsync(`${Config.redis.apcVehicleIdKey}:${vehicleId}`, Config.APC_TTL_SECS, dilaxState.count.toString());
                    } catch (err) {
                        nr.noticeError(err);
                        this.log.error(`vehicleId [${vehicleId}] Failed to cache the occupancy count: ${err}`);
                    }
                }
            } catch (err) {
                nr.noticeError(err);
                this.log.error(`vehicleId [${vehicleId}] Failed to process dilax-adapter apc count/state: ${err}`);
            }

            await this.saveVehicleTripInfo(vehicleId, vehicleLabel, tripId, stopId, dilaxEvent);
        });

        return dilaxEventEnriched;
    }

    private async saveVehicleTripInfo(
        vehicleId: string | null,
        vehicleLabel: string | null | undefined,
        tripId: string | null,
        stopId: string | null | undefined,
        dilaxEvent: DilaxEvent,
    ) {
        if (!vehicleId) {
            return;
        }
        try {
            const vehicleTripInfo: VehicleTripInfo = {
                vehicleInfo: {
                    vehicleId,
                    label: vehicleLabel,
                },
                tripId,
                stopId,
                dilaxMessage: dilaxEvent,
                lastReceivedTimestamp: dilaxEvent.clock.utc,
            };
            // Cache for 2 days.
            await this.redisClient.setexAsync(`${Config.redis.keyVehicleTripInfo}:${vehicleId}`, 2 * 24 * 60 * 60, JSON.stringify(vehicleTripInfo));
        } catch (err) {
            nr.noticeError(err);
            this.log.error(`Failed to save VehicleTripInfo for ${vehicleId}:${tripId}: ${err.stack || err.message}`);
        }
    }
}
