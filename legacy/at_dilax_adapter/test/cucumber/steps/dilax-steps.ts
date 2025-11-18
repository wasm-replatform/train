import { AuthTokenRetriever } from "at-realtime-common/auth";
import { HttpClient } from "at-realtime-common/http-client";
import { KafkaConsumer, KafkaProducer, MockKafka } from "at-realtime-common/kafka";
import { Logger } from "at-realtime-common/logger";
import { IRedisConfig, Redis } from "at-realtime-common/redis";
import { assert } from "chai";
import { TableDefinition } from "cucumber";
import { after, before, binding, given, then, when } from "cucumber-tsflow";
import * as _ from "lodash";
import * as NodeCache from "node-cache";
import { Config } from "../../../src/config";
import { KafkaConsumerConfig } from "../../../src/config/kafka-consumer";
import { KafkaProducerConfig } from "../../../src/config/kafka-producer";
import { DilaxState } from "../../../src/dilax-adapter-processor";
import Main from "../../../src/main";
import BlockMgtClientAPI from "../../../src/services/block-mgt-client-api";
import CcStaticApi from "../../../src/services/cc-static-api";
import FleetApi from "../../../src/services/fleet-api";
import GtfsStaticApi from "../../../src/services/gtfs-static-api";
import { Server } from "../support/server";
const promisifyAll = require("bluebird").promisifyAll;

class CcStaticApiStub extends CcStaticApi {
    public stopInfo: any = {};

    public async getStopsInfoByLongLat(): Promise<any> {
        return [this.stopInfo];
    }
}

class FleetApiStub extends FleetApi {
    public fleetInfo: any = {};

    public async trainByLabel() {
        return {
            id: this.fleetInfo.vehicleId,
            capacity: {
                total: this.fleetInfo.capacityTotal,
                seating: this.fleetInfo.capacitySeating,
            },
        };
    }
}

class GtfsStaticApiStub extends GtfsStaticApi {
    public stopTypes: any = {
        route_type: 2,
        stop_code: "140",
        parent_stop_code: "140",
    };
    public async getStopTypes() {
        return [this.stopTypes];
    }
}

class BlockMgtClientApiStub extends BlockMgtClientAPI {
    public tripId = null;
    public async getAllocationByVehicleId() {
        return {
            tripId: this.tripId,
        };
    }
    public async getAllAllocations() {
        return [];
    }
}

/**
 * Dilax adapter test steps.
 */
@binding()
class DilaxSteps {
    private log = Config.logger;

    private redisClient: Redis;
    private main: Main;
    private publisherInput: KafkaProducer;
    private consumerOutput: KafkaConsumer;
    private server: any;
    private dilaxEvent: any;
    private dilaxEnrichedEventMessage: any;
    private ccStaticApiStub: CcStaticApiStub;
    private fleetApiStub: FleetApiStub;
    private gtfsStaticApiStub: GtfsStaticApiStub;
    private blockMgtClientApiStub: BlockMgtClientApiStub;
    private vehicleId: string;
    private producerConfig: KafkaProducerConfig;
    private consumerConfig: KafkaConsumerConfig;

    @before({ tag: undefined, timeout: 3000 })
    public async before(): Promise<void> {
        Config.fleetApiUrl = "http://localhost:3000";
        process.env = {
            KAFKA_CONSUMER_GROUP: "at-dilax-adapter-adapter",
            KAFKA_ENVIRONMENT: "dev",
            KAFKA_HOSTS: "mock-host",
        };
        // boot http server
        this.server = new Server();
        this.server.start(3000);
        this.producerConfig = new KafkaProducerConfig();
        this.consumerConfig = new KafkaConsumerConfig();

        this.consumerOutput = new MockKafka(this.log, Config, {
            topics: [this.producerConfig.topic],
        });
        this.dilaxEnrichedEventMessage = null;
        this.consumerOutput.onMessage((message) => {
            this.log.info(`dilaxEnrichedEventMessage [${message.value}]`);
            this.dilaxEnrichedEventMessage = JSON.parse(message.value as string);
        });

        this.publisherInput = new MockKafka(this.log, Config, this.producerConfig);

        this.redisClient = new Redis({} as Logger, { mock: true } as IRedisConfig);
        const dataStr = JSON.stringify({
            occupancyStatus: 0,
        });
        this.redisClient.getAsync = async (key): Promise<string | null> => {
            return dataStr;
        };
        const redisAsync = promisifyAll(this.redisClient);
        const httpClient = {} as unknown as HttpClient;
        const tokenRetriever = {
            getToken: () => {
                return "token";
            },
        } as unknown as AuthTokenRetriever;

        this.ccStaticApiStub = new CcStaticApiStub(httpClient);
        this.fleetApiStub = new FleetApiStub(redisAsync, httpClient);
        this.gtfsStaticApiStub = new GtfsStaticApiStub(new NodeCache(), httpClient);
        this.blockMgtClientApiStub = new BlockMgtClientApiStub(tokenRetriever, httpClient);

        this.main = new Main(
            new MockKafka(this.log, Config, this.consumerConfig),
            new MockKafka(this.log, Config, this.producerConfig),
            this.producerConfig.topic,
            this.fleetApiStub,
            this.ccStaticApiStub,
            this.gtfsStaticApiStub,
            this.blockMgtClientApiStub,
            redisAsync,
        );
        await this.consumerOutput.start();
        await this.publisherInput.start();
        await this.main.start();
    }

