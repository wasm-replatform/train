import { assert } from "chai";
import { DataTable } from "@cucumber/cucumber";
import { after, before, binding, given, then, when } from "cucumber-tsflow";
import { TestingModule } from "@nestjs/testing";
import { bootstrapAppForTest } from "../support/app";
import { TestServer } from "../support/server";
import { KafkaConsumer, KafkaProducer, MockKafka, ConfluentTopics, Versions, getConfluentTopic } from "at-realtime-common/kafka";
import { Redis } from "at-realtime-common/redis";
import { DecodedSerialData, SmarTrakEvent as Event } from "at-realtime-common/model";
import { transit_realtime } from "at-realtime-common/gtfs-realtime";
import VehiclePosition = transit_realtime.VehiclePosition;
import { RedisConfig } from "../../../src/config/redis";
import { PassengerCountEvent } from "../../../src/processors/passenger-count";
import * as moment from "moment-timezone";
import { DeadReckoningMessage } from "../../../src/model/dead-reckoning";

@binding()
export class TrainBusSteps {
    private testSever: TestServer;
    private redisClient: Redis;
    private redisConfig: RedisConfig;

    private creatorOfSrcMessages: KafkaProducer;
    private vpConsumer: KafkaConsumer;
    private receiveVpPromise: Promise<{ value: Buffer | string | null }>;
    private drConsumer: KafkaConsumer;
    private receiveDrPromise: Promise<{ value: Buffer | string | null }>;
    private moduleRef: TestingModule;

    private vehiclePosition: VehiclePosition;
    private deadReckoning: DeadReckoningMessage;

    private event: Event;
    private serialEvent: Event;
    private passengerCountEvent: PassengerCountEvent;

    @before()
    public async before(): Promise<void> {
        process.env = {
            CONFLUENT_KAFKA_ENVIRONMENT: "dev",
            KAFKA_CONSUMER_GROUP: "smartrak-gtfs-adapter",
            USE_CAF_TOPIC: "true",
        };

        const supportServerPort = 3003;
        this.moduleRef = await bootstrapAppForTest(supportServerPort);
        this.redisClient = this.moduleRef.get(Redis);
        this.redisConfig = this.moduleRef.get(RedisConfig);
        this.testSever = new TestServer();
        this.vpConsumer = new MockKafka({}, {}, { topics: [getConfluentTopic(ConfluentTopics.GTFS_VP, process.env.CONFLUENT_KAFKA_ENVIRONMENT || "", Versions.V1)] });
        this.drConsumer = new MockKafka({}, {}, { topics: [getConfluentTopic(ConfluentTopics.DEAD_RECKONING, process.env.CONFLUENT_KAFKA_ENVIRONMENT || "", Versions.V1)] });

        this.creatorOfSrcMessages = new MockKafka({}, {}, {});
        this.receiveVpPromise = new Promise<{ value: Buffer | string | null }>((resolve) => {
            this.vpConsumer.onMessage((msg) => {
                resolve(msg);
            });
        });
        this.receiveDrPromise = new Promise<{ value: Buffer | string | null }>((resolve) => {
            this.drConsumer.onMessage((msg) => {
                resolve(msg);
            });
        });

        await this.vpConsumer.start();
        await this.drConsumer.start();
        await this.creatorOfSrcMessages.start();
        await this.testSever.start(supportServerPort);
    }

    @after()
    public async after(): Promise<void> {
        if (this.vpConsumer) {
            await this.vpConsumer.stop();
        }
        if (this.drConsumer) {
            await this.drConsumer.stop();
        }
        (
            this.moduleRef.get(Redis) as unknown as {
                redis: {
                    flushall: () => {
                        /** noop */
                    };
                };
            }
        ).redis.flushall();
        await this.moduleRef.get(KafkaConsumer).stop();
        await this.moduleRef.close();
        await this.testSever.stop();
    }

    @given(/^block api response:$/)
    public async setBlockApiResponse(table: DataTable) {
        this.testSever.setBlockApiResponse(table.hashes());
    }

    @when(/^the smartrak event is published to the raw data caf topic$/, "", 50000)
    public async publishCAFEvent() {
        await this.creatorOfSrcMessages.publish(getConfluentTopic(ConfluentTopics.CAF_AVL, process.env.CONFLUENT_KAFKA_ENVIRONMENT || "", Versions.V2), {
            value: JSON.stringify(this.event),
        });
    }

