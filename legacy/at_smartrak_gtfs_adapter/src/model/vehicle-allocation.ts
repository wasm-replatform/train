import { Expose } from "@nestjs/class-transformer";
import { IsString } from "@nestjs/class-validator";
import { BaseModel } from "at-realtime-common/common";

export class Allocation extends BaseModel {
    @Expose()
    @IsString()
    public tripId: string;

    @Expose()
    @IsString()
    public serviceDate: string;

    @Expose()
    @IsString()
    public startTime: string;

    @Expose()
    @IsString()
    public vehicleId: string;

    constructor(partial: Partial<Allocation>) {
        super();
        this.assign(partial);
    }
}
