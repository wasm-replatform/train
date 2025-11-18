use crate::error::Result;
use crate::models::PassengerCountEvent;
use crate::{Provider, StateStore};

const TTL_PASSENGER_COUNT: u64 = 3 * 60 * 60; // 3 hours

/// Persists the passenger count event in the state store with a limited TTL.
///
/// # Errors
///
/// Returns an error when serialization fails or the provider's state store is
/// unavailable.
pub async fn process_passenger_count(
    provider: &impl Provider, event: &PassengerCountEvent,
) -> Result<()> {
    let key = format!(
        "{}:{}:{}:{}:{}",
        "smartrakGtfs:passengerCountEvent",
        &event.vehicle.id,
        &event.trip.trip_id,
        &event.trip.start_date,
        &event.trip.start_time
    );

    let bytes = serde_json::to_vec(event)?;
    StateStore::set(provider, &key, &bytes, Some(TTL_PASSENGER_COUNT)).await?;
    Ok(())
}