    @when(/^the passenger count event is published to the passenger count topic$/, "", 50000)
    public async publishPassengerCountEvent() {
        await this.creatorOfSrcMessages.publish(getConfluentTopic(ConfluentTopics.PASSENGER_COUNT, process.env.CONFLUENT_KAFKA_ENVIRONMENT || "", Versions.V1), {
            value: JSON.stringify(this.passengerCountEvent),
        });
    }

    @given(/^serialData event preprocessed$/)
    public async sendEventWithSpecificTimestamp() {
        await this.creatorOfSrcMessages.publish(getConfluentTopic(ConfluentTopics.SMARTRAK_BUS_AVL, process.env.CONFLUENT_KAFKA_ENVIRONMENT || "", Versions.V1), {
            value: JSON.stringify(this.event),
        });
    }

    @given(/^a location event with UTC timestamp "(.*)"$/)
    public setupDefaultLocationEvent(timestamp: string): void {
        if (this.event) {
            this.serialEvent = this.event;
        }
        this.event = <Event>{
            eventType: "Location",
            messageData: {
                timestamp: new Date(timestamp),
            },
            locationData: {
                latitude: -36.9105233,
                longitude: 174.680465,
                heading: 94,
                speed: 80,
            },
            eventData: {
                odometer: 123456789,
            },
        };
    }

    @given(/^a location event without location data and with UTC timestamp "(.*)"$/)
    public setupDefaultLocationWithoutLocationDataEvent(timestamp: number): void {
        if (this.event) {
            this.serialEvent = this.event;
        }
        this.event = <Event>{
            eventType: "Location",
            locationData: {},
            messageData: {
                timestamp: new Date(timestamp),
            },
            eventData: {
                odometer: 123456789,
            },
        };
    }

    @given(/^a location event without latitude and longitude with UTC timestamp "(.*)"$/)
    public setupDefaultLocationWithoutLatitudeAndLongitudeEvent(timestamp: string): void {
        if (this.event) {
            this.serialEvent = this.event;
        }
        this.event = <Event>{
            eventType: "Location",
            messageData: {
                timestamp: new Date(timestamp),
            },
            locationData: {
                heading: 94,
                speed: 80,
                odometer: 123456,
            },
            eventData: {},
        };
    }

    @given(/^(.*) data:$/)
    public setEventData(property: string, table: DataTable): void {
        const rows = table.raw();
        const properties = rows[0];
        const values = rows[1];

        const data: any = {};

        // Populate the location data.
        for (let i = 0; i < properties.length; i++) {
            data[properties[i]] = this.convert(values[i]);
        }

        // Ugly, but avoids "remotedata data" in the step.
        (<any>this.event)[property + "Data"] = data;
    }

    @given(/^passenger count event:$/)
    public setPassengerCount(table: DataTable) {
        const row = table.hashes()[0];
        this.passengerCountEvent = {
            vehicle: { id: row.vehicleId },
            trip: {
                tripId: row.tripId,
                startDate: row.serviceDate,
                startTime: row.startTime,
                routeId: row.routeId,
            },
            occupancyStatus: row.occupancyStatus,
        } as PassengerCountEvent;
    }

    @when(/^the smartrak event is published to the raw data smartrak topic$/, "", 50000)
    public async publishSmartrakEvent() {
        if (this.serialEvent) {
            await this.creatorOfSrcMessages.publish(getConfluentTopic(ConfluentTopics.SMARTRAK_BUS_AVL, process.env.CONFLUENT_KAFKA_ENVIRONMENT || "", Versions.V1), {
                value: JSON.stringify(this.serialEvent),
            });
            await new Promise<void>((resolve, reject) => {
                // Poll Redis to see if the the trip descriptor has been stored
                const interval = setInterval(() => {
                    this.redisClient.getAsync(`${this.redisConfig.keys.tripKey}:${this.serialEvent.remoteData.externalId}`).then((result) => {
                        if (result && result !== "null") {
                            clearInterval(interval);
                            resolve();
                        }
                    });
                }, 100);
            });
        }

        await this.creatorOfSrcMessages.publish(getConfluentTopic(ConfluentTopics.SMARTRAK_BUS_AVL, process.env.CONFLUENT_KAFKA_ENVIRONMENT || "", Versions.V1), {
            value: JSON.stringify(this.event),
        });
    }

