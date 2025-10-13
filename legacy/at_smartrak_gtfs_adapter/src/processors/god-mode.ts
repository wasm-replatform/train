// DEBUG ONLY
// allows to put any vehicle on any trip
// make sure it is disabled in production
import { Injectable } from "@nestjs/common";
import { SmarTrakEvent } from "at-realtime-common/model";

interface VehicleToTripMap {
    [vehicleId: string]: string;
}

@Injectable()
export class GodMode {
    private vehicleToTripMap: VehicleToTripMap = {};

    public resetAll() {
        this.vehicleToTripMap = {};
    }

    public resetVehicle(vehicleId: string) {
        delete this.vehicleToTripMap[vehicleId];
    }

    public setVehicleToTrip(vehicleId: string, tripId: string) {
        this.vehicleToTripMap[vehicleId] = tripId;
    }

    public preprocess(event: SmarTrakEvent): void {
        if (event.eventType === "SerialData") {
            if (!event.remoteData || !event.remoteData.externalId || !event.serialData || !event.serialData.decodedSerialData) {
                return;
            }

            const vehicleId = event.remoteData.externalId;

            if (this.vehicleToTripMap[vehicleId]) {
                event.serialData.decodedSerialData.lineId = "";
                event.serialData.decodedSerialData.tripNumber = this.vehicleToTripMap[vehicleId];
            }

            if (this.vehicleToTripMap[vehicleId] === "empty") {
                event.serialData.decodedSerialData.lineId = "";
                event.serialData.decodedSerialData.tripNumber = "";
            }
        }
    }

    public describe() {
        return JSON.stringify(this.vehicleToTripMap);
    }
}
