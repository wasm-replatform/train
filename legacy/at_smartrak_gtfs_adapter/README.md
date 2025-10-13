# at_smartrak_gtfs_adapter

This repository is responsible for consuming different connectors data and transforming that data in to GTFS VPs.

### Table of Contents

-   [Verify that Works](#markdown-header-verify-that-works)
-   [Changelog](#markdown-header-changelog)

# Verify that Works

After deployment make sure to check that application has been started and works properly:

-   Check PT for start up logs after deploy:

```
test-at-smartrak-gtfs-adapter-01 info [27816] Status endpoint is running
test-at-smartrak-gtfs-adapter-01 info [27816] New active gtfs version detected (prev , new 20200401133848_v89.54)
test-at-smartrak-gtfs-adapter-01 info [27816] Fetching new mappings 7280383d-fe4f-4e8e-884f-5ab5699bc791
test-at-smartrak-gtfs-adapter-01 info [27816] Starting Smartrak GTFS Adapter...
test-at-smartrak-gtfs-adapter-01 info [27816] Starting Kafka Producer (T: az-realtime-gtfs-vp.v2)
test-at-smartrak-gtfs-adapter-01 info [27816] Starting Kafka Consumer (T: ["az-realtime-smartrak-bus-avl","az-realtime-smartrak-train-avl","az-realtime-r9k-to-smartrak","az-realtime-kiwirail-to-smartrak","az-realtime-health-check"], CG: at-smartrak-gtfs-adapter-v2)
```

-   Make sure that `/swap` has been triggered correctly for all instances (can have some retries)

```
test-at-smartrak-gtfs-adapter-01 info [27816] Swap called.
test-at-smartrak-gtfs-adapter-01 info [27816] Swap OK.
```

-   Wait for about 5 - 10 min before processing
-   Check NR for `AT Smartrak GTFS [ENV]` for `received_message_counter` on all topics
-   Check NR for `AT Smartrak GTFS [ENV]` for `published_message_counter` on vp topic
-   Check NR for `AT Smartrak GTFS [ENV]` for `heartbeat`
-   Check NR for lag in `[CONSUMER GROUP]/[TOPIC]`
-   Check if vehicles have trips on the map and moving
-   Smartrak Adapter sends statistic to papertrail every 1 min check if data is going through:

```
test-at-smartrak-gtfs-adapter-01 info [19080] Elapse: 59986ms (sent: 1365 | received: 1734 | publishQueue: 2 | partitions: {"az-realtime-smartrak-bus-avl":[0,10,2,4,6,8],"az-realtime-smartrak-train-avl":[0,10,2,4,6,8],"az-realtime-r9k-to-smartrak":[0,10,2,4,6,8],"az-realtime-kiwwirail-to-smartrak":[0,10,2,4,6,8],"az-realtime-health-check":[0,10,2,4,6,8]})
```

### Notes:

-   Kafka re-balance messages are alright after deployment/restart (for short amount of time):

```
test-at-smartrak-gtfs-adapter-01 error [1780] {"timestamp":"2020-04-02T19:47:40.687Z","logger":"kafkajs","message":"Response SyncGroup(key: 14, version: 1)","broker":"atazkafkt02.aucklandtransport.govt.nz:9092","clientId":"test-at-smartrak-gtfs-adapter-01-1780","error":"The group is rebalancing, so a rejoin is needed","correlationId":1,"size":14}
```

# Changelog

| Date       | Ticket   | Changes                                                                                                                                                                                         |
| ---------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 20/06/2023 | ATR-5128 | [Confluent kafka migration](https://bitbucket.org/Propellerhead/at_smartrak_gtfs_adapter/pull-requests/160)                                                                                     |
| 15/09/2022 | ATR-4100 | [Remove cache remapping used to support new gtfs deployment](https://bitbucket.org/Propellerhead/at_smartrak_gtfs_adapter/pull-requests/156)                                                    |
| 11/08/2022 | ATR-3579 | [Output new gtfs ids to Kakfa topics](https://bitbucket.org/Propellerhead/at_smartrak_gtfs_adapter/pull-requests/155)                                                                           |
| 18/11/2020 | ATR-2706 | [Blacklist to filter out vehicle's events](https://bitbucket.org/Propellerhead/at_smartrak_gtfs_adapter/pull-requests/136)                                                                      |
| 17/08/2020 | ATR-2554 | [Call fleet api for capacity, update winston](https://bitbucket.org/Propellerhead/at_smartrak_gtfs_adapter/pull-requests/131)                                                                   |
| 25/06/2020 |          | [Update redis common, update build common, improve slot start up logic](https://bitbucket.org/Propellerhead/at_smartrak_gtfs_adapter/pull-requests/128/update-redis-common-update-build-common) |
| 02/06/2020 | ATR-2390 | [Update passenger counter only if both trains exist, improve code style](https://bitbucket.org/Propellerhead/at_smartrak_gtfs_adapter/pull-requests/123/update-passenger-counter-only-if-both)  |
| 26/05/2020 | ATR-2358 | [Update deps, implement new slots](https://bitbucket.org/Propellerhead/at_smartrak_gtfs_adapter/pull-requests/122/update-deps-implement-new-slots)                                              |
| 02/04/2020 | ATR-2246 | [Combined occupancy for multiple trains](https://bitbucket.org/Propellerhead/at_smartrak_gtfs_adapter/pull-requests/117)                                                                        |