    @then(/^it is not published to the GTFS realtime topic$/, undefined, 30000)
    public assertNotPublishToGTFSKafka(): Promise<void> {
        // Wait for the message then validate.
        return new Promise((resolve, reject) => {
            this.receiveVpPromise.then((msg) => reject("Received a message, expected none:" + msg.value));
            setTimeout(resolve, 2000);
        });
    }

    @then(/^it is not published to the Dead Reckoning topic$/, undefined, 30000)
    public assertNotPublishToDeadReckoningKafka(): Promise<void> {
        // Wait for the message then validate.
        return new Promise((resolve, reject) => {
            this.receiveDrPromise.then((msg) => reject("Received a message, expected none:" + msg.value));
            setTimeout(resolve, 2000);
        });
    }

    @then(/^a message is published to the Dead Reckoning topic with odometer value "(.*)"$/, undefined, 70000)
    public async assertMessagePublishedToDeadReckoningKafka(odometer: number): Promise<void> {
        return new Promise<void>((resolve) => {
            // Check for a matching message once per second
            const interval = setInterval(async () => {
                const message = await this.receiveDrPromise;
                const entity = JSON.parse(message.value as string);
                if (entity) {
                    if (entity.vehicle && odometer) {
                        // Save the message for possible other assertions
                        this.deadReckoning = entity;
                        assert.equal(this.deadReckoning.position.odometer, odometer);
                    }

                    clearInterval(interval);
                    resolve();
                }
            }, 100);
        });
    }

    @then(/^a GTFS-RT feed entity is published to the (train|bus) GTFS-RT topic with timestamp from "(.*)"$/, undefined, 70000)
    public async assertMessageTimestampPublished(type: string, timestamp: number): Promise<void> {
        return new Promise<void>((resolve) => {
            // Check for a matching message once per second
            const interval = setInterval(async () => {
                const message = await this.receiveVpPromise;
                const entity = JSON.parse(message.value as string);
                if (entity) {
                    if (entity.vehicle && timestamp) {
                        // Save the message for possible other assertions
                        this.vehiclePosition = entity.vehicle;
                        const result = moment(timestamp).unix();
                        assert.equal(this.vehiclePosition.timestamp, result);
                    }

                    clearInterval(interval);
                    resolve();
                }
            }, 100);
        });
    }

    @then(/^trip cache should be empty$/, undefined, 50000)
    public async cacheEmpty() {
        await new Promise<void>((resolve, reject) => {
            setTimeout(() => {
                this.redisClient.getAsync(`${this.redisConfig.keys.tripKey}:${this.event.remoteData.externalId}`).then((result) => {
                    if (result) {
                        assert.fail(result, undefined, "No trip info expected");
                    } else {
                        resolve();
                    }
                });
            }, 2000);
        });
    }

    @then(/^occupancy level is "(.*)"$/, undefined, 50000)
    public async storeOccupancyLevel(occupancyLevel: string) {
        if (!occupancyLevel) {
            assert.equal(this.vehiclePosition.occupancyStatus, undefined);
            return;
        }
        assert.equal(this.vehiclePosition.occupancyStatus.toString(), occupancyLevel, "Occupancy level");
    }

    @then(/^vehicle position:$/)
    public assertJsonLocationData(table: DataTable): void {
        this.validateProperties(this.vehiclePosition.position, table);
    }

    @then(/^vehicle details:$/)
    public assertJsonRemoteData(table: DataTable): void {
        this.validateProperties(this.vehiclePosition.vehicle, table);
    }

    @then(/^trip descriptor:$/)
    public checkTrip(table: DataTable) {
        const row = table.hashes()[0];
        assert.equal(this.vehiclePosition?.trip?.tripId, row.tripId);
        assert.equal(this.vehiclePosition?.trip?.routeId, row.routeId);
        assert.equal(this.vehiclePosition?.trip?.startDate, row.startDate);
        assert.equal(this.vehiclePosition?.trip?.startTime, row.startTime);
    }

    @then(/^no trip descriptor$/)
    public checkNoTrip() {
        assert.equal(this.vehiclePosition.trip, undefined);
    }

    @given(/^a Smartrak serial event with serialData:$/)
    public setupSerialEvent(table: DataTable): void {
        const row = table.hashes()[0];
        this.event = new Event();
        this.event.eventType = "SerialData";
        this.event.remoteData = {
            remoteId: 64514,
            remoteName: "PC 4026",
            externalId: "23126",
        };

        this.event.serialData = {
            source: 0,
            serialBytes: "asdf",
            decodedSerialData: {
                hasTripEndedFlag: true,
                driverId: "20613520",
                tripId: row.tripId,
                startAt: row.startAt,
                tripActive: true,
                tripEnded: false,
                passengersNumber: Number(row.passengersNumber) || 0,
            } as unknown as DecodedSerialData,
        };
    }

