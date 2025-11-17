use std::env;

use chrono::{Duration, Utc};
use chrono_tz::Tz;

#[derive(Debug, Clone)]
pub struct Config {
    pub timezone: Tz,
    pub god_mode_enabled: bool,
    pub accuracy_threshold: f64,
    pub trip_duration_buffer: Duration,
    pub serial_data_filter_threshold: Duration,
    pub default_train_total_capacity: i64,
    pub default_train_seating_capacity: i64,
    pub keys: Keys,
    pub topics: Topics,
}

impl Config {
    pub fn from_env() -> Self {
        let timezone = env::var("TIMEZONE")
            .ok()
            .and_then(|value| value.parse::<Tz>().ok())
            .unwrap_or(chrono_tz::Pacific::Auckland);
        let god_mode_enabled = env_bool("GOD_MODE", false);
        let accuracy_threshold = env_f64("ACCURACY_THRESHOLD", 0.0);
        let trip_duration_buffer = Duration::seconds(env_i64("TRIP_DURATION_BUFFER", 3_600));
        let serial_data_filter_threshold =
            Duration::seconds(env_i64("SERIAL_DATA_FILTER_THRESHOLD", 900));
        let default_train_total_capacity = env_i64("DEFAULT_TRAIN_TOTAL_CAPACITY", 373);
        let default_train_seating_capacity = env_i64("DEFAULT_TRAIN_SEATING_CAPACITY", 230);
        let keys = Keys::from_env();
        let topics = Topics::from_env();

        Self {
            timezone,
            god_mode_enabled,
            accuracy_threshold,
            trip_duration_buffer,
            serial_data_filter_threshold,
            default_train_total_capacity,
            default_train_seating_capacity,
            keys,
            topics,
        }
    }

    pub fn trip_key(&self, vehicle_id: &str) -> String {
        format!("{}:{}", self.keys.trip, vehicle_id)
    }

    pub fn sign_on_key(&self, vehicle_id: &str) -> String {
        format!("{}:{}", self.keys.vehicle_sign_on, vehicle_id)
    }

    pub fn passenger_count_key(
        &self, vehicle_id: &str, trip_id: &str, start_date: &str, start_time: &str,
    ) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.keys.passenger_count, vehicle_id, trip_id, start_date, start_time
        )
    }

    pub fn fleet_key_by_label(&self, label: &str) -> String {
        format!("{}:label:{}", self.keys.fleet, label)
    }

    pub fn fleet_key_by_id(&self, vehicle_id: &str) -> String {
        format!("{}:vehicleId:{}", self.keys.fleet, vehicle_id)
    }

    pub fn fleet_capacity_key(&self, vehicle_id: &str, route_id: &str) -> String {
        format!("{}:capacityBasedOnRouteId:{}:{}", self.keys.fleet, vehicle_id, route_id)
    }

    pub fn block_key(&self, vehicle_id: &str) -> String {
        format!("{}:{}", self.keys.block_management, vehicle_id)
    }

    pub fn trip_mgt_key(&self, trip_id: &str, service_date: &str) -> String {
        format!("{}:{}:{}", self.keys.trip_management, trip_id, service_date)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::from_env()
    }
}

#[derive(Debug, Clone)]
pub struct Keys {
    pub trip: String,
    pub fleet: String,
    pub vehicle_sign_on: String,
    pub trip_management: String,
    pub block_management: String,
    pub allocated_vehicle: String,
    pub vehicle_blacklist: String,
    pub passenger_count: String,
}

impl Keys {
    fn from_env() -> Self {
        Self {
            trip: env::var("REDIS_KEY_TRIP")
                .unwrap_or_else(|_| "smartrakGtfs:trip:vehicle".to_string()),
            fleet: env::var("REDIS_KEY_FLEET").unwrap_or_else(|_| "smartrakGtfs:fleet".to_string()),
            vehicle_sign_on: env::var("REDIS_KEY_VEHICLE_SIGN_ON")
                .unwrap_or_else(|_| "smartrakGtfs:vehicle:signOn".to_string()),
            trip_management: env::var("REDIS_KEY_TRIP_MANAGEMENT")
                .unwrap_or_else(|_| "smartrakGtfs:tripManagement".to_string()),
            block_management: env::var("REDIS_KEY_BLOCK_MANAGEMENT")
                .unwrap_or_else(|_| "smartrakGtfs:blockManagement".to_string()),
            allocated_vehicle: env::var("REDIS_KEY_ALLOCATED_VEHICLE")
                .unwrap_or_else(|_| "smartrakGtfs:trip:allocatedVehicle".to_string()),
            vehicle_blacklist: env::var("REDIS_KEY_VEHICLE_BLACKLIST")
                .unwrap_or_else(|_| "smartrakGtfs:vehicleBlacklist".to_string()),
            passenger_count: env::var("REDIS_KEY_PASSENGER_COUNT")
                .unwrap_or_else(|_| "smartrakGtfs:passengerCountEvent".to_string()),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Topics {
    pub passenger_count_markers: Vec<String>,
    pub caf_markers: Vec<String>,
    pub smartrak_markers: Vec<String>,
    pub passthrough_markers: Vec<String>,
    pub vehicle_position: Option<String>,
    pub dead_reckoning: Option<String>,
}

impl Topics {
    fn from_env() -> Self {
        let parse_list = |key: &str| -> Vec<String> {
            env::var(key)
                .map(|value| {
                    value
                        .split(',')
                        .map(|entry| entry.trim().to_string())
                        .filter(|entry| !entry.is_empty())
                        .collect()
                })
                .unwrap_or_default()
        };

        Self {
            passenger_count_markers: parse_list("PASSENGER_COUNT_TOPICS"),
            caf_markers: parse_list("CAF_TOPICS"),
            smartrak_markers: parse_list("SMARTRAK_TOPICS"),
            passthrough_markers: parse_list("PASSTHROUGH_TOPICS"),
            vehicle_position: env::var("VEHICLE_POSITION_TOPIC").ok(),
            dead_reckoning: env::var("DEAD_RECKONING_TOPIC").ok(),
        }
    }

    pub fn matches_passenger_topic(&self, topic: &str) -> bool {
        self.passenger_count_markers.iter().any(|pattern| topic.contains(pattern))
    }

    pub fn matches_caf_topic(&self, topic: &str) -> bool {
        self.caf_markers.iter().any(|pattern| topic.contains(pattern))
    }

    pub fn matches_smartrak_topic(&self, topic: &str) -> bool {
        self.smartrak_markers.iter().any(|pattern| topic.contains(pattern))
    }

    pub fn matches_passthrough_topic(&self, topic: &str) -> bool {
        self.passthrough_markers.iter().any(|pattern| topic.contains(pattern))
    }
}

fn env_bool(key: &str, default: bool) -> bool {
    env::var(key)
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "true" | "1" | "yes"))
        .unwrap_or(default)
}

fn env_f64(key: &str, default: f64) -> f64 {
    env::var(key).ok().and_then(|value| value.parse::<f64>().ok()).unwrap_or(default)
}

fn env_i64(key: &str, default: i64) -> i64 {
    env::var(key).ok().and_then(|value| value.parse::<i64>().ok()).unwrap_or(default)
}
