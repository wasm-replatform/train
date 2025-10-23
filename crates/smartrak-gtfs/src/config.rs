use std::env;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono_tz::Tz;

#[derive(Debug, Clone)]
pub struct Config {
    // Tracks AppConfig parity from legacy/at_smartrak_gtfs_adapter/src/config/app.ts.
    pub timezone: Tz,
    pub enable_god_mode: bool,
    pub accuracy_threshold: f64,
    pub trip_duration_buffer: i64,
    pub serial_data_filter_threshold: i64,
    pub default_train_total_capacity: u32,
    pub default_train_seating_capacity: u32,
    pub redis_keys: CacheKeys,
    pub topics: Topics,
    pub fleet_api_url: String,
    pub trip_management_url: String,
    pub block_management_url: String,
    pub new_relic_prefix: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let timezone = env::var("TIMEZONE").unwrap_or_else(|_| "Pacific/Auckland".to_string());
        let timezone = timezone.parse::<Tz>().context("parsing TIMEZONE")?;

        let app_name = env::var("APP_NAME").unwrap_or_default();
        let enable_god_mode = env_bool("GOD_MODE", false);
        let accuracy_threshold = env_f64("ACCURACY_THRESHOLD", 0.0);
        let trip_duration_buffer = env_i64("TRIP_DURATION_BUFFER", 3_600);
        let serial_data_filter_threshold = env_i64("SERIAL_DATA_FILTER_THRESHOLD", 900);
        let default_train_total_capacity = env_u32("DEFAULT_TRAIN_TOTAL_CAPACITY", 373);
        let default_train_seating_capacity = env_u32("DEFAULT_TRAIN_SEATING_CAPACITY", 230);
        let redis_keys = CacheKeys::default();
        let topics = Topics::from_env()?;
        let fleet_api_url = env::var("FLEET_API_URL")
            .unwrap_or_else(|_| "https://www-dev-at-fleet-api-01.azurewebsites.net".to_string());
        let trip_management_url = env::var("TRIP_MANAGEMENT_URL")
            .unwrap_or_else(|_| "https://www-dev-trip-mgt-api-01.azurewebsites.net".to_string());
        let block_management_url = env::var("BLOCK_MANAGEMENT_URL")
            .unwrap_or_else(|_| "https://www-dev-block-mgt-api-01.azurewebsites.net".to_string());

        Ok(Self {
            timezone,
            enable_god_mode,
            accuracy_threshold,
            trip_duration_buffer,
            serial_data_filter_threshold,
            default_train_total_capacity,
            default_train_seating_capacity,
            redis_keys,
            topics,
            fleet_api_url,
            trip_management_url,
            block_management_url,
            new_relic_prefix: app_name,
        })
    }

    pub fn trip_key(&self, vehicle_id: &str) -> String {
        format!("{}:{}", self.redis_keys.trip_key, vehicle_id)
    }

    pub fn sign_on_key(&self, vehicle_id: &str) -> String {
        format!("{}:{}", self.redis_keys.vehicle_so_time_key, vehicle_id)
    }

    pub fn passenger_count_key(
        &self, vehicle_id: &str, trip_id: &str, start_date: &str, start_time: &str,
    ) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.redis_keys.passenger_count_key, vehicle_id, trip_id, start_date, start_time
        )
    }

    pub fn fleet_key_by_label(&self, label: &str) -> String {
        format!("{}:label:{}", self.redis_keys.fleet_key, label)
    }

    pub fn fleet_key_by_id(&self, vehicle_id: &str) -> String {
        format!("{}:vehicleId:{}", self.redis_keys.fleet_key, vehicle_id)
    }

    pub fn fleet_capacity_key(&self, vehicle_id: &str, route_id: &str) -> String {
        format!("{}:capacityBasedOnRouteId:{}:{}", self.redis_keys.fleet_key, vehicle_id, route_id)
    }

    pub fn block_key(&self, vehicle_id: &str) -> String {
        format!("{}:{}", self.redis_keys.block_management_key, vehicle_id)
    }

    pub fn trip_mgt_key(&self, trip_id: &str, service_date: &str) -> String {
        format!("{}:{}:{}", self.redis_keys.trip_management_key, trip_id, service_date)
    }
}

// Mirrors redis key layout from legacy/at_smartrak_gtfs_adapter/src/config/redis.ts.
#[derive(Debug, Clone)]
pub struct CacheKeys {
    pub trip_key: &'static str,
    pub fleet_key: &'static str,
    pub vehicle_so_time_key: &'static str,
    pub trip_management_key: &'static str,
    pub block_management_key: &'static str,
    pub allocated_vehicle_key: &'static str,
    pub vehicle_blacklist_key: &'static str,
    pub passenger_count_key: &'static str,
}

