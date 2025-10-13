import * as http from "http";
import * as express from "express";
import * as bodyParser from "body-parser";
import { Application } from "express";

export class Server {
    private app: Application;
    private server: http.Server;

    public start(port: number): void {
        this.app = express();
        this.app.use(bodyParser.raw());
        this.server = this.app.listen(port);
    }

    public stop(): void {
        this.server.close();
    }
}
