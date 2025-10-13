Feature: Publish an Enriched Dilax event to Kakfa
    When a Dilax event is published to the raw data topic,
    a corresponding Enriched Dilax event is published to the Enriched Dilax topic.

    Scenario: a Dilax Enriched event is published when the Dilax event is received
        Given a Dilax event with data:
            | vehicleLabel | lat        | lon        | inDoor1 | outDoor1 | inDoor2 | outDoor2 | utc          |
            | AM484        | -36.854700 | 174.777400 | 4       | 3        | 2       | 1        | 1591658525   |
        And fleet api vehicle mapping data:
            | vehicleLabel   | vehicleId | capacityTotal | capacitySeating |
            | AMP        484 | 59484     | 373           | 230             |
        And cc static api stop info data:
            | stopId         | stopCode |
            | 0140-56c57897  | 140      |
        And vehicle allocation data:
            | vehicleId | tripId                              | startDate | startTime |
            | 59484     | 249-820055-46440-2-1104313-919ae291 | 20220810  | 07:30:00  |
        When the event is published to the raw data topic
        Then a Dilax Enriched event is published:
            | tripId                              | stopId        | startDate | startTime |
            | 249-820055-46440-2-1104313-919ae291 | 0140-56c57897 | 20220810  | 07:30:00  |
        And passenger occupancy update:
            | occupancyStatus |
            | 0               |
