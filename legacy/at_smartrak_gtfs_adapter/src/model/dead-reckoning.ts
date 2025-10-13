import { transit_realtime } from "at-realtime-common/gtfs-realtime";

import ITripDescriptor = transit_realtime.ITripDescriptor;
export class DeadReckoningMessage {
    public id: string;
    public receivedAt: number;
    public position: PositionDR;
    public trip: ITripDescriptor;
    public vehicle: VehicleDR;

    constructor(id: string, receivedAt: number, position: PositionDR, trip: ITripDescriptor, vehicle: VehicleDR) {
        this.id = id;
        this.receivedAt = receivedAt;
        this.position = position;
        this.trip = trip;
        this.vehicle = vehicle;
    }
}
export class PositionDR {
    public odometer: number;

    constructor(odometer: number) {
        this.odometer = odometer;
    }
}
export class VehicleDR {
    public id: string;

    constructor(id: string) {
        this.id = id;
    }
}
