import { ConfluentSecretRetriever } from "../../src/confluent-secret-retriever";
import { KeyVault } from "../../src/key-vault";

jest.useFakeTimers();

describe("Test loading configuration", () => {
    beforeEach(async () => {
        ConfluentSecretRetriever.confluentKafkaSecret = { username: "", password: "" };
        ConfluentSecretRetriever.schemaRegistrySecret = { apiKey: "", apiSecret: "" };
    });

    afterEach(async () => {
        jest.clearAllTimers();
        jest.clearAllMocks();
    });

    it("when kafka secret is not retrieved, should throw error for invalid kafka key", async () => {
        jest.spyOn(KeyVault.prototype, "getSecret").mockImplementation(() => Promise.resolve(""));
        await expect(ConfluentSecretRetriever.retrieve(jest.fn())).rejects.toThrow("No valid kafka keys");
    });

    it("when kafka secret is retrieved, should get correct confluent kafka username and password", async () => {
        const validSecret = { key: "ABCDEF", secret: "my secret" };
        const getSecret = jest.spyOn(KeyVault.prototype, "getSecret").mockImplementation(() => Promise.resolve(JSON.stringify(validSecret)));
        await ConfluentSecretRetriever.retrieve(jest.fn());
        jest.runAllTimers();

        expect(getSecret).toHaveBeenCalledWith("realtime-confluent-key");
        expect(ConfluentSecretRetriever.confluentKafkaSecret.username).toEqual(validSecret.key);
        expect(ConfluentSecretRetriever.confluentKafkaSecret.password).toEqual(validSecret.secret);

        expect(getSecret).toHaveBeenCalledWith("realtime-schema-registry-key");
        expect(ConfluentSecretRetriever.schemaRegistrySecret.apiKey).toEqual(validSecret.key);
        expect(ConfluentSecretRetriever.schemaRegistrySecret.apiSecret).toEqual(validSecret.secret);
    });

    it("when has local dev, should not call get secret", async () => {
        ConfluentSecretRetriever.confluentKafkaSecret = { username: "ABCDEF", password: "my secret" };
        ConfluentSecretRetriever.schemaRegistrySecret = { apiKey: "ABCDEF", apiSecret: "my secret" };
        const getSecret = jest.spyOn(KeyVault.prototype, "getSecret").mockImplementation(() => Promise.resolve(""));

        await ConfluentSecretRetriever.retrieve(jest.fn());
        jest.runAllTimers();

        expect(getSecret).not.toHaveBeenCalled();
    });

    it("when kafka secret is changed, should close app", async () => {
        const initSecret = { key: "ABCDEF", secret: "my secret" };
        const updatedSecret = { key: "FEDCBA", secret: "my secret 2" };
        const getSecret = jest
            .spyOn(KeyVault.prototype, "getSecret")
            .mockReturnValueOnce(Promise.resolve(JSON.stringify(initSecret)))
            .mockReturnValueOnce(Promise.resolve(JSON.stringify(initSecret)))
            .mockReturnValue(Promise.resolve(JSON.stringify(updatedSecret)));
        const closeApp = jest.fn(() => {
            console.log("Close App");
        });

        await ConfluentSecretRetriever.retrieve(closeApp);
        jest.runAllTimers();

        expect(getSecret).toHaveBeenCalledTimes(5);
        expect(getSecret).toHaveBeenCalledWith("realtime-confluent-key");
        expect(getSecret).toHaveBeenCalledWith("realtime-schema-registry-key");
        expect(closeApp).toHaveBeenCalled();
    });
});