impl Default for CacheKeys {
    fn default() -> Self {
        Self {
            trip_key: "smartrakGtfs:trip:vehicle",
            fleet_key: "smartrakGtfs:fleet",
            vehicle_so_time_key: "smartrakGtfs:vehicle:signOn",
            trip_management_key: "smartrakGtfs:tripManagement",
            block_management_key: "smartrakGtfs:blockManagement",
            allocated_vehicle_key: "smartrakGtfs:trip:allocatedVehicle",
            vehicle_blacklist_key: "smartrakGtfs:vehicleBlacklist",
            passenger_count_key: "smartrakGtfs:passengerCountEvent",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Topics {
    pub subscriptions: Vec<String>,
    pub passenger_count_topic: String,
    pub vp_topic: String,
    pub dr_topic: String,
    pub caf_topic: Option<String>,
}

impl Topics {
    fn from_env() -> Result<Self> {
        // Mirrors topic resolution in legacy/at_smartrak_gtfs_adapter/src/config/kafka-producer.ts.
        let env_name =
            env::var("CONFLUENT_KAFKA_ENVIRONMENT").unwrap_or_else(|_| "dev".to_string());
        let use_caf_topic = env_bool("USE_CAF_TOPIC", false);
        let use_schema_registry = env_bool("USE_SCHEMA_REGISTRY_PRODUCER", false);

        let smartrak_bus_v1 =
            get_confluent_topic(ConfluentTopic::SmartrakBusAvl, &env_name, Version::V1);
        let smartrak_bus_v2 =
            get_confluent_topic(ConfluentTopic::SmartrakBusAvl, &env_name, Version::V2);
        let smartrak_train =
            get_confluent_topic(ConfluentTopic::SmartrakTrainAvl, &env_name, Version::V1);
        let r9k_to_smartrak =
            get_confluent_topic(ConfluentTopic::R9kToSmartrak, &env_name, Version::V1);
        let passenger_count =
            get_confluent_topic(ConfluentTopic::PassengerCount, &env_name, Version::V1);
        let caf_topic = if use_caf_topic {
            Some(get_confluent_topic(ConfluentTopic::CafAvl, &env_name, Version::V2))
        } else {
            None
        };

        let mut subscriptions = vec![
            smartrak_bus_v1.clone(),
            smartrak_bus_v2.clone(),
            smartrak_train.clone(),
            r9k_to_smartrak.clone(),
            passenger_count.clone(),
        ];
        if let Some(caf) = caf_topic.clone() {
            subscriptions.push(caf);
        }

        let vp_topic = get_confluent_topic(
            ConfluentTopic::GtfsVp,
            &env_name,
            if use_schema_registry { Version::V2 } else { Version::V1 },
        );
        let dr_topic = get_confluent_topic(ConfluentTopic::DeadReckoning, &env_name, Version::V1);

        Ok(Self {
            subscriptions,
            passenger_count_topic: passenger_count,
            vp_topic,
            dr_topic,
            caf_topic,
        })
    }

    pub fn contains(&self, topic: &str) -> bool {
        self.subscriptions.iter().any(|candidate| candidate == topic)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Version {
    V1,
    V2,
}

impl Version {
    const fn suffix(self) -> &'static str {
        match self {
            Version::V1 => "v1",
            Version::V2 => "v2",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ConfluentTopic {
    SmartrakBusAvl,
    SmartrakTrainAvl,
    R9kToSmartrak,
    PassengerCount,
    CafAvl,
    GtfsVp,
    DeadReckoning,
}

impl ConfluentTopic {
    const fn name(self) -> &'static str {
        match self {
            ConfluentTopic::SmartrakBusAvl => "smartrak-avl",
            ConfluentTopic::SmartrakTrainAvl => "smartrak-train-avl",
            ConfluentTopic::R9kToSmartrak => "r9k-to-smartrak",
            ConfluentTopic::PassengerCount => "passenger-count",
            ConfluentTopic::CafAvl => "caf-avl",
            ConfluentTopic::GtfsVp => "gtfs-vp",
            ConfluentTopic::DeadReckoning => "dead-reckoning",
        }
    }
}

pub fn get_confluent_topic(topic: ConfluentTopic, environment: &str, version: Version) -> String {
    format!("{}-realtime-{}.{}", environment, topic.name(), version.suffix())
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

fn env_u32(key: &str, default: u32) -> u32 {
    env::var(key).ok().and_then(|value| value.parse::<u32>().ok()).unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
    env::var(key).ok().and_then(|value| value.parse::<u64>().ok()).unwrap_or(default)
}

// Cache TTLs keep parity with legacy processors in legacy/at_smartrak_gtfs_adapter/src/processors.
pub const CACHE_TTL_FLEET_SUCCESS: Duration = Duration::from_secs(10 * 60);
pub const CACHE_TTL_FLEET_FAILURE: Duration = Duration::from_secs(60);
pub const CACHE_TTL_TRIP_SUCCESS: Duration = Duration::from_secs(20);
pub const CACHE_TTL_TRIP_FAILURE: Duration = Duration::from_secs(10);
pub const CACHE_TTL_BLOCK_SUCCESS: Duration = Duration::from_secs(20);
pub const CACHE_TTL_BLOCK_FAILURE: Duration = Duration::from_secs(10);
pub const CACHE_TTL_PASSENGER_COUNT: Duration = Duration::from_secs(3 * 60 * 60);
pub const CACHE_TTL_TRIP_SERIAL: Duration = Duration::from_secs(4 * 60 * 60);
pub const CACHE_TTL_TRIP_TRAIN: Duration = Duration::from_secs(3 * 60 * 60);
pub const CACHE_TTL_SIGN_ON: Duration = Duration::from_secs(24 * 60 * 60);
