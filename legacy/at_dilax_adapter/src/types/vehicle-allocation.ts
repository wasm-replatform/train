export type VehicleAllocation = {
    operationalBlockId: string;
    tripId: string;
    serviceDate: string;
    startTime: string;
    vehicleId: string;
    vehicleLabel: string;
    routeId: string;
    directionId?: number | null;
    referenceId: string;
    endTime: string;
    delay: number;
    startDatetime: number;
    endDatetime: number;
    isCanceled: boolean;
    isCopied: boolean;
    timezone: string;
    creationDatetime: string;
};