    @after({ tag: undefined, timeout: 3000 })
    public async after(): Promise<void> {
        this.server.stop();
        this.main?.stop();
        this.publisherInput?.stop();
        this.consumerOutput?.stop();
        (this.redisClient as any)?.redis?.flushall();
    }

    @given(/^a Dilax event with data:$/)
    public setupDilaxEvent(table: any): void {
        const rows = table.raw();
        const properties = rows[0];
        const values = rows[1];

        const data: any = {};

        // Populate the location data.
        for (let i = 0; i < properties.length; i++) {
            data[properties[i]] = this.convert(values[i]);
        }
        this.dilaxEvent = {
            device: {
                operator: "AUCKLAND",
                // tslint:disable-next-line:no-string-literal
                site: data.vehicleLabel,
                model: "CARM1",
                serial: "1420-03137",
            },
            clock: {
                // tslint:disable-next-line:no-string-literal
                utc: data.utc,
            },
            wpt: {
                sat: "5",
                // tslint:disable-next-line:no-string-literal
                lat: data.lat,
                // tslint:disable-next-line:no-string-literal
                lon: data.lon,
                speed: 0,
            },
            doors: [
                {
                    name: "M1D1",
                    // tslint:disable-next-line:no-string-literal
                    in: data.inDoor1,
                    // tslint:disable-next-line:no-string-literal
                    out: data.outDoor1,
                    art: 1,
                    st: "open",
                },
                {
                    name: "TD1",
                    // tslint:disable-next-line:no-string-literal
                    in: data.inDoor2,
                    // tslint:disable-next-line:no-string-literal
                    out: data.outDoor2,
                    art: 0,
                    st: "open",
                },
            ],
        };
        this.log.info(`dilaxEvent [${JSON.stringify(this.dilaxEvent)}]`);
    }

    @given(/^stop times data:$/)
    public setStopTimesInfo(table: any): void {
        const rows = table.raw();
        const values = rows[1];
        const stopTimesInfo = {
            stopIdFirst: undefined,
            stopIdLast: undefined,
        };
        stopTimesInfo.stopIdFirst = values[0];
        stopTimesInfo.stopIdLast = values[1];
        this.log.info(`stopTimesInfo [${JSON.stringify(stopTimesInfo)}]`);
    }

    @given(/^vehicle allocation data:$/)
    public setVehicleAllocationData(table: any): void {
        const tripId = _.get(table.hashes(), "[0].tripId");
        this.blockMgtClientApiStub.tripId = tripId;
        this.log.info(`vehicle allocated tripId [${tripId}]`);
    }

    @given(/^cc static api stop info data:$/)
    public setStopInfo(table: any): void {
        const rows = table.raw();
        const values = rows[1];
        const stopInfo = { stopId: undefined, stopCode: undefined };
        stopInfo.stopId = values[0];
        stopInfo.stopCode = values[1];
        this.ccStaticApiStub.stopInfo = stopInfo;
        this.log.info(`stopInfo [${JSON.stringify(stopInfo)}]`);
    }

    @given(/^fleet api vehicle mapping data:$/)
    public setVehicleMapping(table: any): void {
        const fleetInfo = table.hashes()[0];
        this.fleetApiStub.fleetInfo = fleetInfo;
        this.log.info(`vehicleId2VehicleLabel [${JSON.stringify(fleetInfo)}]`);
        this.vehicleId = fleetInfo.vehicleId;
    }

    @when(/^the event is published to the raw data topic$/, "", 50000)
    public async publishEvent(): Promise<void> {
        await this.publisherInput.publish(this.consumerConfig.topics[0], { value: JSON.stringify(this.dilaxEvent) });
    }

    @when(/^a Dilax Enriched event is published:$/, "", 50000)
    public async assertEnrichedEventPublished(table: TableDefinition): Promise<void> {
        const rows = table.raw();
        const values = rows[1];
        const tripId = values[0];
        const stopId = values[1];
        this.log.info(`tripId [${tripId}] stopId [${stopId}]`);
        await new Promise<void>((resolve, reject) => {
            // Check for a matching message once per second
            const interval = setInterval(() => {
                if (this.dilaxEnrichedEventMessage) {
                    const stopCodeActual = this.dilaxEnrichedEventMessage.stop_id.substring(0, this.dilaxEnrichedEventMessage.stop_id.indexOf("-"));
                    const stopCodeExpected = stopId.substring(0, stopId.indexOf("-"));
                    this.log.info(`stopCodeExpected [${stopCodeExpected}] stopCodeActual [${stopCodeActual}]`);
                    assert.equal(this.dilaxEnrichedEventMessage.trip_id, tripId, "trip_id");
                    assert.equal(stopCodeActual, stopCodeExpected, "stop_id");
                    clearInterval(interval);
                    resolve();
                }
            }, 1000);
        });
    }

    @then(/^passenger occupancy update:$/, undefined, 60000)
    public async assertOccupancyStatusUpdated(table: TableDefinition): Promise<void> {
        const rows = table.raw();
        const values = rows[1];
        const ocupancyStatus = values[0];
        await new Promise<void>((resolve) => {
            const fetchInterval = setInterval(async () => {
                this.redisClient.getAsync(`${Config.redis.apcVehicleIdStateKey}:${this.vehicleId}`).then((data) => {
                    const dilaxState = JSON.parse(data || "") as DilaxState;
                    assert.equal(dilaxState.occupancyStatus, ocupancyStatus, "Occupancy status");
                    clearInterval(fetchInterval);
                    resolve();
                });
            }, 1000);
        });
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

export = DilaxSteps;
