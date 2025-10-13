
# R9K Position Adapter

The R9K Position Adapter is responsible for transforming R9k data for specific stations into 
Smartrak events. The transformed events are then published to `smartrak_gtfs_adapter`.


<!-- Review and remove unneeded sections below
## Verifying deployment

TODO: automate these checks

After deployment make sure to check that application has been started and works properly:

- Check PT for start up logs after deploy:
```
dev-at-r9k-position-adapter-01 info [15996] Status endpoint is running 
dev-at-r9k-position-adapter-01 info [15996] Starting R9K Position adapter.. 
dev-at-r9k-position-adapter-01 info [15996] Starting Kafka Consumer (T: ["az-realtime-r9k","az-realtime-health-check"], CG: at-r9k-position-adapter-v2) 
dev-at-r9k-position-adapter-01 info [15996] Starting Kafka Producer (T: az-realtime-r9k-to-smartrak)
dev-at-r9k-position-adapter-01 info [15996] Status check configured. 
dev-at-r9k-position-adapter-01 info [15996] Elapse: 60001ms (sent: 0 | received: 12 | publishQueue: 0 | partitions: {"az-realtime-r9k":[3,4,5,6,8,9],"az-realtime-health-check":[1,2,3,5,6,9]}) 
```

### Confluent logs
```
dev-at-r9k-position-adapter-01 info [15996] Status endpoint is running 
dev-at-r9k-position-adapter-01 info [15996] Starting R9K Position adapter.. 
dev-at-r9k-position-adapter-01 info [15996] Starting Kafka Consumer (T: ["dev-realtime-r9k.v1","az-realtime-health-check"], CG: dev-r9k-position-adapter-v2) 
dev-at-r9k-position-adapter-01 info [15996] Starting Kafka Producer (T: dev-realtime-r9k-to-smartrak.v1)
dev-at-r9k-position-adapter-01 info [15996] Status check configured. 
dev-at-r9k-position-adapter-01 info [15996] Elapse: 60001ms (sent: 0 | received: 12 | publishQueue: 0 | partitions: {"az-realtime-r9k":[3,4,5,6,8,9],"az-realtime-health-check":[1,2,3,5,6,9]}) 
```

- Make sure that `/swap` has been triggered correctly (can have some retries)

```
test-at-r9k-position-adapter-01 info [24352] Swap called.
test-at-r9k-position-adapter-01 info [24352] Swap OK.
```

- Wait for about 5 - 10 min before processing
- Check NR for `AT R9k Position Adapter [ENV]` for `received_message_counter`
- Check NR for `AT R9k Position Adapter [ENV]` for `heartbeat`
- Check NR for lag in R9k Position Adapter

## Troubleshooting

* If r9k connector does not receive any messages than r9k adapter wont either (usually we don't get anything at night)
* Kafka re-balance messages are alright after deployment (for short amount of time):
```
error {"timestamp":"2020-03-01T20:59:47.285Z","logger":"kafkajs","message":"Response Heartbeat(key: 12, version: 1)","broker":"atazkafkd03.australiaeast.cloudapp.azure.com:9092","clientId":"dev-at-r9k-position-adapter-01-10196","error":"The group is rebalancing, so a rejoin is needed","correlationId":590284,"size":10} 
error {"timestamp":"2020-03-01T20:59:47.285Z","logger":"kafkajs","message":"The group is rebalancing, re-joining","groupId":"at-r9k-position-adapter-v2","memberId":"dev-at-r9k-position-adapter-01-10196-aa2291c5-5bf9-4230-a123-d2e866bfc1e0","error":"The group is rebalancing, so a rejoin is needed","retryCount":0,"retryTime":433} 
error {"timestamp":"2020-03-01T20:59:47.543Z","logger":"kafkajs","message":"Response Heartbeat(key: 12, version: 1)","broker":"atazkafkd03.australiaeast.cloudapp.azure.com:9092","clientId":"dev-at-r9k-position-adapter-01-9168","error":"The group is rebalancing, so a rejoin is needed","correlationId":507590,"size":10} 
error {"timestamp":"2020-03-01T20:59:47.543Z","logger":"kafkajs","message":"The group is rebalancing, re-joining","groupId":"at-r9k-position-adapter-v2","memberId":"dev-at-r9k-position-adapter-01-9168-1204386d-2fef-4ab3-8a89-204857041843","error":"The group is rebalancing, so a rejoin is needed","retryCount":0,"retryTime":431} 
```
-->