    @given(/^trip management result:$/)
    public stubTripManagementData(table: DataTable) {
        const rows = table.hashes();
        rows.forEach((row) => {
            this.testSever.setTrip({
                routeId: row.routeId,
                serviceDate: row.serviceDate,
                endTime: row.endTime || "",
                delay: row.delay || 0,
                startTime: row.departureTime,
                tripId: row.tripId,
                status: row.status || "IN_PROGRESS",
                stops: [
                    {
                        stopId: "one",
                        stopSequence: 1,
                    },
                    {
                        stopId: "two",
                        stopSequence: 2,
                    },
                    {
                        stopId: "eighteen",
                        stopSequence: 18,
                    },
                ],
            });
        });
    }

    @given(/^assigned vehicle:$/)
    public stubAssignedVehicle(table: DataTable) {
        const redisClient = this.moduleRef.get(Redis);
        const redisConfig = this.moduleRef.get(RedisConfig);
        const row = table.hashes()[0];
        const key = `${redisConfig.keys.allocatedVehicleKey}:${row.serviceDate}:${row.departureTime}:${row.tripId}`;
        redisClient.setAsync(key, row.vehicleId);
    }

    @given(/^vehicle "(.*)" sign on at "(.*)"$/)
    public stubVehicleSignOn(vehicleId: string, signOnTime: string) {
        const key = `${this.redisConfig.keys.vehicleSOTimeKey}:${vehicleId}`;
        this.redisClient.setAsync(key, JSON.stringify(moment.utc(signOnTime).unix()));
    }

    @given(/^there is cached trip for the same vehicle:$/)
    public stubRedisTripDescriptor(table: DataTable) {
        const row = table.hashes()[0];

        const tripInstance = {
            serviceDate: row.serviceDate,
            startTime: row.departureTime,
            endTime: row.endTime || "",
            delay: row.delay || 0,
            tripId: row.tripId,
            routeId: row.routeId,
            stops: [
                {
                    stopId: "one",
                    stopSequence: 1,
                },
                {
                    stopId: "two",
                    stopSequence: 2,
                },
                {
                    stopId: "eighteen",
                    stopSequence: 18,
                },
            ],
        };

        this.redisClient.setAsync(`${this.redisConfig.keys.tripKey}:${row.vehicleId}`, JSON.stringify(tripInstance));
    }

    @given(/^a trip with departure time "(.*)" and trip id "(.*)"$/)
    public tripWithDepartureTime(departureTime: string, tripId: string) {
        this.testSever.setTrip({
            trip_id: tripId,
            route_id: "25",
            stopTimes: [
                {
                    stop_id: "one",
                    departure_time: departureTime,
                    stop_sequence: 1,
                },
                {
                    stop_id: "two",
                    departure_time: departureTime,
                    stop_sequence: 2,
                },
                {
                    stop_id: "eighteen",
                    departure_time: departureTime,
                    stop_sequence: 18,
                },
            ],
        });
    }

    @given(/^blacklisted vehicle id "(.*)"$/)
    public async setVehicleBlacklist(vehicleId: string): Promise<void> {
        this.redisClient.setAsync(`${this.redisConfig.keys.vehicleBlacklistKey}:${vehicleId}`, "");
    }

    /**
     * Validates that object properties are the same as the ones defined in the data table.
     * @param object object to validate.
     * @param table data table consisted of 2 rows, one for property names and one for values.
     */
    private validateProperties(object: any, table: DataTable): void {
        const rows = table.raw();
        const properties = rows[0];
        const values = rows[1];

        // Validate the message received contains properties.
        for (let i = 0; i < properties.length; i++) {
            assert.equal(object[properties[i]], this.convert(values[i]), properties[i]);
        }
    }

    private convert(value: string): boolean | number | string {
        const maybe: boolean | number | string = this.maybeBoolean(value);
        return typeof maybe === "boolean" ? maybe : this.maybeNumber(value);
    }

    private maybeNumber(value: string): number | string {
        const maybe = Number(value);
        return isNaN(maybe) ? value : maybe;
    }

    private maybeBoolean(value: string): boolean | string {
        if (value === "true") {
            return true;
        } else if (value === "false") {
            return false;
        }
        return value;
    }
}
