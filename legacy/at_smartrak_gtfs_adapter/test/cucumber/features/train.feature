
Feature: Publish a Smartrak EMU GTFS-RT location feed to Kafka
		When a Smartrak train event is published to the raw data topic,
	a corresponding GTFS RT feed is published to the GTFS RT topic.

	Scenario: Publish EMU GTFS-RT train events
		Given a location event with UTC timestamp "2017-10-18T22:55:07.000Z"
		And location data:
			| latitude    | longitude  | heading | speed | gpsAccuracy |
			| -36.9105233 | 174.680465 | 74      | 60    | 1           |
		And event data:
			| odometer  |
			| 201534366 |
		And remote data:
			| remoteId | remoteName  | externalId |
			| 1234     | AM999 train | AM999      |
		When the smartrak event is published to the raw data smartrak topic
		Then a GTFS-RT feed entity is published to the train GTFS-RT topic with timestamp from "2017-10-18T22:55:07.000Z"
		And vehicle position:
			| latitude    | longitude  | bearing | speed              | odometer  |
			| -36.9105233 | 174.680465 | 74      | 16.666666666666668 | 201534366 |
		And vehicle details:
			| id    | label          |
			| 98765 | AMP        999 |

	Scenario: Publish CAF train GTFS realtime vehicle position
		Given a location event with UTC timestamp "2017-10-18T22:55:07.000Z"
		And location data:
			| latitude    | longitude  | heading | speed | gpsAccuracy |
			| -36.9105233 | 174.680465 | 74      | 60    | 1           |
		And event data:
			| odometer  |
			| 201534366 |
		And remote data:
			| remoteId | remoteName  | externalId |
			| 1234     | AM666 train | AM666      |
		When the smartrak event is published to the raw data caf topic
		Then a GTFS-RT feed entity is published to the train GTFS-RT topic with timestamp from "2017-10-18T22:55:07.000Z"
		And vehicle position:
			| latitude    | longitude  | bearing | speed              | odometer  |
			| -36.9105233 | 174.680465 | 74      | 16.666666666666668 | 201534366 |
		And vehicle details:
			| id    | label          |
			| 43210 | AMP        666 |

	Scenario: An innaccurate location event is not published
		Given a location event with UTC timestamp "2017-10-18T22:55:07.000Z"
		And location data:
			| latitude    | longitude  | heading | speed | gpsAccuracy |
			| -36.9105233 | 174.680465 | 74      | 60    | -1          |
		And event data:
			| odometer  |
			| 201534366 |
		And remote data:
			| remoteId | remoteName  |
			| 1234     | AM999 train |
		When the smartrak event is published to the raw data smartrak topic
		Then it is not published to the GTFS realtime topic

	Scenario: Publish Smartrak train location event to GTFS realtime Kafka with trip descriptor (from cache)
		Given a location event with UTC timestamp "2019-10-09T13:20:00.000Z"
		And location data:
			| latitude    | longitude  | heading | speed | gpsAccuracy |
			| -36.9105233 | 174.680465 | 74      | 60    | 1           |
		And event data:
			| odometer  |
			| 201534366 |
		And remote data:
			| remoteId | remoteName  | externalId |
			| 1234     | AM999 train | AM999      |
		And block api response:
			| tripId | startTime | serviceDate | vehicleId |
			| 5209   | 02:20:00  | 20191010    | 98765     |
		And trip management result:
			| tripId | routeId | departureTime | serviceDate |
			| 5209   | 25      | 02:20:00      | 20191010    |
		And there is cached trip for the same vehicle:
			| vehicleId | tripId | routeId | departureTime | serviceDate |
			| 98765     | 5209   | 25      | 02:20:00      | 20191010    |
		When the smartrak event is published to the raw data smartrak topic
		Then a GTFS-RT feed entity is published to the bus GTFS-RT topic with timestamp from "2019-10-09T13:20:00.000Z"
		And vehicle position:
			| latitude    | longitude  | bearing | speed              | odometer  |
			| -36.9105233 | 174.680465 | 74      | 16.666666666666668 | 201534366 |
		And vehicle details:
			| id    | label          |
			| 98765 | AMP        999 |
		And trip descriptor:
			| tripId | routeId | startDate | startTime |
			| 5209   | 25      | 20191010  | 02:20:00  |

	Scenario: Publish Smartrak train location event to GTFS realtime Kafka with trip descriptor (no cache)
		Given a location event with UTC timestamp "2019-10-09T13:20:00.000Z"
		And location data:
			| latitude    | longitude  | heading | speed | gpsAccuracy |
			| -36.9105233 | 174.680465 | 74      | 60    | 1           |
		And event data:
			| odometer  |
			| 201534366 |
		And remote data:
			| remoteId | remoteName  | externalId |
			| 1234     | AM999 train | AM999      |
		And block api response:
			| tripId | startTime | serviceDate | vehicleId |
			| 5209   | 02:20:00  | 20191010    | 98765     |
			| 5209   | 02:20:00  | 20191010    | 98766     |
		And trip management result:
			| tripId | routeId | departureTime | serviceDate |
			| 5209   | 25      | 02:20:00      | 20191010    |
		When the smartrak event is published to the raw data smartrak topic
		Then a GTFS-RT feed entity is published to the bus GTFS-RT topic with timestamp from "2019-10-09T13:20:00.000Z"
		And vehicle position:
			| latitude    | longitude  | bearing | speed              | odometer  |
			| -36.9105233 | 174.680465 | 74      | 16.666666666666668 | 201534366 |
		And vehicle details:
			| id    | label          |
			| 98765 | AMP        999 |
		And trip descriptor:
			| tripId | routeId | startDate | startTime |
			| 5209   | 25      | 20191010  | 02:20:00  |

	Scenario: Publish Smartrak train location event to GTFS realtime Kafka without trip descriptor if vehicle is not main Cargo
		Given a location event with UTC timestamp "2019-10-09T13:24:30.000Z"
		And location data:
			| latitude    | longitude  | heading | speed | gpsAccuracy |
			| -36.9105233 | 174.680465 | 74      | 60    | 1           |
		And event data:
			| odometer  |
			| 201534366 |
		And remote data:
			| remoteId | remoteName  | externalId |
			| 1234     | AM999 train | AM999      |
		And block api response:
			| tripId | startTime | serviceDate | vehicleId |
			| 5209   | 02:20:00  | 20191010    | 98766     |
			| 5209   | 02:20:00  | 20191010    | 98765     |
		And trip management result:
			| tripId | routeId | departureTime | serviceDate |
			| 5209   | 25      | 02:20:00      | 20191010    |
		When the smartrak event is published to the raw data smartrak topic
		Then a GTFS-RT feed entity is published to the bus GTFS-RT topic with timestamp from "2019-10-09T13:24:30.000Z"
		And vehicle position:
			| latitude    | longitude  | bearing | speed              | odometer  |
			| -36.9105233 | 174.680465 | 74      | 16.666666666666668 | 201534366 |
		And vehicle details:
			| id    | label          |
			| 98765 | AMP        999 |
		And no trip descriptor

	Scenario: Publish Smartrak train location event to GTFS realtime Kafka without trip descriptor if block mgt returns empty
		Given a location event with UTC timestamp "2019-10-09T13:24:30.000Z"
		And location data:
			| latitude    | longitude  | heading | speed | gpsAccuracy |
			| -36.9105233 | 174.680465 | 74      | 60    | 1           |
		And event data:
			| odometer  |
			| 201534366 |
		And remote data:
			| remoteId | remoteName  | externalId |
			| 1234     | AM999 train | AM999      |
		And trip management result:
			| tripId | routeId | departureTime | serviceDate |
			| 5209   | 25      | 02:20:00      | 20191010    |
		And there is cached trip for the same vehicle:
			| vehicleId | tripId | routeId | departureTime | serviceDate |
			| 98765     | 5209   | 25      | 02:20:00      | 20191010    |
		When the smartrak event is published to the raw data smartrak topic
		Then a GTFS-RT feed entity is published to the bus GTFS-RT topic with timestamp from "2019-10-09T13:24:30.000Z"
		And vehicle position:
			| latitude    | longitude  | bearing | speed              | odometer  |
			| -36.9105233 | 174.680465 | 74      | 16.666666666666668 | 201534366 |
		And vehicle details:
			| id    | label          |
			| 98765 | AMP        999 |
		And no trip descriptor

	Scenario: Publish Smartrak train location event to GTFS realtime Kafka without trip descriptor if trip mgt returns empty
		Given a location event with UTC timestamp "2019-10-09T13:24:30.000Z"
		And location data:
			| latitude    | longitude  | heading | speed | gpsAccuracy |
			| -36.9105233 | 174.680465 | 74      | 60    | 1           |
		And event data:
			| odometer  |
			| 201534366 |
		And remote data:
			| remoteId | remoteName  | externalId |
			| 1234     | AM999 train | AM999      |
		And block api response:
			| tripId | startTime | serviceDate | vehicleId |
			| 5209   | 02:20:00  | 20191010    | 98765     |
		And there is cached trip for the same vehicle:
			| vehicleId | tripId | routeId | departureTime | serviceDate |
			| 98765     | 5208   | 25      | 02:20:00      | 20191010    |
		When the smartrak event is published to the raw data smartrak topic
		Then a GTFS-RT feed entity is published to the bus GTFS-RT topic with timestamp from "2019-10-09T13:24:30.000Z"
		And vehicle position:
			| latitude    | longitude  | bearing | speed              | odometer  |
			| -36.9105233 | 174.680465 | 74      | 16.666666666666668 | 201534366 |
		And vehicle details:
			| id    | label          |
			| 98765 | AMP        999 |
		And no trip descriptor

	Scenario: Should set correct occupancy status based on the passenger count event
		Given a location event with UTC timestamp "2019-10-09T13:20:00.000Z"
		And location data:
			| latitude    | longitude  | heading | speed | gpsAccuracy |
			| -36.9105233 | 174.680465 | 74      | 60    | 1           |
		And event data:
			| odometer  |
			| 201534366 |
		And remote data:
			| remoteId | remoteName  | externalId |
			| 1234     | AM999 train | AM999      |
		And there is cached trip for the same vehicle:
			| vehicleId | tripId | routeId | departureTime | serviceDate |
			| 98765     | 5209   | 25      | 02:20:00      | 20191010    |
		And block api response:
			| tripId | startTime | serviceDate | vehicleId |
			| 5209   | 02:20:00  | 20191010    | 98765     |
		And passenger count event:
			| vehicleId  | tripId   | serviceDate  | startTime  | occupancyStatus |
			| 98765      | 5209     | 20191010     | 02:20:00   | EMPTY           |
		When the passenger count event is published to the passenger count topic
		And the smartrak event is published to the raw data smartrak topic
		Then a GTFS-RT feed entity is published to the bus GTFS-RT topic with timestamp from "2019-10-09T13:20:00.000Z"
		And occupancy level is "EMPTY"

	Scenario: Should send message to Dead Reckoning topic if there no location data at all but contains odometer data
		Given a location event without location data and with UTC timestamp "2019-10-09T13:20:00.000Z"
		And event data:
			| odometer  |
			| 123456    |
		And remote data:
			| remoteId | remoteName  | externalId |
			| 1234     | AM666 train | AM666      |
		And there is cached trip for the same vehicle:
			| vehicleId | tripId | routeId | departureTime | serviceDate |
			| 43210     | 5209   | 25      | 02:20:00      | 20191010    |
		And block api response:
			| tripId | startTime | serviceDate | vehicleId |
			| 5209   | 02:20:00  | 20191010    | 43210     |
		When the smartrak event is published to the raw data caf topic
		Then a message is published to the Dead Reckoning topic with odometer value "123456"

	Scenario: Should send message to Dead Reckoning topic if there no latitude and longitude but contains odometer data in location data
		Given a location event without latitude and longitude with UTC timestamp "2019-10-09T13:20:00.000Z"
		And remote data:
			| remoteId | remoteName  | externalId |
			| 1234     | AM666 train | AM666      |
		And there is cached trip for the same vehicle:
			| vehicleId | tripId | routeId | departureTime | serviceDate |
			| 43210     | 5209   | 25      | 02:20:00      | 20191010    |
		And block api response:
			| tripId | startTime | serviceDate | vehicleId |
			| 5209   | 02:20:00  | 20191010    | 43210     |
		When the smartrak event is published to the raw data caf topic
		Then a message is published to the Dead Reckoning topic with odometer value "123456"
