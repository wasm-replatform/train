import * as nr from "newrelic";
import * as moment from "moment-timezone";
import { Config } from "./config";
import { GtfsApi } from "./services/gtfs-api";
import { BlockMgtApi } from "./services/block-mgt-api";
import { ChangeType, MovementType, TrainUpdate } from "./train-update";
import { EventData, LocationData, MessageData, RemoteData, SmarTrakEvent } from "at-connector-common";

export class R9kToSmartrak {
    constructor(private gtfsApi: GtfsApi, private blockMgtApi: BlockMgtApi) { }

    public async convert(trainUpdate: TrainUpdate): Promise<SmarTrakEvent[]> {
        // Filter out updates which don't tell us about the progress of a trip (these can come at any time)
        if (trainUpdate.changes.length === 1 && this.getMovementType(trainUpdate.changes[0].changeType) === MovementType.OTHER) {
            nr.incrementMetric(`${Config.newRelicPrefix}/not_relevant_type_message_counter`, 1);
            return [];
        }

        const stopInfo = this.gtfsApi.getStopInfoByStopCode(Config.STATION_ID_TO_STOP_CODE_MAP[trainUpdate.changes[0].station]);
        if (!stopInfo || !this.isR9KRelevantStation(stopInfo)) {
            nr.incrementMetric(`${Config.newRelicPrefix}/not_relevant_station_message_counter`, 1);
            return [];
        }

        const allocatedVehicles = await this.blockMgtApi.getVehiclesByExternalRefId(trainUpdate.evenTrainId || trainUpdate.oddTrainId);
        return allocatedVehicles
            .map((label) => {
                const overwriteLocation = Config.DEPARTURE_LOCATION_OVERWRITE[Number.parseInt(stopInfo.stop_code, 10)];
                const eventLocation = this.getMovementType(trainUpdate.changes[0].changeType) !== MovementType.ARRIVAL && overwriteLocation ? overwriteLocation : stopInfo;
                return this.toSmartrakEvent(label, eventLocation, trainUpdate.createdDate);
            });
    }

    private isR9KRelevantStation(trainStation: { stop_code: string }) {
        return Config.filterStations.some((station) => Config.STATION_ID_TO_STOP_CODE_MAP[Number.parseInt(station, 10)] === trainStation.stop_code);
    }

    private getMovementType(changeType: ChangeType): MovementType {
        switch (+changeType) {
            case ChangeType.ReachedFinalDestination:
            case ChangeType.ArrivedAtStation:
                return MovementType.ARRIVAL;
            case ChangeType.ExitedFirstStation:
            case ChangeType.ExitedStation:
            case ChangeType.PassedStationWithoutStopping:
                return MovementType.DEPARTURE;
            case ChangeType.ScheduleChange:
                return MovementType.PREDICTION;
        }

        return MovementType.OTHER;
    }

    private toSmartrakEvent(vehicleLabel: string, station: { stop_lat: number, stop_lon: number }, trainUpdateCreationDate: string): SmarTrakEvent {
        const smarTrakEvent = new SmarTrakEvent();
        smarTrakEvent.eventType = "Location";
        smarTrakEvent.receivedAt = moment.tz(trainUpdateCreationDate, Config.r9kDateFormat, Config.timezone).toDate();

        const messageData = new MessageData();
        messageData.timestamp = moment.tz(Config.timezone).utc().toDate();
        smarTrakEvent.messageData = messageData;

        smarTrakEvent.eventData = new EventData();

        const remoteData = new RemoteData();
        remoteData.externalId = vehicleLabel.replace(/ /g, "");
        smarTrakEvent.remoteData = remoteData;

        const locationData = new LocationData();
        locationData.gpsAccuracy = 0;
        locationData.latitude = station.stop_lat;
        locationData.longitude = station.stop_lon;
        locationData.speed = 0;
        smarTrakEvent.locationData = locationData;

        return smarTrakEvent;
    }
}
