use std::env;

use anyhow::{Context, Result};

/// Adapter configuration loaded from the environment.
#[derive(Debug, Clone)]
pub struct Config {
    pub fleet_api_url: String,
    pub gtfs_static_api_url: String,
    pub block_mgt_api_url: String,
    pub cc_static_api_url: String,
    pub apc_ttl_secs: u64,
    pub legacy_redis_ttl_secs: u64,
    pub vehicle_trip_info_ttl_secs: u64,
    pub connection_lost_threshold_mins: u64,
    pub redis: RedisKeys,
    pub timezone: &'static str,
    pub stop_search_radius_meters: u32,
    pub diesel_train_prefix: &'static str,
}

impl Config {
    const DEFAULT_TIMEZONE: &'static str = "Pacific/Auckland";
    const DEFAULT_LEGACY_TTL_SECS: u64 = 3 * 30 * 60;
    const DEFAULT_VEHICLE_TRIP_INFO_TTL: u64 = 2 * 24 * 60 * 60;
    const DEFAULT_STOP_SEARCH_RADIUS_METERS: u32 = 150;
    const DEFAULT_DIESEL_TRAIN_PREFIX: &'static str = "ADL";

    /// Construct a configuration from environment variables, validating required fields.
    pub fn from_env() -> Result<Self> {
        let fleet_api_url = get_env("FLEET_API_URL")?;
        let gtfs_static_api_url = get_env("GTFS_STATIC_URL")?;
        let block_mgt_api_url = get_env("BLOCK_MGT_CLIENT_API_URL")?;
        let cc_static_api_url = get_env("CC_STATIC_API_HOST")?;

        let apc_ttl_secs =
            env::var("APC_TTL_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(3600);
        let connection_lost_threshold_mins = env::var("DILAX_CONNECTION_LOST_THRESHOLD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);

        Ok(Self {
            fleet_api_url,
            gtfs_static_api_url,
            block_mgt_api_url,
            cc_static_api_url,
            apc_ttl_secs,
            legacy_redis_ttl_secs: Self::DEFAULT_LEGACY_TTL_SECS,
            vehicle_trip_info_ttl_secs: Self::DEFAULT_VEHICLE_TRIP_INFO_TTL,
            connection_lost_threshold_mins,
            redis: RedisKeys::default(),
            timezone: Self::DEFAULT_TIMEZONE,
            stop_search_radius_meters: Self::DEFAULT_STOP_SEARCH_RADIUS_METERS,
            diesel_train_prefix: Self::DEFAULT_DIESEL_TRAIN_PREFIX,
        })
    }
}

/// Redis key configuration.
#[derive(Debug, Clone)]
pub struct RedisKeys {
    pub key_occupancy: String,
    pub apc_vehicle_id_migrated_key: String,
    pub apc_vehicle_id_key: String,
    pub apc_vehicle_trip_key: String,
    pub apc_vehicle_id_state_key: String,
    pub key_vehicle_trip_info: String,
    pub lost_connections_set: String,
}

impl RedisKeys {
    pub fn namespaced_key(&self, base: &str, identifier: &str) -> String {
        format!("{base}:{identifier}")
    }
}

impl Default for RedisKeys {
    fn default() -> Self {
        Self {
            key_occupancy: "trip:occupancy".to_string(),
            apc_vehicle_id_migrated_key: "apc:vehicleIdMigrated".to_string(),
            apc_vehicle_id_key: "apc:vehicleId".to_string(),
            apc_vehicle_trip_key: "apc:trips".to_string(),
            apc_vehicle_id_state_key: "apc:vehicleIdState".to_string(),
            key_vehicle_trip_info: "apc:vehicleTripInfo".to_string(),
            lost_connections_set: "apc:lostConnections".to_string(),
        }
    }
}

fn get_env(name: &str) -> Result<String> {
    env::var(name).with_context(|| format!("missing required environment variable `{name}`"))
}
