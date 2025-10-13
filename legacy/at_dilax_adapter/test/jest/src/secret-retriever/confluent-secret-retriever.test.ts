import { Config } from "../../../../src/config";
import { ConfluentSecretRetriever } from "../../../../src/secret-retriever/confluent-secret-retriever";
import { KeyVault } from "../../../../src/secret-retriever/key-vault";

jest.useFakeTimers();

describe("Test loading configuration", () => {
    beforeEach(async () => {
        jest.clearAllMocks();
        jest.clearAllTimers();
        ConfluentSecretRetriever.confluentKafkaSecret = { username: "", password: "" };
        Config.useConfluentKafkaConfig = true;
    });

    test("when kafka secret is not retrieved, should throw error for invalid kafka key", async () => {
        jest.spyOn(KeyVault.prototype, "getSecret").mockImplementation(() => Promise.resolve(""));
        await expect(ConfluentSecretRetriever.retrieveAndWatch(jest.fn())).rejects.toThrow("No valid kafka keys");
    });

    test("when kafka secret is retrieved, should get correct confluent kafka username and password", async () => {
        const validSecret = { key: "ABCDEF", secret: "my secret" };
        const getSecret = jest.spyOn(KeyVault.prototype, "getSecret").mockImplementation(() => Promise.resolve(JSON.stringify(validSecret)));
        await ConfluentSecretRetriever.retrieveAndWatch(jest.fn());
        expect(getSecret).toHaveBeenCalledWith("realtime-confluent-key");
        expect(ConfluentSecretRetriever.confluentKafkaSecret.username).toEqual(validSecret.key);
        expect(ConfluentSecretRetriever.confluentKafkaSecret.password).toEqual(validSecret.secret);
    });

    test("when has local dev, should not call get secret", async () => {
        ConfluentSecretRetriever.confluentKafkaSecret = { username: "ABCDEF", password: "my secret" };
        const getSecret = jest.spyOn(KeyVault.prototype, "getSecret").mockImplementation(() => Promise.resolve(""));
        await ConfluentSecretRetriever.retrieveAndWatch(jest.fn());
        expect(getSecret).not.toHaveBeenCalled();
    });

    test("when kafka secret is changed, should close app", async () => {
        const initSecret = { key: "ABCDEF", secret: "my secret" };
        const updatedSecret = { key: "FEDCBA", secret: "my secret 2" };
        const getSecret = jest
            .spyOn(KeyVault.prototype, "getSecret")
            .mockImplementation(() => Promise.resolve(JSON.stringify(initSecret)))
            .mockImplementationOnce(() => Promise.resolve(JSON.stringify(initSecret)))
            .mockImplementationOnce(() => Promise.resolve(JSON.stringify(updatedSecret)));
        const closeApp = jest.fn();
        await ConfluentSecretRetriever.retrieveAndWatch(closeApp);
        jest.runAllTimers();
        expect(getSecret).toHaveBeenCalledTimes(5);
        expect(getSecret).toHaveBeenCalledWith("realtime-confluent-key");
        expect(getSecret).toHaveBeenCalledWith("realtime-schema-registry-key");
    });
});
