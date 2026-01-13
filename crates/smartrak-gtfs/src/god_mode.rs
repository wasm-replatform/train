use std::collections::HashMap;

use anyhow::{Context, Result};
use qwasr_sdk::{Config, StateStore};
use serde::{Deserialize, Serialize};

use crate::{EventType, SmarTrakMessage};

const KEY_GOD_MODE: &str = "god_mode:overrides";
const TTL_GOD_MODE: u64 = 24 * 60 * 60; // 24 hours

#[derive(Debug, Default, Serialize, Deserialize)]
struct GodModeState {
    overrides: HashMap<String, String>,
}

/// Load the current God Mode state from the state store.
async fn load_state(state_store: &impl StateStore) -> Result<GodModeState> {
    let Some(bytes) = state_store.get(KEY_GOD_MODE).await? else {
        return Ok(GodModeState::default());
    };
    let state = serde_json::from_slice(&bytes).context("deserializing god mode state")?;
    Ok(state)
}

/// Save the current God Mode state to the state store.
async fn save_state(state_store: &impl StateStore, state: &GodModeState) -> Result<()> {
    let bytes = serde_json::to_vec(state).context("serializing god mode state")?;
    state_store.set(KEY_GOD_MODE, &bytes, Some(TTL_GOD_MODE)).await?;
    Ok(())
}

/// Reset all vehicle overrides.
///
/// # Errors
///
/// Returns an error if the state cannot be persisted to the state store.
pub async fn reset_all(state_store: &impl StateStore) -> Result<()> {
    let state = GodModeState::default();
    save_state(state_store, &state).await
}

/// Reset the override for a specific vehicle.
///
/// # Errors
///
/// Returns an error if the state cannot be loaded or persisted to the state store.
pub async fn reset_vehicle(state_store: &impl StateStore, vehicle_id: &str) -> Result<()> {
    let mut state = load_state(state_store).await?;
    state.overrides.remove(vehicle_id);
    save_state(state_store, &state).await
}

/// Set a vehicle to a specific trip ID.
///
/// # Errors
///
/// Returns an error if the state cannot be loaded or persisted to the state store.
pub async fn set_vehicle_to_trip(
    state_store: &impl StateStore, vehicle_id: impl Into<String>, trip_id: impl Into<String>,
) -> Result<()> {
    let mut state = load_state(state_store).await?;
    state.overrides.insert(vehicle_id.into(), trip_id.into());
    save_state(state_store, &state).await
}

/// Describe the current state of all overrides as a JSON string.
///
/// # Errors
///
/// Returns an error if the state cannot be loaded from the state store.
pub async fn describe(state_store: &impl StateStore) -> Result<String> {
    let state = load_state(state_store).await?;
    let map: Vec<(String, String)> = state.overrides.into_iter().collect();
    Ok(serde_json::to_string(&map).unwrap_or_default())
}

/// Preprocess a SmarTrak message, applying any vehicle overrides.
///
/// # Errors
///
/// Returns an error if the state cannot be loaded from the state store.
pub async fn preprocess(state_store: &impl StateStore, event: &mut SmarTrakMessage) -> Result<()> {
    if event.event_type != EventType::SerialData {
        return Ok(());
    }

    let Some(remote_data) = event.remote_data.as_ref() else {
        return Ok(());
    };

    let Some(vehicle_id) = remote_data.external_id.as_deref() else {
        return Ok(());
    };

    let Some(serial) = event.serial_data.as_mut() else {
        return Ok(());
    };

    let Some(decoded) = serial.decoded_serial_data.as_mut() else {
        return Ok(());
    };

    let state = load_state(state_store).await?;
    if let Some(override_trip) = state.overrides.get(vehicle_id) {
        decoded.line_id = None;

        if override_trip == "empty" {
            decoded.trip_id = None;
            decoded.trip_number = None;
        } else {
            decoded.trip_id = Some(override_trip.clone());
            decoded.trip_number = Some(override_trip.clone());
        }
    }

    Ok(())
}

/// Check if God Mode is enabled via configuration.
///
/// # Errors
///
/// Returns an error if the configuration cannot be read.
pub async fn is_enabled(provider: &impl Config) -> Result<bool> {
    Ok(Config::get(provider, "GOD_MODE_ENABLED").await.ok().is_some_and(|value| {
        let normalized = value.trim().to_ascii_lowercase();
        matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
    }))
}
