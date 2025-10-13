import BlockMgtClientAPI from "./services/block-mgt-client-api";
import * as NodeCache from "node-cache";
import * as newrelic from "newrelic";
import * as moment from "moment-timezone";
import * as _ from "lodash";
import { Redis } from "at-realtime-common/redis";
import { Config } from "./config";
import { VehicleTripInfo } from "./types/vehicle-trip-info";
import { VehicleAllocation } from "./types/vehicle-allocation";

export class DilaxLostConnectionsDetector {
    private static readonly DIESEL_TRAIN_PREFIX = "ADL";

    private fetchAllocationsTimer: NodeJS.Timeout;
    private detectLostConnectionsTimer: NodeJS.Timeout;
    public readonly allocationsCacheKey = "allocations";

    constructor(private redisClient: Redis, private readonly blockManagementClientAPI: BlockMgtClientAPI, private readonly cache = new NodeCache()) {}

    public async init(): Promise<void> {
        await this.startFetchingAllocations();
    }

    public stop(): void {
        if (this.fetchAllocationsTimer) {
            clearTimeout(this.fetchAllocationsTimer);
        }
        if (this.detectLostConnectionsTimer) {
            clearTimeout(this.detectLostConnectionsTimer);
        }
    }

    private async startFetchingAllocations(): Promise<void> {
        try {
            const allAllocations = await this.blockManagementClientAPI.getAllAllocations();
            Config.logger.info(`Loaded ${allAllocations.length} allocations`);
            const serviceDate = moment.tz(Config.timezone).format("YYYYMMDD");
            const allocationsForToday = await Promise.all(
                allAllocations.filter(
                    (allocation) =>
                        allocation.serviceDate === serviceDate && allocation.vehicleId && !allocation.vehicleLabel.startsWith(DilaxLostConnectionsDetector.DIESEL_TRAIN_PREFIX),
                ),
            );
            Config.logger.info(`Caching ${allocationsForToday.length} allocations for today`);

            this.cache.set(this.allocationsCacheKey, allocationsForToday);
            newrelic.incrementMetric(`${Config.newRelicPrefix}/allocations-fetched`);
        } catch (error) {
            newrelic.noticeError(error);
            Config.logger.error(`Error fetching allocations: ${error.stack || error.message}`);
        } finally {
            this.fetchAllocationsTimer = setTimeout(this.startFetchingAllocations.bind(this), 60000);
        }
    }

    public async startDetectingLostConnections(): Promise<void> {
        try {
            const candidates = await this.detectCandidates();
            const triggeredDetectionsSet = `${Config.redis.lostConnectionsSet}${moment.tz(Config.timezone).format("YYYYMMDD")}`;
            candidates.forEach(async (candidate) => {
                try {
                    const vehicleTripKey = `${candidate.vehicleTripInfo.vehicleInfo.vehicleId}|${candidate.allocation.tripId}`;
                    const alreadySent = await this.redisClient.smembersAsync(triggeredDetectionsSet);
                    if (alreadySent.includes(vehicleTripKey)) {
                        Config.logger.info(`Already send ${vehicleTripKey}`);
                    } else {
                        const site = _.get(candidate.vehicleTripInfo.dilaxMessage, "device.site");
                        const dilaxVehicleId = site ? `${site} - ` : "";
                        const latitude = _.get(candidate.vehicleTripInfo.dilaxMessage, "wpt.lat");
                        const longitude = _.get(candidate.vehicleTripInfo.dilaxMessage, "wpt.lon");
                        const timestampString = `Last Message Timestamp: \
                        ${
                            candidate.vehicleTripInfo.lastReceivedTimestamp
                                ? moment.unix(parseInt(candidate.vehicleTripInfo.lastReceivedTimestamp, 10)).tz(Config.timezone).format("YYYY-MM-DD HH:mm:ss z")
                                : "Never received a Dilax message"
                        }`;
                        let coordinatesString = "Last Coordinates: ";
                        if (latitude || longitude) {
                            coordinatesString += latitude ? `Latitude: ${latitude}; ` : "";
                            coordinatesString += longitude ? `Longitude: ${longitude}` : "";
                        } else {
                            coordinatesString += "No GPS Position available";
                        }

                        Config.logger.warn(`Dilax Connection Lost: Vehicle: ${dilaxVehicleId}${candidate.vehicleTripInfo.vehicleInfo.label} - \
                    ${candidate.vehicleTripInfo.vehicleInfo.vehicleId}; TripId: ${candidate.allocation.tripId}; ${timestampString}; ${coordinatesString}`);
                        const existingSet = await this.redisClient.smembersAsync(triggeredDetectionsSet);
                        existingSet.push(vehicleTripKey);
                        await this.redisClient.saddAsync(triggeredDetectionsSet, existingSet);
                        await this.redisClient.expireAsync(triggeredDetectionsSet, 7 * 24 * 60 * 60);
                        await this.redisClient.setexAsync(`${triggeredDetectionsSet}:${vehicleTripKey}`, 7 * 24 * 60 * 60, JSON.stringify(candidate));
                    }
                } catch (error) {
                    newrelic.noticeError(error);
                    Config.logger.error(`Error detecting lost connections: ${error.stack || error.message}`);
                }
            });
        } catch (error) {
            newrelic.noticeError(error);
            Config.logger.error(`Error detecting lost connections: ${error.stack || error.message}`);
        } finally {
            this.detectLostConnectionsTimer = setTimeout(this.startDetectingLostConnections.bind(this), 10000);
        }
    }

