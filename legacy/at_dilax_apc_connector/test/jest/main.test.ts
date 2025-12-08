import { KafkaProducer } from "at-realtime-common/kafka";
import { Main } from "../../src/main";

describe("Test main", () => {
    let main: Main;
    let kafkaStart: jest.SpyInstance;
    let kafkaStop: jest.SpyInstance;

    beforeEach(async () => {
        process.env = {
            CONFLUENT_KAFKA_ENVIRONMENT: "test",
        };

        jest.clearAllMocks();
        main = new Main();
        kafkaStart = jest.spyOn(KafkaProducer.prototype, "start").mockImplementation(() => Promise.resolve());
        kafkaStop = jest.spyOn(KafkaProducer.prototype, "stop").mockImplementation(() => Promise.resolve());
    });

    it("Main start and stop, should start and stop confluent producer", async () => {
        await main.start();
        expect(kafkaStart).toHaveBeenCalledTimes(1);
        await main.close();
        expect(kafkaStop).toHaveBeenCalledTimes(1);
    });
});
