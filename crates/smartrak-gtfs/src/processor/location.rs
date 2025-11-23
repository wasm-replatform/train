use std::env;

use chrono::{Duration, NaiveDate, TimeZone};
use chrono_tz::Tz;
use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::models::{
    BlockInstance, DeadReckoningMessage, FeedEntity, PassengerCountEvent, Position, PositionDr,
    SmartrakEvent, TripDescriptor, TripInstance, VehicleDescriptor, VehicleDr, VehicleInfo,
    VehiclePosition,
};
use crate::{Provider, StateStore, block_mgt, fleet, trip};

const TTL_TRIP_TRAIN: Duration = Duration::seconds(3 * 60 * 60);
const TTL_SIGN_ON: Duration = Duration::seconds(24 * 60 * 60);
const TIMEZONE: Tz = chrono_tz::Pacific::Auckland;

fn env_i64(key: &str, default: i64) -> i64 {
    env::var(key).ok().and_then(|value| value.parse::<i64>().ok()).unwrap_or(default)
}

const fn duration_secs(duration: Duration) -> u64 {
    duration.num_seconds().unsigned_abs()
}

pub enum LocationOutcome {
    VehiclePosition(FeedEntity),
    DeadReckoning(DeadReckoningMessage),
}

/// Attempts to resolve vehicle metadata from the Fleet API using multiple heuristics.
///
/// # Errors
///
/// Returns an error when the Fleet provider cannot be queried.
pub async fn resolve_vehicle(
    provider: &impl Provider, vehicle_id_or_label: &str,
) -> Result<Option<VehicleInfo>> {
    let candidate = vehicle_id_or_label.trim();

    if candidate.is_empty() {
        return Ok(None);
    }

    fleet::get_vehicle_by_id_or_label(provider, candidate).await.map_err(Error::from)
}

/// Processes a Smartrak Kafka payload and emits outbound messages when applicable.
///
/// # Errors
///
/// Returns an error when the incoming payload cannot be parsed or when domain logic
/// encounters an unrecoverable condition.
pub async fn process_location(
    provider: &impl Provider, event: &SmartrakEvent, vehicle: &VehicleInfo,
) -> Result<Option<LocationOutcome>> {
    if !is_location_event_valid(event) {
        return Ok(None);
    }

    let _vehicle_identifier = event.vehicle_identifier().ok_or_else(|| {
        Error::ProcessingError(format!(
            "remoteData.externalId {}",
            event.remote_data.as_ref().and_then(|rd| rd.external_id.clone()).unwrap_or_default()
        ))
    })?;

    //let _guard = lock(&format!("location:{vehicle_identifier}")).await;

    let event_timestamp = event
        .timestamp_unix()
        .ok_or_else(|| Error::ProcessingError(event.message_data.timestamp.clone()))?;

    if vehicle.vehicle_type.is_train() {
        let allocation =
            block_mgt::get_allocation_by_vehicle(provider, &vehicle.id, event_timestamp).await?;
        assign_train_to_trip(provider, vehicle, allocation, event_timestamp).await?;
    }
    let trip_instance = current_trip_instance(provider, &vehicle.id, event_timestamp).await?;
    let trip_descriptor = trip_instance.as_ref().map(TripInstance::to_trip_descriptor);
    let odometer = event.location_data.odometer.or(event.event_data.odometer);

    if (event.location_data.latitude.is_none() || event.location_data.longitude.is_none())
        && let (Some(odometer_value), Some(descriptor)) = (odometer, trip_descriptor.clone())
    {
        let dr_message = DeadReckoningMessage {
            id: Uuid::new_v4().to_string(),
            received_at: event_timestamp,
            position: PositionDr { odometer: odometer_value },
            trip: descriptor,
            vehicle: VehicleDr { id: vehicle.id.clone() },
        };
        return Ok(Some(LocationOutcome::DeadReckoning(dr_message)));
    }

    let descriptor = VehicleDescriptor {
        id: vehicle.id.clone(),
        label: vehicle.label.clone(),
        license_plate: vehicle.registration.clone(),
    };

    let occupancy_status = if let Some(trip) = trip_descriptor.as_ref() {
        get_occupancy_status(provider, vehicle, trip).await?
    } else {
        None
    };

    let position = Position {
        latitude: event.location_data.latitude,
        longitude: event.location_data.longitude,
        bearing: event.location_data.heading,
        speed: event.location_data.speed.map(|value| value * 1000.0 / 3600.0),
        odometer,
    };

    let vehicle_position = VehiclePosition {
        position: Some(position),
        trip: trip_descriptor,
        vehicle: Some(descriptor),
        occupancy_status,
        timestamp: event_timestamp,
    };

    let entity = FeedEntity { id: vehicle.id.clone(), vehicle: Some(vehicle_position) };
    Ok(Some(LocationOutcome::VehiclePosition(entity)))
}

fn deserialize_optional<T>(bytes: Option<&[u8]>) -> Option<T>
where
    T: DeserializeOwned,
{
    bytes.and_then(|raw| serde_json::from_slice::<T>(raw).ok())
}

fn is_location_event_valid(event: &SmartrakEvent) -> bool {
    event.remote_data.is_some() && event.location_data.gps_accuracy >= 0.0
}

