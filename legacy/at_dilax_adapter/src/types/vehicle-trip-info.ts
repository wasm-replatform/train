export interface VehicleTripInfo {
    lastReceivedTimestamp?: string;
    dilaxMessage?: DilaxEvent;
    tripId: string | null;
    stopId?: string | null;
    vehicleInfo: {
        label: string | undefined | null;
        vehicleId: string;
    };
}

export interface DilaxEvent {
    device: {
        operator: string;
        site: string;
        model: string;
        serial: string;
    };
    wpt?: {
        sat: string;
        lat: string;
        lon: string;
        speed: number;
    };
    clock: {
        utc: string;
    };
    doors: {
        name: string;
        in: number;
        out: number;
        art: number;
        st: string;
    }[];
}
