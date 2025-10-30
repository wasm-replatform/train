use std::borrow::Cow;
use std::time::Duration;

/// Redis key namespace configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisConfig {
    pub key_occupancy: Cow<'static, str>,
    pub apc_vehicle_id_migrated_key: Cow<'static, str>,
    pub apc_vehicle_id_key: Cow<'static, str>,
    pub apc_vehicle_trip_key: Cow<'static, str>,
    pub apc_vehicle_id_state_key: Cow<'static, str>,
    pub key_vehicle_trip_info: Cow<'static, str>,
    pub vehicle_label_key: Cow<'static, str>,
    pub lost_connections_set: Cow<'static, str>,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            key_occupancy: Cow::Borrowed("trip:occupancy"),
            apc_vehicle_id_migrated_key: Cow::Borrowed("apc:vehicleIdMigratred"),
            apc_vehicle_id_key: Cow::Borrowed("apc:vehicleId"),
            apc_vehicle_trip_key: Cow::Borrowed("apc:trips"),
            apc_vehicle_id_state_key: Cow::Borrowed("apc:vehicleIdState"),
            key_vehicle_trip_info: Cow::Borrowed("apc:vehicleTripInfo"),
            vehicle_label_key: Cow::Borrowed("smartrakGtfs:vehicleLabel"),
            lost_connections_set: Cow::Borrowed("apc:lostConnections"),
        }
    }
}

/// Application configuration derived from the host environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub timezone: Cow<'static, str>,
    pub apc_ttl: Duration,
    pub occupancy_state_ttl: Duration,
    pub lost_connection_retention: Duration,
    pub lost_connection_threshold: Duration,
    pub stop_refresh_interval: Duration,
    pub allocation_refresh_interval: Duration,
    pub reset_count_on_trip_ended: bool,
    pub redis: RedisConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            timezone: Cow::Borrowed("Pacific/Auckland"),
            apc_ttl: Duration::from_secs(60 * 60),
            occupancy_state_ttl: Duration::from_secs(3 * 30 * 60),
            lost_connection_retention: Duration::from_secs(7 * 24 * 60 * 60),
            lost_connection_threshold: Duration::from_secs(60 * 60),
            stop_refresh_interval: Duration::from_secs(24 * 60 * 60),
            allocation_refresh_interval: Duration::from_secs(60),
            reset_count_on_trip_ended: false,
            redis: RedisConfig::default(),
        }
    }
}
