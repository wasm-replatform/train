use anyhow::Context;
use chrono::Utc;
use realtime::bad_request;

use crate::{
    DecodedSerialData, Provider, Result, SmarTrakError, SmarTrakMessage, StateStore, TripInstance,
    trip,
};

const TTL_TRIP_SERIAL_SECS: u64 = 4 * 60 * 60;
const TTL_SIGN_ON_SECS: u64 = 24 * 60 * 60;
const TTL_SERIAL_TIMESTAMP_SECS: u64 = 24 * 60 * 60;

const SERIAL_DATA_THRESHOLD: i64 = 900;

// Processes SmarTrak serial data events, updating allocations and  state.
pub async fn process(message: &SmarTrakMessage, provider: &impl Provider) -> Result<()> {
    let Some(vehicle_id) = message.vehicle_id() else {
        return Err(bad_request!("missing vehicle identifier"));
    };

    // validate timestamp
    let timestamp = message.timestamp()?;

    // is this a future-dated (by 900 secs) timestamp?
    if timestamp > Utc::now().timestamp() + SERIAL_DATA_THRESHOLD {
        return Err(SmarTrakError::BadTime("future-dated serial data message".to_string()).into());
    }

    update_timestamp(provider, timestamp, vehicle_id).await?;

    let Some(serial_data) = message.serial_data.as_ref() else {
        return Err(bad_request!("missing serialData"));
    };
    let Some(decoded) = serial_data.decoded_serial_data.as_ref() else {
        return Err(bad_request!("missing decoded serial data"));
    };

    allocate_vehicle(provider, vehicle_id, decoded, timestamp).await
}

// Updates the timestamp if it is newer than the previously stored timestamp.
async fn update_timestamp(store: &impl StateStore, timestamp: i64, vehicle_id: &str) -> Result<()> {
    let key = format!("smartrakGtfs:serialTimestamp:{vehicle_id}");

    // check previous timestamp
    let previous = StateStore::get(store, &key).await?;
    if serde_json::from_value::<i64>(previous.into()).is_ok_and(|prev| prev >= timestamp) {
        return Err(SmarTrakError::BadTime("outdated serial data message".to_string()).into());
    }

    // store new timestamp
    let value = serde_json::to_vec(&timestamp).context("failed to serialize timestamp")?;
    StateStore::set(store, &key, &value, Some(TTL_SERIAL_TIMESTAMP_SECS)).await?;

    Ok(())
}

async fn allocate_vehicle(
    provider: &impl Provider, vehicle_id: &str, decoded: &DecodedSerialData, event_timestamp: i64,
) -> Result<()> {
    let trip_key = format!("smartrakGtfs:trip:vehicle:{vehicle_id}");
    let sign_on_key = format!("smartrakGtfs:vehicle:signOn:{vehicle_id}");
    let serial_timestamp_key = format!("smartrakGtfs:serialTimestamp:{vehicle_id}");

    let Some(trip_id) = decoded.trip_id.as_deref() else {
        tracing::debug!(vehicle_id, "no trip id found, clearing state");

        StateStore::delete(provider, &sign_on_key).await?;
        StateStore::delete(provider, &trip_key).await?;
        StateStore::delete(provider, &serial_timestamp_key).await?;

        return Ok(());
    };

    let prev_trip = StateStore::get(provider, &trip_key).await?;
    if serde_json::from_value::<TripInstance>(prev_trip.into())
        .is_ok_and(|prev| prev.trip_id == trip_id)
    {
        return Ok(());
    }

    let trip = trip::get_nearest_trip_instance(provider, trip_id, event_timestamp).await?;
    match trip {
        Some(instance) if !instance.has_error() => {
            save_trip(provider, vehicle_id, event_timestamp, instance).await
        }
        _ => {
            StateStore::delete(provider, &sign_on_key).await?;
            StateStore::delete(provider, &trip_key).await?;
            StateStore::delete(provider, &serial_timestamp_key).await?;

            Ok(())
        }
    }
}

async fn save_trip(
    provider: &impl Provider, vehicle_id: &str, event_timestamp: i64, trip: TripInstance,
) -> Result<()> {
    let trip_key = format!("smartrakGtfs:trip:vehicle:{vehicle_id}");
    let sign_on_key = format!("smartrakGtfs:vehicle:signOn:{vehicle_id}");

    let trip_bytes = serde_json::to_vec(&trip).context("failed to serialize trip")?;
    StateStore::set(provider, &trip_key, &trip_bytes, Some(TTL_TRIP_SERIAL_SECS)).await?;

    let timestamp_bytes =
        serde_json::to_vec(&event_timestamp).context("failed to serialize message timestamp")?;
    StateStore::set(provider, &sign_on_key, &timestamp_bytes, Some(TTL_SIGN_ON_SECS)).await?;

    Ok(())
}
