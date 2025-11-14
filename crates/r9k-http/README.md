
# R9K HTTP Connector

The R9K HTTP connector receives R9K data and posts to the Confluent topic `{env}-realtime-r9k.v1`. 

R9K data is received from track-side sensors that are triggered when a train passes. This position
data is used to help improve train location information when in underground stations (where GPS is
not available).