async fn assign_train_to_trip(
    provider: &impl Provider, vehicle: &VehicleInfo, allocation: Option<BlockInstance>,
    event_timestamp: i64,
) -> Result<()> {
    let trip_key = format!("smartrakGtfs:trip:vehicle:{}", &vehicle.id);
    let sign_on_key = format!("smartrakGtfs:vehicle:signOn:{}", &vehicle.id);

    let Some(block_instance) = allocation else {
        StateStore::delete(provider, &sign_on_key).await?;
        StateStore::delete(provider, &trip_key).await?;
        return Ok(());
    };

    if block_instance.has_error() {
        return Ok(());
    }

    if block_instance.vehicle_ids.first() != Some(&vehicle.id) {
        StateStore::delete(provider, &sign_on_key).await?;
        StateStore::delete(provider, &trip_key).await?;
        return Ok(());
    }

    let previous_bytes = StateStore::get(provider, &trip_key).await?;

    if let Some(previous) = deserialize_optional::<TripInstance>(previous_bytes.as_deref()) {
        let same_trip = previous.trip_id == block_instance.trip_id
            && previous.start_time == block_instance.start_time
            && previous.service_date == block_instance.service_date;
        if same_trip {
            return Ok(());
        }
    }

    let new_trip = trip::get_trip_instance(
        provider,
        &block_instance.trip_id,
        &block_instance.service_date,
        &block_instance.start_time,
    )
    .await?;

    let Some(trip) = new_trip else {
        StateStore::delete(provider, &sign_on_key).await?;
        StateStore::delete(provider, &trip_key).await?;
        return Ok(());
    };

    if trip.has_error() {
        return Ok(());
    }

    let trip_bytes =
        serde_json::to_vec(&trip).map_err(|e| Error::InvalidFormat(e.to_string()))?;
    StateStore::set(provider, &trip_key, &trip_bytes, Some(duration_secs(TTL_TRIP_TRAIN))).await?;

    let timestamp_bytes =
        serde_json::to_vec(&event_timestamp).map_err(|e| Error::InvalidTimestamp(e.to_string()))?;
    StateStore::set(provider, &sign_on_key, &timestamp_bytes, Some(duration_secs(TTL_SIGN_ON)))
        .await?;
    Ok(())
}

async fn current_trip_instance(
    provider: &impl Provider, vehicle_id: &str, event_timestamp: i64,
) -> Result<Option<TripInstance>> {
    let trip_key = format!("smartrakGtfs:trip:vehicle:{}", &vehicle_id);
    let sign_on_key = format!("smartrakGtfs:vehicle:signOn:{}", &vehicle_id);
    let bytes = StateStore::get(provider, &trip_key).await?;

    if let Some(instance) = deserialize_optional::<TripInstance>(bytes.as_deref()) {
        let sign_on_bytes = StateStore::get(provider, &sign_on_key).await?;
        let sign_on = deserialize_optional::<i64>(sign_on_bytes.as_deref());
        if let (Some(sign_on_ts), Some(start), Some(end)) = (
            sign_on,
            time_to_timestamp(&instance.service_date, &instance.start_time, TIMEZONE),
            time_to_timestamp(&instance.service_date, &instance.end_time, TIMEZONE),
        ) {
            let duration = end - start
                + Duration::seconds(env_i64("TRIP_DURATION_BUFFER", 3_600)).num_seconds();
            if event_timestamp - duration > sign_on_ts {
                StateStore::delete(provider, &sign_on_key).await?;
                StateStore::delete(provider, &trip_key).await?;
                return Ok(None);
            }
        }
        return Ok(Some(instance));
    }

    Ok(None)
}

async fn get_occupancy_status(
    provider: &impl Provider, vehicle: &VehicleInfo, trip: &TripDescriptor,
) -> Result<Option<String>> {
    let Some(start_date) = trip.start_date.as_ref() else {
        return Ok(None);
    };

    let Some(start_time) = trip.start_time.as_ref() else {
        return Ok(None);
    };

    let key = format!(
        "smartrakGtfs:passengerCountEvent:{}:{}:{}:{}",
        &vehicle.id, &trip.trip_id, start_date, start_time
    );
    let bytes = StateStore::get(provider, &key).await?;
    let passenger_event: Option<PassengerCountEvent> = deserialize_optional(bytes.as_deref());
    Ok(passenger_event.and_then(|event| event.occupancy_status))
}

fn time_to_timestamp(date: &str, time: &str, tz: Tz) -> Option<i64> {
    if date.is_empty() || time.is_empty() {
        return None;
    }

    let date = NaiveDate::parse_from_str(date, "%Y%m%d").ok()?;
    let parts: Vec<_> = time.split(':').collect();
    if parts.len() != 3 {
        return None;
    }

    let hours: i64 = parts[0].parse().ok()?;
    let minutes: i64 = parts[1].parse().ok()?;
    let seconds: i64 = parts[2].parse().ok()?;
    let base = date.and_hms_opt(0, 0, 0)?;
    let datetime = tz.from_local_datetime(&base).single()?;
    Some((datetime + Duration::seconds(hours * 3_600 + minutes * 60 + seconds)).timestamp())
}
