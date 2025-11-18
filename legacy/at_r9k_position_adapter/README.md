
# at-r9k-adapter-adapter

This repository is responsible for transforming R9k data for specific stations (check `process.env.STATIONS` default to `Britomart(0), Pukekohe(40), Manukau(19)`) into Smartrak like events and publish this data to [at_smartrak_gtfs_adapter](https://bitbucket.org/Propellerhead/at_smartrak_gtfs_adapter/src/master/).

### Table of Contents

- [Verify that Works](#markdown-header-verify-that-works)
- [Changelog](#markdown-header-changelog)
- [Feature Flags](#markdown-header-feature-flags)
- [Build and run instructions](#markdown-header-build-and-run-instructions)

# Verify that Works

After deployment make sure to check that application has been started and works properly:

- Check PT for start up logs after deploy:
```
dev-at-r9k-adapter-adapter-01 info [15996] Status endpoint is running 
dev-at-r9k-adapter-adapter-01 info [15996] Starting R9K Position adapter.. 
dev-at-r9k-adapter-adapter-01 info [15996] Starting Kafka Consumer (T: ["az-realtime-r9k","az-realtime-health-check"], CG: at-r9k-adapter-adapter-v2) 
dev-at-r9k-adapter-adapter-01 info [15996] Starting Kafka Producer (T: az-realtime-r9k-to-smartrak)
dev-at-r9k-adapter-adapter-01 info [15996] Status check configured. 
dev-at-r9k-adapter-adapter-01 info [15996] Elapse: 60001ms (sent: 0 | received: 12 | publishQueue: 0 | partitions: {"az-realtime-r9k":[3,4,5,6,8,9],"az-realtime-health-check":[1,2,3,5,6,9]}) 
```

### Confluent logs
```
dev-at-r9k-adapter-adapter-01 info [15996] Status endpoint is running 
dev-at-r9k-adapter-adapter-01 info [15996] Starting R9K Position adapter.. 
dev-at-r9k-adapter-adapter-01 info [15996] Starting Kafka Consumer (T: ["dev-realtime-r9k.v1","az-realtime-health-check"], CG: dev-r9k-adapter-adapter-v2) 
dev-at-r9k-adapter-adapter-01 info [15996] Starting Kafka Producer (T: dev-realtime-r9k-to-smartrak.v1)
dev-at-r9k-adapter-adapter-01 info [15996] Status check configured. 
dev-at-r9k-adapter-adapter-01 info [15996] Elapse: 60001ms (sent: 0 | received: 12 | publishQueue: 0 | partitions: {"az-realtime-r9k":[3,4,5,6,8,9],"az-realtime-health-check":[1,2,3,5,6,9]}) 
```
- Make sure that `/swap` has been triggered correctly (can have some retries)
```
test-at-r9k-adapter-adapter-01 info [24352] Swap called.
test-at-r9k-adapter-adapter-01 info [24352] Swap OK.
```
- Wait for about 5 - 10 min before processing
- Check NR for `AT R9k Position Adapter [ENV]` for `received_message_counter`
- Check NR for `AT R9k Position Adapter [ENV]` for `heartbeat`
- Check NR for lag in R9k Position Adapter

### Notes: 
* If r9k connector does not receive any messages than r9k adapter wont either (usually we don't get anything at night)
* Kafka re-balance messages are alright after deployment (for short amount of time):
```
error {"timestamp":"2020-03-01T20:59:47.285Z","logger":"kafkajs","message":"Response Heartbeat(key: 12, version: 1)","broker":"atazkafkd03.australiaeast.cloudapp.azure.com:9092","clientId":"dev-at-r9k-adapter-adapter-01-10196","error":"The group is rebalancing, so a rejoin is needed","correlationId":590284,"size":10} 
error {"timestamp":"2020-03-01T20:59:47.285Z","logger":"kafkajs","message":"The group is rebalancing, re-joining","groupId":"at-r9k-adapter-adapter-v2","memberId":"dev-at-r9k-adapter-adapter-01-10196-aa2291c5-5bf9-4230-a123-d2e866bfc1e0","error":"The group is rebalancing, so a rejoin is needed","retryCount":0,"retryTime":433} 
error {"timestamp":"2020-03-01T20:59:47.543Z","logger":"kafkajs","message":"Response Heartbeat(key: 12, version: 1)","broker":"atazkafkd03.australiaeast.cloudapp.azure.com:9092","clientId":"dev-at-r9k-adapter-adapter-01-9168","error":"The group is rebalancing, so a rejoin is needed","correlationId":507590,"size":10} 
error {"timestamp":"2020-03-01T20:59:47.543Z","logger":"kafkajs","message":"The group is rebalancing, re-joining","groupId":"at-r9k-adapter-adapter-v2","memberId":"dev-at-r9k-adapter-adapter-01-9168-1204386d-2fef-4ab3-8a89-204857041843","error":"The group is rebalancing, so a rejoin is needed","retryCount":0,"retryTime":431} 
```

# Changelog
| Date       | Ticket   | Changes                                                                                                                                                  |
| ---------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 10/07/2023 | ATR-5173 | Confluent kafka migration                                                                                                                                |
| 25/05/2020 | ATR-2354 | [Add slots, full code refactor, remove public api](https://bitbucket.org/Propellerhead/at_r9k_position_adapter/pull-requests/96/code-refactor-add-slots) |
| 02/02/2020 | ATR-2156 | - Refactor kafka onMessage commit logic                                                                                                                  |
|            |          | - Add timeouts and maxSockets limit to all outbound http requests                                                                                        |

## Feature Flags

## Build and run instructions

To run the project locally:

1. Copy and rename the `.env.example` to `.env` and populate the missing env vars 
2. Add the following code for to the top of the retrieve method in the ConfluentSecretRetriever class:
```bash
public static confluentKafkaSecret = { username: "[API KEY]", password: "[API SECRET]" };
return;
```