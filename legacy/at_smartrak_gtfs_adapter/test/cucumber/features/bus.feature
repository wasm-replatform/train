Feature: Publish a Smartrak bus GTFS realtime vehicle position to Kakfa
    When a Smartrak bus event is published to the raw data topic, a corresponding GTFS RT feed is published to the GTFS RT bus topic.

  Scenario: Don't publish an event with no remote data
    Given a location event with UTC timestamp "2018-10-23T14:24:30.000Z"
    And location data:
      | latitude    | longitude  | heading | speed |
      | -36.9105233 | 174.680465 | 94      | 60    |
    And event data:
      | odometer  |
      | 201534366 |
    When the smartrak event is published to the raw data smartrak topic
    Then it is not published to the GTFS realtime topic

  Scenario: Publish Smartrak bus GTFS realtime vehicle position
    Given a location event with UTC timestamp "2018-10-23T14:24:30.000Z"
    And location data:
      | latitude    | longitude  | heading | speed |
      | -36.9105233 | 174.680465 | 94      | 60    |
    And event data:
      | odometer  |
      | 201534366 |
    And remote data:
      | externalId | remoteName |
      | 4563       | name       |
    When the smartrak event is published to the raw data smartrak topic
    Then a GTFS-RT feed entity is published to the bus GTFS-RT topic with timestamp from "2018-10-23T14:24:30.000Z"
    And vehicle position:
      | latitude    | longitude  | bearing | speed              | odometer  |
      | -36.9105233 | 174.680465 | 94      | 16.666666666666668 | 201534366 |
    And vehicle details:
      | id   | label   | licensePlate |
      | 4563 | BT 1234 | ENE46        |

  Scenario: Publish Smartrak bus serial event to GTFS realtime Kafka
    Given a Smartrak serial event with serialData:
      | tripId     | startAt       |
      | 1084005208 | 1531146000000 |
    And message data:
      | timestamp                |
      | 2018-10-23T14:24:30.000Z |
    And remote data:
      | externalId | remoteName |
      | 4563       | name       |
    And trip management result:
      | tripId     | routeId | departureTime | serviceDate |
      | 1084005208 | 25      | 02:20:00      | 20181024    |
    And a location event with UTC timestamp "2018-10-23T14:24:30.000Z"
    And location data:
      | latitude    | longitude  | heading | speed |
      | -36.9105233 | 174.680465 | 94      | 60    |
    And event data:
      | odometer  |
      | 201534366 |
    And remote data:
      | externalId | remoteName |
      | 4563       | name       |
    When the smartrak event is published to the raw data smartrak topic
    Then a GTFS-RT feed entity is published to the bus GTFS-RT topic with timestamp from "2018-10-23T14:24:30.000Z"
    And vehicle position:
      | latitude    | longitude  | bearing | speed              | odometer  |
      | -36.9105233 | 174.680465 | 94      | 16.666666666666668 | 201534366 |
    And vehicle details:
      | id   | label   | licensePlate |
      | 4563 | BT 1234 | ENE46        |
    And trip descriptor:
      | tripId     | routeId | startDate | startTime |
      | 1084005208 | 25      | 20181024  | 02:20:00  |

  Scenario: Passenger count adapter sends passenger count event and adapter should cache it and apply to VP
    Given a location event with UTC timestamp "2018-10-23T14:24:30.000Z"
    And location data:
      | latitude    | longitude  | heading | speed |
      | -36.9105233 | 174.680465 | 94      | 60    |
    And event data:
      | odometer  |
      | 201534366 |
    And remote data:
      | externalId | remoteName |
      | 4563       | name       |
    And passenger count event:
      | vehicleId  | tripId   | serviceDate  | startTime  | occupancyStatus |
      | 4563       | 5209     | 20191010     | 02:20:00   | FULL            |
    And there is cached trip for the same vehicle:
			| vehicleId | tripId | routeId | departureTime | serviceDate |
			| 4563      | 5209   | 25      | 02:20:00      | 20191010    |
    When the passenger count event is published to the passenger count topic
    And the smartrak event is published to the raw data smartrak topic
    Then a GTFS-RT feed entity is published to the bus GTFS-RT topic with timestamp from "2018-10-23T14:24:30.000Z"
    And occupancy level is "FULL"

  Scenario Outline: Nearest trip udapte attached to VP when Smartrak event is received (copy trips)
    Given a Smartrak serial event with serialData:
      | tripId     | startAt       |
      | 1084005208 | 1531146000000 |
    And message data:
      | timestamp    |
      | <event time> |
    And remote data:
      | externalId | remoteName |
      | 4563       | name       |
    And trip management result:
      | tripId     | routeId | departureTime | serviceDate | status      |
      | 1084005208 | 25      | 02:30:00      | 20181010    | COMPLETED   |
      | 1084005208 | 25      | 02:20:00      | 20181010    | IN_PROGRESS |
      | 1084005208 | 25      | 23:30:00      | 20181011    | NOT_STARTED |
      | 1084005208 | 25      | 25:30:00      | 20181011    | COMPLETED   |
      | 1084005208 | 25      | 02:20:00      | 20181012    | NOT_STARTED |
    And a location event with UTC timestamp "<event time>"
    And location data:
      | latitude    | longitude  | heading | speed |
      | -36.9105233 | 174.680465 | 94      | 60    |
    And event data:
      | odometer  |
      | 201534366 |
    And remote data:
      | externalId | remoteName |
      | 4563       | name       |
    When the smartrak event is published to the raw data smartrak topic
    Then a GTFS-RT feed entity is published to the bus GTFS-RT topic with timestamp from "<event time>"
    And vehicle position:
      | latitude    | longitude  | bearing | speed              | odometer  |
      | -36.9105233 | 174.680465 | 94      | 16.666666666666668 | 201534366 |
    And vehicle details:
      | id   | label   | licensePlate |
      | 4563 | BT 1234 | ENE46        |
    And trip descriptor:
      | tripId     | routeId | startDate                  | startTime                  |
      | 1084005208 | 25      | <expected trip start date> | <expected trip start time> |

    Examples:
      | event time               | expected trip start time | expected trip start date |
      | 2018-10-09T13:25:30.000Z | 02:30:00                 | 20181010                 |
      | 2018-10-09T13:24:30.000Z | 02:20:00                 | 20181010                 |
      | 2018-10-11T11:10:30.000Z | 23:30:00                 | 20181011                 |
      | 2018-10-11T12:30:00.000Z | 25:30:00                 | 20181011                 |
      | 2018-10-11T13:10:00.000Z | 02:20:00                 | 20181012                 |

  Scenario: Publish Smartrak bus serial event to GTFS realtime Kafka with correct trip attached (trip id changed)
    Given a Smartrak serial event with serialData:
      | tripId     | startAt       |
      | 1084005205 | 1531146000000 |
    And message data:
      | timestamp                |
      | 2018-10-23T13:24:30.000Z |
    And remote data:
      | externalId | remoteName |
      | 4563       | name       |
    And trip management result:
      | tripId     | routeId | departureTime | serviceDate |
      | 1084005205 | 25      | 02:20:00      | 20181024    |
    And a location event with UTC timestamp "2018-10-23T13:24:30.000Z"
    And location data:
      | latitude    | longitude  | heading | speed |
      | -36.9105233 | 174.680465 | 94      | 60    |
    And event data:
      | odometer  |
      | 201534366 |
    And remote data:
      | externalId | remoteName |
      | 4563       | name       |
    And there is cached trip for the same vehicle:
      | vehicleId | tripId     | routeId | departureTime | serviceDate |
      | 4563      | 1084005208 | 25      | 02:20:00      | 20181024    |
    When the smartrak event is published to the raw data smartrak topic
    Then a GTFS-RT feed entity is published to the bus GTFS-RT topic with timestamp from "2018-10-23T13:24:30.000Z"
    And vehicle position:
      | latitude    | longitude  | bearing | speed              | odometer  |
      | -36.9105233 | 174.680465 | 94      | 16.666666666666668 | 201534366 |
    And vehicle details:
      | id   | label   | licensePlate |
      | 4563 | BT 1234 | ENE46        |
    And trip descriptor:
      | tripId     | routeId | startDate | startTime |
      | 1084005205 | 25      | 20181024  | 02:20:00  |

  Scenario: Always attach the first trip it has been signed on
    Given a Smartrak serial event with serialData:
      | tripId     | startAt       |
      | 1084005208 | 1531146000000 |
    And message data:
      | timestamp                |
      | 2018-10-23T13:24:30.000Z |
    And remote data:
      | externalId | remoteName |
      | 4563       | name       |
    And trip management result:
      | tripId     | routeId | departureTime | serviceDate |
      | 1084005208 | 25      | 02:20:00      | 20181024    |
      | 1084005208 | 25      | 02:25:00      | 20181024    |
    And a location event with UTC timestamp "2018-10-23T13:24:30.000Z"
    And location data:
      | latitude    | longitude  | heading | speed |
      | -36.9105233 | 174.680465 | 94      | 60    |
    And event data:
      | odometer  |
      | 201534366 |
    And remote data:
      | externalId | remoteName |
      | 4563       | name       |
    And there is cached trip for the same vehicle:
      | vehicleId | tripId     | routeId | departureTime | serviceDate |
      | 4563      | 1084005208 | 25      | 02:20:00      | 20181024    |
    When the smartrak event is published to the raw data smartrak topic
    Then a GTFS-RT feed entity is published to the bus GTFS-RT topic with timestamp from "2018-10-23T13:24:30.000Z"
    And vehicle position:
      | latitude    | longitude  | bearing | speed              | odometer  |
      | -36.9105233 | 174.680465 | 94      | 16.666666666666668 | 201534366 |
    And vehicle details:
      | id   | label   | licensePlate |
      | 4563 | BT 1234 | ENE46        |
    And trip descriptor:
      | tripId     | routeId | startDate | startTime |
      | 1084005208 | 25      | 20181024  | 02:20:00  |

  Scenario Outline: Do not attach trip descriptor if it is out of trip duration
    Given a location event with UTC timestamp "<location event time>"
    And location data:
      | latitude    | longitude  | heading | speed |
      | -36.9105233 | 174.680465 | 94      | 60    |
    And event data:
      | odometer  |
      | 201534366 |
    And remote data:
      | externalId | remoteName |
      | 4563       | name       |
    And there is cached trip for the same vehicle:
      | vehicleId | tripId     | routeId | departureTime     | serviceDate | endTime         |
      | 4563      | 1084005208 | 25      | <trip start time> | 20181024    | <trip end time> |
    And vehicle "4563" sign on at "<sign on time>"
    When the smartrak event is published to the raw data smartrak topic
    Then a GTFS-RT feed entity is published to the bus GTFS-RT topic with timestamp from "<location event time>"
    And vehicle position:
      | latitude    | longitude  | bearing | speed              | odometer  |
      | -36.9105233 | 174.680465 | 94      | 16.666666666666668 | 201534366 |
    And vehicle details:
      | id   | label   | licensePlate |
      | 4563 | BT 1234 | ENE46        |
    And no trip descriptor

    Examples:
      | location event time      | sign on time             | trip end time | trip start time |
      | 2018-10-23T14:25:30.000Z | 2018-10-23T13:20:00.000Z | 02:25:00      | 02:20:00        |
      | 2018-10-23T14:30:01.000Z | 2018-10-23T13:25:00.000Z | 02:25:00      | 02:20:00        |
      | 2018-10-23T14:15:01.000Z | 2018-10-23T13:10:00.000Z | 02:25:00      | 02:20:00        |

  Scenario: Don't publish a location event when the topic is smartrak but vehicle tag is CAF
    Given a location event with UTC timestamp "2018-10-23T14:24:30.000Z"
    And location data:
      | latitude    | longitude  | heading | speed |
      | -36.9105233 | 174.680465 | 94      | 60    |
    And event data:
      | odometer  |
      | 201534366 |
    And remote data:
      | externalId | remoteName |
      | 5001       | name       |
    When the smartrak event is published to the raw data smartrak topic
    Then it is not published to the GTFS realtime topic

  Scenario: Don't publish a location event when the topic is caf but vehicle tag is Smartrak
    Given a location event with UTC timestamp "2018-10-23T14:24:30.000Z"
    And location data:
      | latitude    | longitude  | heading | speed |
      | -36.9105233 | 174.680465 | 94      | 60    |
    And event data:
      | odometer  |
      | 201534366 |
    And remote data:
      | externalId | remoteName |
      | 5002       | name       |
    When the smartrak event is published to the raw data caf topic
    Then it is not published to the GTFS realtime topic