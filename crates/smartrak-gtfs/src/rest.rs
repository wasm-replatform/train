use std::sync::OnceLock;

use anyhow::{Context, anyhow};
use serde::Serialize;
use thiserror::Error;

use crate::cache::CacheRepository;
use crate::config::Config;
use crate::god_mode::GodMode;
use crate::model::fleet::VehicleInfo;
use crate::model::trip::TripInstance;

/// Errors emitted by the SmarTrak REST service layer.
#[derive(Debug, Error)]
pub enum RestError {
    /// Resource was not found or the action is intentionally hidden.
    #[error("resource not found")]
    NotFound,
    /// Wrapper for underlying domain or infrastructure errors.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Response payload for `GET /info/:vehicleId`.
/// Mirrors legacy payload in `legacy/at_smartrak_gtfs_adapter/src/controller/rest.ts#getVehicleInfoById`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleInfoResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fleet_info: Option<VehicleInfo>,
    pub pid: u32,
    pub sign_on_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trip_info: Option<TripInstance>,
    pub vehicle_id: String,
}

/// Generic OK response payload reused by GodMode endpoints.
/// Matches legacy controller responses in `legacy/at_smartrak_gtfs_adapter/src/controller/rest.ts`.
#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub message: String,
    pub process: u32,
}

impl ApiResponse {
    const OK_MESSAGE: &'static str = "Ok";

    fn ok() -> Self {
        Self { message: Self::OK_MESSAGE.to_string(), process: process_id() }
    }
}

/// Facade exposing the REST business operations without HTTP bindings.
pub struct RestService;

impl RestService {
    /// Build the payload returned by `GET /info/:vehicleId`.
    ///
    /// # Errors
    /// Returns [`RestError::Other`] when configuration or cache access fails.
    pub fn vehicle_info(vehicle_id: &str) -> Result<VehicleInfoResponse, RestError> {
        let config = config()?;
        let cache = cache()?;

        let trip_key = config.trip_key(vehicle_id);
        let fleet_key = config.fleet_key_by_id(vehicle_id);
        let sign_on_key = config.sign_on_key(vehicle_id);

        let trip_info = cache.get_json::<TripInstance>(&trip_key).context("reading trip info")?;
        let fleet_info = cache.get_json::<VehicleInfo>(&fleet_key).context("reading fleet info")?;
        let sign_on_time = cache.get(&sign_on_key).context("reading sign-on time")?;

        Ok(VehicleInfoResponse {
            pid: process_id(),
            vehicle_id: vehicle_id.to_string(),
            sign_on_time,
            trip_info,
            fleet_info,
        })
    }

    /// Mirror `GodMode` assignments initiated via `GET /god-mode/set-trip/:vehicleId/:tripId`.
    ///
    /// # Errors
    /// Returns [`RestError::NotFound`] when `GodMode` is disabled and [`RestError::Other`] when
    /// configuration initialization fails.
    pub fn set_vehicle_to_trip(vehicle_id: &str, trip_id: &str) -> Result<ApiResponse, RestError> {
        let config = config()?;
        if !config.enable_god_mode {
            return Err(RestError::NotFound);
        }

        god_mode().set_vehicle_to_trip(vehicle_id.to_string(), trip_id.to_string());
        Ok(ApiResponse::ok())
    }

    /// Mirror `GodMode` resets initiated via `GET /god-mode/reset/:vehicleId`.
    ///
    /// # Errors
    /// Returns [`RestError::NotFound`] when `GodMode` is disabled and [`RestError::Other`] when
    /// configuration initialization fails.
    pub fn reset_vehicle(vehicle_id: &str) -> Result<ApiResponse, RestError> {
        let config = config()?;
        if !config.enable_god_mode {
            return Err(RestError::NotFound);
        }

        if vehicle_id == "all" {
            god_mode().reset_all();
        } else {
            god_mode().reset_vehicle(vehicle_id);
        }

        Ok(ApiResponse::ok())
    }
}

const PROCESS_ID: u32 = 0;

const fn process_id() -> u32 {
    PROCESS_ID
}

fn config() -> Result<&'static Config, RestError> {
    static CONFIG: OnceLock<Config> = OnceLock::new();

    if let Some(config) = CONFIG.get() {
        return Ok(config);
    }

    let config = Config::from_env()?;
    let _ = CONFIG.set(config);
    CONFIG.get().ok_or_else(|| RestError::Other(anyhow!("config initialization failed")))
}

fn cache() -> Result<&'static CacheRepository, RestError> {
    static CACHE: OnceLock<CacheRepository> = OnceLock::new();

    if let Some(cache) = CACHE.get() {
        return Ok(cache);
    }

    let repo = CacheRepository::new()?;
    let _ = CACHE.set(repo);
    CACHE.get().ok_or_else(|| RestError::Other(anyhow!("cache initialization failed")))
}

fn god_mode() -> &'static GodMode {
    static GOD_MODE: OnceLock<GodMode> = OnceLock::new();
    GOD_MODE.get_or_init(GodMode::default)
}
