import { Injectable } from "@nestjs/common";
import { IAuthTokenRetrieverConfig } from "at-realtime-common/auth";
@Injectable()
export class AuthTokenRetrieverConfig implements IAuthTokenRetrieverConfig {
    public domain = "AucklandTransport.govt.nz";
    public clientId = process.env.APP_MANIFEST_CLIENT_ID || "8340ed14-0be3-497a-9807-889b24e14f10";
    public keyVault = {
        host: `https://${process.env.KEY_VAULT || "kv-ae-realtime-d01"}.vault.azure.net`,
        secretNameSystemClientSecret: process.env.KEY_VAULT_SECRET_NAME_SYSTEM_CLIENT_SECRET || "system-client-secret",
    };
    public localDevEnv = {
        accessToken: process.env.LOCAL_ACCESS_TOKEN || "",
    };
}
