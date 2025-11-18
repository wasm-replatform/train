use std::env;

use chrono::Utc;
use tracing::{debug, warn};

use crate::error::{Error, Result};
use crate::models::{DecodedSerialData, SmartrakEvent, TripInstance};
use crate::{Provider, StateStore, trip};

const TTL_TRIP_SERIAL_SECS: u64 = 4 * 60 * 60;
const TTL_SIGN_ON_SECS: u64 = 24 * 60 * 60;
const TTL_SERIAL_TIMESTAMP_SECS: u64 = 24 * 60 * 60;

fn env_i64(key: &str, default: i64) -> i64 {
    env::var(key).ok().and_then(|value| value.parse::<i64>().ok()).unwrap_or(default)
}

/// Processes SmarTrak serial data events, updating allocations and in-memory state.
///
/// # Errors
///
/// Returns an error when required event fields are missing, cached state cannot
/// be accessed, or downstream lookups fail.
pub async fn process_serial_data(provider: &impl Provider, event: &SmartrakEvent) -> Result<()> {
    if !is_serial_event_valid(event) {
        return Ok(());
    }

    let Some(remote) = event.remote_data.as_ref() else {
        return Err(Error::MissingField("remoteData".to_string()));
    };

    let Some(vehicle_id) = remote.external_id.as_deref() else {
        return Err(Error::MissingField("remoteData.externalId".to_string()));
    };

    let event_timestamp = event
        .timestamp_unix()
        .ok_or_else(|| Error::InvalidTimestamp(event.message_data.timestamp.clone()))?;

    let Some(serial_data) = event.serial_data.as_ref() else {
        return Err(Error::MissingField("serialData".to_string()));
    };

    let Some(decoded) = serial_data.decoded_serial_data.as_ref() else {
        return Err(Error::MissingField("serialData.decodedSerialData".to_string()));
    };

    //let _guard = lock(&format!("serialData:{vehicle_id}")).await;

    if mark_serial_timestamp(provider, vehicle_id, event_timestamp).await? {
        warn!(vehicle_id, "received older serial data event");
        return Ok(());
    }

    allocate_vehicle_to_trip(provider, vehicle_id, decoded, event_timestamp).await
}

async fn mark_serial_timestamp(
    provider: &impl Provider, vehicle_id: &str, timestamp: i64,
) -> Result<bool> {
    let key = format!("smartrakGtfs:serialTimestamp:{}", &vehicle_id);
    let previous_bytes = StateStore::get(provider, &key).await?;
    if serde_json::from_value::<i64>(previous_bytes.into())
        .is_ok_and(|previous| previous >= timestamp)
    {
        return Ok(true);
    }

    let timestamp_bytes =
        serde_json::to_vec(&timestamp).map_err(|e| Error::InvalidTimestamp(e.to_string()))?;
    StateStore::set(provider, &key, &timestamp_bytes, Some(TTL_SERIAL_TIMESTAMP_SECS)).await?;
    Ok(false)
}

fn is_serial_event_valid(event: &SmartrakEvent) -> bool {
    let Some(remote) = event.remote_data.as_ref() else {
        return false;
    };

    let has_vehicle = remote.external_id.is_some();
    let has_serial =
        event.serial_data.as_ref().and_then(|serial| serial.decoded_serial_data.as_ref()).is_some();

    if !has_vehicle || !has_serial {
        return false;
    }

    let Some(timestamp) = event.timestamp_unix() else {
        return false;
    };

    let future_delta = timestamp - Utc::now().timestamp();
    if future_delta > env_i64("SERIAL_DATA_FILTER_THRESHOLD", 900) {
        warn!(future_delta, "serial data event rejected because it is from the future");
        return false;
    }

    true
}

async fn allocate_vehicle_to_trip(
    provider: &impl Provider, vehicle_id: &str, decoded: &DecodedSerialData, event_timestamp: i64,
) -> Result<()> {
    let trip_key = format!("smartrakGtfs:trip:vehicle:{}", &vehicle_id);
    let sign_on_key = format!("smartrakGtfs:vehicle:signOn:{}", &vehicle_id);
    let serial_timestamp_key = format!("smartrakGtfs:serialTimestamp:{}", &vehicle_id);

    let Some(trip_id) = decoded.trip_id.as_deref() else {
        debug!(vehicle_id, "serial data without trip id, clearing state");
        StateStore::delete(provider, &sign_on_key).await?;
        StateStore::delete(provider, &trip_key).await?;
        StateStore::delete(provider, &serial_timestamp_key).await?;
        return Ok(());
    };

    let previous_bytes = StateStore::get(provider, &trip_key).await?;

    if serde_json::from_value::<TripInstance>(previous_bytes.into())
        .is_ok_and(|previous| previous.trip_id == trip_id)
    {
        return Ok(());
    }

    let trip = trip::get_nearest_trip_instance(provider, trip_id, event_timestamp).await?;

    match trip {
        Some(instance) if instance.has_error() => {
            StateStore::delete(provider, &sign_on_key).await?;
            StateStore::delete(provider, &trip_key).await?;
            StateStore::delete(provider, &serial_timestamp_key).await?;
            Ok(())
        }
        Some(instance) => persist_trip(provider, vehicle_id, event_timestamp, instance).await,
        None => {
            StateStore::delete(provider, &sign_on_key).await?;
            StateStore::delete(provider, &trip_key).await?;
            StateStore::delete(provider, &serial_timestamp_key).await?;
            Ok(())
        }
    }
}

async fn persist_trip(
    provider: &impl Provider, vehicle_id: &str, event_timestamp: i64, trip: TripInstance,
) -> Result<()> {
    let trip_key = format!("smartrakGtfs:trip:vehicle:{}", &vehicle_id);
    let sign_on_key = format!("smartrakGtfs:vehicle:signOn:{}", &vehicle_id);

    let trip_bytes =
        serde_json::to_vec(&trip).map_err(|e| Error::InvalidTimestamp(e.to_string()))?;
    StateStore::set(provider, &trip_key, &trip_bytes, Some(TTL_TRIP_SERIAL_SECS)).await?;

    let timestamp_bytes =
        serde_json::to_vec(&event_timestamp).map_err(|e| Error::InvalidTimestamp(e.to_string()))?;
    StateStore::set(provider, &sign_on_key, &timestamp_bytes, Some(TTL_SIGN_ON_SECS)).await?;
    Ok(())
}
