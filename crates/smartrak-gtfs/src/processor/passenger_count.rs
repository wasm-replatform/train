use chrono::Duration;

use crate::error::Result;
use crate::models::PassengerCountEvent;
use crate::provider::{Provider, StateStore};

const TTL_PASSENGER_COUNT: u64 = Duration::hours(3).num_seconds() as u64; // 3 hours

pub async fn process_passenger_count(provider: &impl Provider, event: &PassengerCountEvent) -> Result<()> {
    let key = 
    format!(
        "{}:{}:{}:{}:{}",
        "smartrakGtfs:passengerCountEvent", &event.vehicle.id, &event.trip.trip_id, &event.trip.start_date, &event.trip.start_time
    );
 
    let bytes = serde_json::to_vec(event)?;
    StateStore::set(provider, &key, &bytes, Some(TTL_PASSENGER_COUNT)).await?;
    Ok(())
}
