use std::sync::Arc;

use anyhow::Result;
use tracing::debug;

use crate::cache::CacheRepository;
use crate::config::{CACHE_TTL_PASSENGER_COUNT, Config};
use crate::model::events::PassengerCountEvent;
use crate::model::gtfs::OccupancyStatus;
use crate::model::trip::TripDescriptor;

#[derive(Debug, Clone)]
// Port of legacy/at_smartrak_gtfs_adapter/src/processors/passenger-count.ts.
pub struct PassengerCountProcessor {
    config: Arc<Config>,
    cache: Arc<CacheRepository>,
}

impl PassengerCountProcessor {
    pub fn new(config: Arc<Config>, cache: Arc<CacheRepository>) -> Self {
        Self { config, cache }
    }

    pub async fn process(&self, event: PassengerCountEvent) -> Result<()> {
        let key = self.config.passenger_count_key(
            &event.vehicle.id,
            &event.trip.trip_id,
            &event.trip.start_date,
            &event.trip.start_time,
        );

        debug!(redis_key = %key, occupancy = ?event.occupancy_status, "storing passenger count event");
        self.cache.set_json_ex(&key, CACHE_TTL_PASSENGER_COUNT, &event)?;
        Ok(())
    }

    pub async fn lookup_occupancy(
        cache: &CacheRepository, config: &Config, vehicle_id: &str, trip: &TripDescriptor,
    ) -> Result<Option<OccupancyStatus>> {
        let key = config.passenger_count_key(
            vehicle_id,
            trip.trip_id(),
            trip.start_date(),
            trip.start_time(),
        );
        let Some(event) = cache.get_json::<PassengerCountEvent>(&key)? else {
            return Ok(None);
        };

        let occupancy = event.occupancy_status.as_deref().and_then(map_occupancy_status);
        Ok(occupancy)
    }
}

fn map_occupancy_status(value: &str) -> Option<OccupancyStatus> {
    match value {
        "EMPTY" => Some(OccupancyStatus::Empty),
        "MANY_SEATS_AVAILABLE" => Some(OccupancyStatus::ManySeatsAvailable),
        "FEW_SEATS_AVAILABLE" => Some(OccupancyStatus::FewSeatsAvailable),
        "STANDING_ROOM_ONLY" => Some(OccupancyStatus::StandingRoomOnly),
        "CRUSHED_STANDING_ROOM_ONLY" => Some(OccupancyStatus::CrushedStandingRoomOnly),
        "FULL" => Some(OccupancyStatus::Full),
        "NOT_ACCEPTING_PASSENGERS" => Some(OccupancyStatus::NotAcceptingPassengers),
        _ => None,
    }
}
