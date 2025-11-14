use std::fmt::{self, Display};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::error::Error;
use crate::provider::StateStore;
use crate::types::{DilaxMessage, Door};

const KEY_OCCUPANCY: &str = "trip:occupancy";
const KEY_VEHICLE_STATE: &str = "apc:vehicleIdState";
const KEY_VEHICLE_ID: &str = "apc:vehicleId";
const KEY_VEHICLE_ID_MIGRATED: &str = "apc:vehicleIdMigrated";
const KEY_TRIPS: &str = "apc:trips";
const KEY_TRIP_INFO: &str = "apc:vehicleTripInfo";

const TTL_APC: u64 = 60 * 60; // 1 hour
const TTL_OCCUPANCY_STATE: u64 = 90 * 60; // 90 minutes
const TTL_VEHICLE_TRIP_INFO: u64 = 48 * 60 * 60; // 48 hours

/// Update the vehicle state with the latest Dilax APC event.
///
/// # Errors
///
/// This function will return an error if there is an issue reading or writing
/// to the state store, or if the event data is malformed.
pub async fn update_vehicle(
    vehicle_id: &str, trip_id: Option<&str>, seating_capacity: i64, total_capacity: i64,
    event: &DilaxMessage, state_store: &impl StateStore,
) -> Result<()> {
    let state_key = format!("{KEY_VEHICLE_STATE}:{vehicle_id}");

    // fetch existing state or create
    let state_prev = state_store.get(&state_key).await?;
    let mut state = if let Some(raw) = &state_prev {
        serde_json::from_slice::<TripState>(raw).unwrap_or_default()
    } else {
        let mut new_state = TripState::default();
        migrate_legacy_keys(vehicle_id, &mut new_state, state_store).await?;
        new_state
    };

    // check for duplicate/out-of-order message
    let token = event.clock.utc.parse::<i64>().context("parsing Dilax token")?;
    if token <= state.token {
        warn!(
            vehicle_id = %vehicle_id,
            token = token,
            last_token = state.token,
            "Received duplicate or out-of-order Dilax message"
        );
        return Ok(());
    }

    // update token
    state.token = token;

    // reset running count if trip ID changed
    let mut reset_running_count = false;
    if let Some(trip_id) = trip_id {
        match &state.last_trip_id {
            Some(last) if last != trip_id => {
                reset_running_count = true;
                state.last_trip_id = Some(trip_id.to_string());
            }
            None => state.last_trip_id = Some(trip_id.to_string()),
            _ => {}
        }
    } else {
        reset_running_count = true;
    }

    // update occupancy count
    if reset_running_count {
        state.count = occupancy_count(0, &event.doors, vehicle_id, true);
    } else {
        state.count = occupancy_count(state.count, &event.doors, vehicle_id, false);
    }

    // update occupancy status
    let status = occupancy_status(state.count, seating_capacity, total_capacity);
    state.occupancy_status = Some(status);

    // save state
    let state_json =
        serde_json::to_string(&state).map_err(|err| Error::ServerError(err.to_string()))?;
    let replaced = state_store.set(&state_key, state_json.as_bytes(), Some(TTL_APC)).await?;

    if let (Some(before), Some(during)) = (&state_prev, &replaced)
        && before != during
    {
        warn!(
            vehicle_id = %vehicle_id,
            previous = %String::from_utf8_lossy(before),
            replaced = %String::from_utf8_lossy(during),
            "State overwritten concurrently"
        );
    }

    // update occupancy status
    if let Some(ref occupancy) = state.occupancy_status {
        let key = format!("{KEY_OCCUPANCY}:{vehicle_id}");
        state_store.set(&key, occupancy.as_bytes(), Some(TTL_OCCUPANCY_STATE)).await?;
    }

    // update count
    let count_key = format!("{KEY_VEHICLE_ID}:{vehicle_id}");
    state_store.set(&count_key, state.count.to_string().as_bytes(), Some(TTL_APC)).await?;

    Ok(())
}

/// Retrieve the vehicle trip info for a given vehicle ID.
///
/// # Errors
///
/// This function will return an error if there is an issue reading from
/// the state store, or if the stored data is malformed.
pub async fn get_trip(
    vehicle_id: &str, state_store: &impl StateStore,
) -> Result<Option<VehicleTripInfo>> {
    let key = &format!("{KEY_TRIP_INFO}:{vehicle_id}");
    let Some(bytes) = StateStore::get(state_store, key).await? else {
        return Ok(None);
    };
    let info = serde_json::from_slice(&bytes).context("deserializing vehicle trip info")?;
    Ok(Some(info))
}

