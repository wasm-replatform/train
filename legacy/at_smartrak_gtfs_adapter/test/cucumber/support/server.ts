import { FastifyAdapter } from "at-realtime-common/server";

export class TestServer {
    private httpServer: FastifyAdapter;
    private blockApiResponse = [];
    private trip: any[] = [
        {
            route_id: "25B-202",
            stop_id: "1120-01406-64500-2-672e64aa'",
            trip_id: "",
            stopTimes: [
                {
                    arrival_time: "09:25:00",
                    departure_time: "09:25:00",
                    stop_id: "8355-34ef1334",
                    stop_sequence: 1,
                },
            ],
        },
    ];

    public start(port: number): void {
        this.httpServer = new FastifyAdapter();

        this.httpServer.get("/vehicles", (req: any, res: any) => {
            if (req.query.label === "AMP        999") {
                res.send([
                    {
                        id: "98765",
                        label: "AMP        999",
                        type: {
                            type: "Train",
                        },
                        tag: "Smartrak",
                    },
                ]);
            } else if (req.query.label === "AMP        666") {
                res.send([
                    {
                        id: "43210",
                        label: "AMP        666",
                        type: {
                            type: "Train",
                        },
                        tag: "CAF",
                    },
                ]);
            } else if (req.query.id === "4563") {
                res.send([
                    {
                        id: "4563",
                        label: "BT 1234",
                        registration: "ENE46",
                        type: {
                            type: "Bus",
                        },
                        tag: "Smartrak",
                    },
                ]);
            } else if (req.query.id === "5001") {
                res.send([
                    {
                        id: "5001",
                        label: "BT 5001",
                        registration: "ENE47",
                        type: {
                            type: "Bus",
                        },
                        tag: "CAF",
                    },
                ]);
            } else if (req.query.id === "5002") {
                res.send([
                    {
                        id: "5002",
                        label: "BT 5002",
                        registration: "ENE48",
                        type: {
                            type: "Bus",
                        },
                        tag: "Smartrak",
                    },
                ]);
            } else {
                res.send(this.trip.find((t) => t.trip_id === (req.params as any).id));
            }
        });

        this.httpServer.get("/public-restricted/vehicles/byRefId/:vehicleId", (req: any, res: any) => {
            res.send({ response: { vehicles: [] } });
        });

        this.httpServer.post("/tripinstances", (req: any, res: any) => {
            const tripId = req.body.tripIds[0];
            const serviceDate = req.body.serviceDate;
            res.send({ tripInstances: this.trip.filter((t) => t.tripId === tripId && t.serviceDate === serviceDate) });
        });

        this.httpServer.get("/allocations/vehicles/:vehicleId", (req: any, res: any) => {
            res.send({ current: this.blockApiResponse });
        });

        this.httpServer.listen(port);
    }

    public setTrip(trip: any) {
        this.trip.push(trip);
    }

    public setBlockApiResponse(response: any) {
        this.blockApiResponse = response;
    }

    public getTrip(): any {
        return this.trip;
    }

    public async stop() {
        await this.httpServer.close();
    }
}
