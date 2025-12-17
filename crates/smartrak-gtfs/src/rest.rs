// use common::block_mgt;
use common::fleet::{self, Vehicle};
use realtime::{Config, HttpRequest, Identity, Publisher, StateStore};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tracing::{error, info, instrument};

use crate::god_mode::god_mode;
use crate::trip::TripInstance;

const PROCESS_ID: u32 = 0;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleInfoResponse {
    pub pid: u32,
    pub vehicle_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sign_on_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trip_info: Option<TripInstance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fleet_info: Option<Vehicle>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse {
    pub message: String,
    pub process: u32,
}

impl ApiResponse {
    #[must_use]
    pub fn ok() -> Self {
        Self::new("Ok")
    }

    #[must_use]
    pub fn not_found() -> Self {
        Self::new("Ops...")
    }

    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), process: PROCESS_ID }
    }
}

#[derive(Debug, Clone)]
pub enum GodModeOutcome {
    Enabled(ApiResponse),
    Disabled(ApiResponse),
}

/// Logs information about the root endpoint invocation.
#[instrument(level = "info", name = "log_root")]
pub fn log_root(user_agent: Option<&str>) {
    if let Some(agent) = user_agent {
        info!(user_agent = agent, "root endpoint invoked");
    } else {
        info!("root endpoint invoked");
    }
}

/// Retrieves the cached vehicle/trip information used by the legacy REST endpoint.
pub async fn vehicle_info<P>(provider: &P, vehicle_id: &str) -> VehicleInfoResponse
where
    P: HttpRequest + Publisher + StateStore + Identity + Config,
{
    let trip_key = format!("smartrakGtfs:trip:vehicle:{vehicle_id}");
    let trip_info = match StateStore::get(provider, &trip_key).await {
        Ok(bytes) => deserialize_optional::<TripInstance>(bytes),
        Err(err) => {
            error!(vehicle_id, ?err, "failed to fetch trip info from cache");
            None
        }
    };

    let sign_on_key = format!("smartrakGtfs:vehicle:signOn:{vehicle_id}");
    let sign_on_time = match StateStore::get(provider, &sign_on_key).await {
        Ok(bytes) => sign_on_to_string(bytes),
        Err(err) => {
            error!(vehicle_id, ?err, "failed to fetch sign-on time from cache");
            None
        }
    };

    let fleet_info = match fleet::vehicle(vehicle_id, provider).await {
        Ok(info) => info,
        Err(err) => {
            error!(vehicle_id, ?err, "failed to fetch fleet info");
            None
        }
    };

    VehicleInfoResponse {
        pid: PROCESS_ID,
        vehicle_id: vehicle_id.to_string(),
        sign_on_time,
        trip_info,
        fleet_info,
    }
}

/// Applies a God Mode trip override, mirroring the legacy behaviour.
#[must_use]
pub fn god_mode_set_trip(vehicle_id: &str, trip_id: &str) -> GodModeOutcome {
    god_mode().map_or_else(
        || {
            info!("god mode not enabled; set-trip ignored");
            GodModeOutcome::Disabled(ApiResponse::not_found())
        },
        |god_mode| {
            god_mode.set_vehicle_to_trip(vehicle_id.to_string(), trip_id.to_string());
            info!(vehicle_id, trip_id, "god mode override set");
            GodModeOutcome::Enabled(ApiResponse::ok())
        },
    )
}

/// Clears God Mode overrides for a specific vehicle or for all vehicles.
#[must_use]
pub fn god_mode_reset(vehicle_id: &str) -> GodModeOutcome {
    god_mode().map_or_else(
        || {
            info!("god mode not enabled; reset ignored");
            GodModeOutcome::Disabled(ApiResponse::not_found())
        },
        |god_mode| {
            if vehicle_id == "all" {
                god_mode.reset_all();
                info!("god mode overrides reset for all vehicles");
            } else {
                god_mode.reset_vehicle(vehicle_id);
                info!(vehicle_id, "god mode override reset");
            }
            GodModeOutcome::Enabled(ApiResponse::ok())
        },
    )
}

fn deserialize_optional<T>(data: Option<Vec<u8>>) -> Option<T>
where
    T: DeserializeOwned,
{
    data.and_then(|raw| serde_json::from_slice::<T>(&raw).ok())
}

fn sign_on_to_string(data: Option<Vec<u8>>) -> Option<String> {
    deserialize_optional::<Value>(data).and_then(|value| match value {
        Value::Null => None,
        Value::String(s) => Some(s),
        Value::Number(num) => num.as_i64().map(|n| n.to_string()),
        other => Some(other.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::sign_on_to_string;

    #[test]
    fn converts_numeric_sign_on() {
        let value = serde_json::to_vec(&1234_i64).expect("serialize");
        assert_eq!(sign_on_to_string(Some(value)), Some("1234".to_string()));
    }

    #[test]
    fn ignores_null_sign_on() {
        let value = serde_json::to_vec(&Value::Null).expect("serialize");
        assert_eq!(sign_on_to_string(Some(value)), None);
    }
}
