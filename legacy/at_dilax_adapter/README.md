# README

This README would normally document whatever steps are necessary to get your application up and running.

### What is this repository for?

-   AT Dilax adapter
-   Version
-   [Learn Markdown](https://bitbucket.org/tutorials/markdowndemo)

-   Summary of set up

```shell
npm install
npm run build
npm run start
```

1. Copy and rename the `.env.example` to `.env`
2. Populate missing ENV variables in the `.env` (can be taken from `./azure/parameters.json` file and/or from an azure portal, LastPass, or other places the system needs the connection to)
3. Start the project with `npm run dev` command (to start the project from `src` itself). Alternatively, `npm run build && npm start` (to compile and start the project)

# To Do

After schema registry has gone live and worked well:

-   Merge all consumers into one and remove extra copies
-   realtime-common 6.8.8 already has logic to allow a single consumer to deal with both schema and non-schema topics

## Feature Flags

To connect to Confluent locally, you can add the secrets to the `confluent-secret-retriever.ts` file.

## Verify its working

```
Aug 02 18:02:50 at-realtime-test test-at-dilax-adapter-02 [10980] info: Dilax adapter started
Aug 02 18:02:51 at-realtime-test test-at-dilax-adapter-02 [20748] info: Connecting redis to [rc-test-at-realtime.redis.cache.windows.net]
Aug 02 18:02:51 at-realtime-test test-at-dilax-adapter-02 [20748] info: Redis [rc-test-at-realtime.redis.cache.windows.net] is ready
Aug 02 18:02:52 at-realtime-test test-at-dilax-adapter-02 [20748] info: Status check configured
Aug 02 18:02:52 at-realtime-test test-at-dilax-adapter-02 [20748] info: Starting the Dilax adapter...
Aug 02 18:02:52 at-realtime-test test-at-dilax-adapter-02 [20748] info: Loaded 1016 allocations
Aug 02 18:02:52 at-realtime-test test-at-dilax-adapter-02 [20748] info: Caching 414 allocations for today
Aug 02 18:02:52 at-realtime-test test-at-dilax-adapter-02 [20748] info: Start detecting lost dilax connection with time 1690956172
Aug 02 18:02:52 at-realtime-test test-at-dilax-adapter-02 [20748] info: Starting Kafka Producer (T: tst-realtime-dilax-apc-enriched.v1)
Aug 02 18:02:52 at-realtime-test test-at-dilax-adapter-02 [20748] info: Starting Kafka Consumer (T: ["tst-realtime-dilax-apc.v1","az-realtime-health-check"], CG: tst-at-dilax-adapter)
```

-   Configuration

-   Dependencies

```
node 18
```

-   Database configuration
-   How to run tests
-   Deployment instructions

### Contribution guidelines

-   Writing tests
-   Code review
-   Other guidelines

### Who do I talk to?

-   Repo owner or admin
-   Other community or team contact