    public stopDetectingLostConnections(): void {
        clearTimeout(this.detectLostConnectionsTimer);
        Config.logger.info("Stopped detecting lost dilax connections.");
    }

    public async detectCandidates(): Promise<{ detectionTime: number; allocation: VehicleAllocation; vehicleTripInfo: VehicleTripInfo }[]> {
        const detectionTime = moment().unix();
        Config.logger.info(`Start detecting lost dilax connection with time ${detectionTime}`);
        const allocations = this.cache.get<VehicleAllocation[]>(this.allocationsCacheKey) || [];
        const runningTrips = allocations.filter((allocation) => allocation.startDatetime <= detectionTime && allocation.endDatetime >= detectionTime);
        Config.logger.debug(`Following services are currently running: ${JSON.stringify(runningTrips)}`);

        const candidates = await Promise.all(
            runningTrips.map(async (allocation) => {
                const vehicleTripInfoAsString = await this.redisClient.getAsync(`${Config.redis.keyVehicleTripInfo}:${allocation.vehicleId}`);
                if (vehicleTripInfoAsString) {
                    return this.detectForExistingVehicleTripInfo(detectionTime, vehicleTripInfoAsString, allocation);
                } else {
                    return this.detectForAllocation(detectionTime, allocation);
                }
            }),
        );

        return <{ detectionTime: number; allocation: VehicleAllocation; vehicleTripInfo: VehicleTripInfo }[]>candidates.filter((candidate) => candidate);
    }

    private detectForExistingVehicleTripInfo(
        detectionTime: number,
        vehicleTripInfoAsString: string,
        allocation: VehicleAllocation,
    ): { detectionTime: number; allocation: VehicleAllocation; vehicleTripInfo: VehicleTripInfo } | undefined {
        const vehicleTripInfo: VehicleTripInfo = JSON.parse(vehicleTripInfoAsString);
        if (allocation.tripId === vehicleTripInfo.tripId) {
            if (this.isDilaxConnectionLost(detectionTime, Number(vehicleTripInfo.lastReceivedTimestamp))) {
                return { detectionTime, allocation, vehicleTripInfo };
            }
        } else {
            return this.detectForAllocation(detectionTime, allocation, vehicleTripInfo);
        }
    }

    private detectForAllocation(
        detectionTime: number,
        allocation: VehicleAllocation,
        oldVehicleInfoPayload?: VehicleTripInfo,
    ): { detectionTime: number; allocation: VehicleAllocation; vehicleTripInfo: VehicleTripInfo } | undefined {
        if (this.isDilaxConnectionLost(detectionTime, allocation.startDatetime)) {
            Config.logger.debug(`${allocation.vehicleLabel} - ${allocation.vehicleId} lost tracking. \
            TripStartTime: ${allocation.startDatetime}, Detection: ${moment().unix()}`);
            const vehicleTripInfo = oldVehicleInfoPayload || {
                tripId: allocation.tripId,
                vehicleInfo: {
                    label: allocation.vehicleLabel,
                    vehicleId: allocation.vehicleId,
                },
            };
            return {
                detectionTime,
                allocation,
                vehicleTripInfo,
            };
        }
    }

    private isDilaxConnectionLost(detectionTime: number, timestamp: number): boolean {
        return moment.unix(timestamp).add(Config.dilaxConnectionLostThreshold, "minutes").isBefore(moment.unix(detectionTime));
    }
}
