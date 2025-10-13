import { Injectable } from "@nestjs/common";
import { IKeyVaultConfig } from "at-realtime-common/keyvault";
@Injectable()
export class KeyVaultConfig implements IKeyVaultConfig {
    public kvHost: string = "https://" + process.env.KEY_VAULT + ".vault.azure.net";
    public secretWatcherInterval = (Number(process.env.SECRET_WATCHER_INTERVAL) || 5) * 60 * 1000;
}
