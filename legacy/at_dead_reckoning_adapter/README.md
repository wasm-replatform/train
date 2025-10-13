# Dead Reckoning Adapter #

This component consumes messages from realtime-gtfs-vp topic and caches vehicle data that has tripId and odometer values. The component also listens to the dead-reckoning topic which supplies the vehicle id, tripid and odometer reading for messages received without GPS values. When consuming the dead reckoning topic, if a cached VP exists for the vehicle then the difference in odometer readings between the cached VP and the dead reckoning message is used to calculate the distance travelled along the trip shape to create a new VP message with the new location found on the shape.