/// Update the vehicle trip info with the latest Dilax APC event.
///
/// # Errors
///
/// This function will return an error if there is an issue reading or writing
/// to the state store, or if the event data is malformed.
pub async fn set_trip(vehicle_trip: VehicleTripInfo, state_store: &impl StateStore) -> Result<()> {
    let key = format!("{KEY_TRIP_INFO}:{}", vehicle_trip.vehicle_info.vehicle_id);

    let bytes =
        serde_json::to_vec(&vehicle_trip).map_err(|err| Error::ServerError(err.to_string()))?;
    state_store.set(&key, &bytes, Some(TTL_VEHICLE_TRIP_INFO)).await?;

    Ok(())
}

async fn migrate_legacy_keys(
    vehicle_id: &str, state: &mut TripState, state_store: &impl StateStore,
) -> Result<()> {
    let migration_key = format!("{KEY_VEHICLE_ID_MIGRATED}:{vehicle_id}");
    if state_store.get(&migration_key).await?.is_some() {
        return Ok(());
    }

    let legacy_trip_key = format!("{KEY_TRIPS}:{vehicle_id}");
    if let Some(bytes) = state_store.get(&legacy_trip_key).await? {
        let trip_id = String::from_utf8_lossy(&bytes);
        warn!(vehicle_id = %vehicle_id, trip_id = %trip_id, "Migrating legacy trip ID");
        state.last_trip_id = Some(trip_id.to_string());
    }

    let legacy_count_key = format!("{KEY_VEHICLE_ID}:{vehicle_id}");
    let Some(count) = state_store.get(&legacy_count_key).await? else {
        return Ok(());
    };

    let count_str = String::from_utf8_lossy(&count);
    let count_int = count_str.parse::<i64>().context("parsing legacy passenger count")?;

    warn!(vehicle_id = %vehicle_id, count = count_int, "Migrating legacy passenger count");
    state.count = count_int;

    state_store.set(&migration_key, b"true", None).await?;

    Ok(())
}

fn occupancy_status(count: i64, seating_capacity: i64, total_capacity: i64) -> String {
    let occupancy = if count < occupancy_threshold(seating_capacity, 5) {
        OccupancyStatus::Empty
    } else if count < occupancy_threshold(seating_capacity, 40) {
        OccupancyStatus::ManySeatsAvailable
    } else if count < occupancy_threshold(seating_capacity, 90) {
        OccupancyStatus::FewSeatsAvailable
    } else if count < occupancy_threshold(total_capacity, 90) {
        OccupancyStatus::StandingRoomOnly
    } else {
        OccupancyStatus::Full
    };

    // info!(vehicle_id = %vehicle_id, occupancy = %occupancy, "Updated occupancy status");
    occupancy.to_string()
}

const fn occupancy_threshold(base: i64, percent: i64) -> i64 {
    base.saturating_mul(percent).div_euclid(100)
}

fn occupancy_count(previous: i64, doors: &[Door], vehicle_id: &str, skip_out: bool) -> i64 {
    let mut total_in = 0_i64;
    let mut total_out = 0_i64;

    for door in doors {
        total_in += i64::from(door.passengers_in);
        if !skip_out {
            total_out += i64::from(door.passengers_out);
        }
    }

    let current = (previous - total_out).max(0) + total_in;
    if current < 0 {
        warn!(vehicle_id = %vehicle_id, count = current, "Calculated negative passenger count");
    }

    current.max(0)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct TripState {
    pub count: i64,
    pub token: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_trip_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occupancy_status: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
enum OccupancyStatus {
    Empty = 0,
    ManySeatsAvailable = 1,
    FewSeatsAvailable = 2,
    StandingRoomOnly = 3,
    CrushedStandingRoomOnly = 4,
    Full = 5,
    NotAcceptingPassengers = 6,
}

impl Display for OccupancyStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&(*self as u8).to_string())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VehicleTripInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_received_timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dilax_message: Option<DilaxMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trip_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_id: Option<String>,
    pub vehicle_info: VehicleInfo,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VehicleInfo {
    pub label: Option<String>,
    #[serde(rename = "vehicleId")]
    pub vehicle_id: String,
}
