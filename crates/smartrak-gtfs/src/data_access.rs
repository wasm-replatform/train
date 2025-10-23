use anyhow::{Context, Result};
use chrono::{Duration as ChronoDuration, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use dashmap::DashMap;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tracing::error;

use crate::cache::CacheRepository;
use crate::config::{
    CACHE_TTL_BLOCK_FAILURE, CACHE_TTL_BLOCK_SUCCESS, CACHE_TTL_FLEET_FAILURE,
    CACHE_TTL_FLEET_SUCCESS, CACHE_TTL_TRIP_FAILURE, CACHE_TTL_TRIP_SUCCESS, Config,
};
use crate::model::fleet::{VehicleCapacity, VehicleInfo};
use crate::model::trip::{BlockInstance, TripInstance};
use crate::provider::AdapterProvider;

// Mirrors FleetApiService access patterns from legacy/at_smartrak_gtfs_adapter/src/apis/fleet.ts.
#[derive(Debug, Clone)]
pub struct FleetAccess<P: AdapterProvider> {
    config: Arc<Config>,
    provider: P,
    cache: Arc<CacheRepository>,
}

impl<P: AdapterProvider> FleetAccess<P> {
    pub fn new(config: Arc<Config>, provider: P, cache: Arc<CacheRepository>) -> Self {
        Self { config, provider, cache }
    }

    pub async fn by_label(&self, label: &str) -> Result<Option<VehicleInfo>> {
        let key = self.config.fleet_key_by_label(label);
        self.fetch_cached(&key, || async {
            self.provider.fetch_vehicle_by_label(label).await.context("fetching vehicle by label")
        })
        .await
    }

    pub async fn by_id(&self, vehicle_id: &str) -> Result<Option<VehicleInfo>> {
        let key = self.config.fleet_key_by_id(vehicle_id);
        self.fetch_cached(&key, || async {
            self.provider.fetch_vehicle_by_id(vehicle_id).await.context("fetching vehicle by id")
        })
        .await
    }

    pub async fn capacity_for_route(
        &self, vehicle_id: &str, route_id: &str,
    ) -> Result<Option<VehicleCapacity>> {
        let key = self.config.fleet_capacity_key(vehicle_id, route_id);
        self.fetch_cached(&key, || async {
            self.provider
                .fetch_vehicle_capacity(vehicle_id, route_id)
                .await
                .context("fetching vehicle capacity")
        })
        .await
    }

    pub async fn by_id_or_label(&self, vehicle_id_or_label: &str) -> Result<Option<VehicleInfo>> {
        if is_alpha_numeric(vehicle_id_or_label) {
            let (alpha, numeric) = split_alpha_numeric(vehicle_id_or_label);
            let alpha = if alpha == "AM" { "AMP".to_string() } else { alpha.to_string() };
            let mut vehicle_id = alpha;
            if vehicle_id.len() + numeric.len() < 14 {
                let padding = 14 - (vehicle_id.len() + numeric.len());
                vehicle_id.push_str(&" ".repeat(padding));
            }
            vehicle_id.push_str(numeric);
            return self.by_label(&vehicle_id).await;
        }

        if is_train_label(vehicle_id_or_label) {
            return self.by_label(vehicle_id_or_label).await;
        }

        self.by_id(vehicle_id_or_label).await
    }

    async fn fetch_cached<T, F, Fut>(&self, key: &str, loader: F) -> Result<Option<T>>
    where
        T: Serialize + DeserializeOwned + Default + Clone + Send + Sync,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Option<T>>> + Send,
    {
        if let Some(value) = self.cache.get_json::<T>(key)? {
            return Ok(Some(value));
        }

        match loader().await {
            Ok(Some(value)) => {
                self.cache.set_json_ex(key, CACHE_TTL_FLEET_SUCCESS, &value)?;
                Ok(Some(value))
            }
            Ok(None) => {
                self.cache.set_empty(key, CACHE_TTL_FLEET_SUCCESS)?;
                Ok(None)
            }
            Err(err) => {
                error!(key = key, error = %err, "fleet API error");
                self.cache.set_empty(key, CACHE_TTL_FLEET_FAILURE)?;
                Ok(None)
            }
        }
    }
}

// Aligns with TripMgtApi logic from legacy/at_smartrak_gtfs_adapter/src/apis/trip-mgt.ts.
#[derive(Debug, Clone)]
pub struct TripAccess<P: AdapterProvider> {
    config: Arc<Config>,
    provider: P,
    cache: Arc<CacheRepository>,
    parsed_trip_cache: DashMap<String, Vec<TripInstance>>, // assists reuse within same request
}

impl<P: AdapterProvider> TripAccess<P> {
    pub fn new(config: Arc<Config>, provider: P, cache: Arc<CacheRepository>) -> Self {
        Self { config, provider, cache, parsed_trip_cache: DashMap::new() }
    }

    pub async fn get_trip_instance(
        &self, trip_id: &str, service_date: &str, start_time: &str,
    ) -> Result<Option<TripInstance>> {
        let trips = self.get_trips(trip_id, service_date).await?;
        if let Some(trip) = trips.iter().find(|trip| trip.start_time == start_time) {
            return Ok(Some(trip.clone()));
        }
        if let Some(first) = trips.first() {
            if first.has_error() {
                return Ok(Some(first.clone()));
            }
        }
        Ok(None)
    }

    pub async fn get_nearest_trip_instance(
        &self, trip_id: &str, event_timestamp: i64, timezone: Tz,
    ) -> Result<Option<TripInstance>> {
        let Some(utc_dt) = Utc.timestamp_opt(event_timestamp, 0).single() else {
            return Ok(None);
        };
        let tz_dt = timezone.from_utc_datetime(&utc_dt.naive_utc());
        let current_hours: i64 = tz_dt.hour() as i64;
        let current_service_date = tz_dt.format("%Y%m%d").to_string();

        let mut trips = self.get_trips(trip_id, &current_service_date).await?;
        if trips.first().map_or(false, TripInstance::has_error) {
            return Ok(trips.into_iter().next());
        }

        if current_hours < 4 {
            let previous_date = (tz_dt - ChronoDuration::days(1)).format("%Y%m%d").to_string();
            let mut previous = self.get_trips(trip_id, &previous_date).await?;
            if previous.first().map_or(false, TripInstance::has_error) {
                return Ok(previous.into_iter().next());
            }
            trips.append(&mut previous);
        }

        if trips.is_empty() {
            return Ok(None);
        }

        trips.sort_by(|left, right| {
            let left_time =
                parse_trip_time(timezone, &left.service_date, &left.start_time).unwrap_or(i64::MAX);
            let right_time = parse_trip_time(timezone, &right.service_date, &right.start_time)
                .unwrap_or(i64::MAX);
            (event_timestamp - left_time).abs().cmp(&(event_timestamp - right_time).abs())
        });

        Ok(trips.into_iter().next())
    }

    async fn get_trips(&self, trip_id: &str, service_date: &str) -> Result<Vec<TripInstance>> {
        let cache_key = self.config.trip_mgt_key(trip_id, service_date);
        if let Some(entry) = self.parsed_trip_cache.get(&cache_key) {
            return Ok(entry.clone());
        }

        if let Some(trips) = self.cache.get_json::<Vec<TripInstance>>(&cache_key)? {
            self.parsed_trip_cache.insert(cache_key.clone(), trips.clone());
            return Ok(trips);
        }

        match self.provider.fetch_trip_instances(trip_id, service_date).await {
            Ok(trips) => {
                self.cache.set_json_ex(&cache_key, CACHE_TTL_TRIP_SUCCESS, &trips)?;
                self.parsed_trip_cache.insert(cache_key.clone(), trips.clone());
                Ok(trips)
            }
            Err(err) => {
                error!(trip_id = trip_id, service_date = service_date, error = %err, "trip management API error");
                let placeholder = vec![TripInstance::error_marker()];
                self.cache.set_json_ex(&cache_key, CACHE_TTL_TRIP_FAILURE, &placeholder)?;
                self.parsed_trip_cache.insert(cache_key.clone(), placeholder.clone());
                Ok(placeholder)
            }
        }
    }
}

// Mirrors BlockMgtApi behaviour from legacy/at_smartrak_gtfs_adapter/src/apis/block-mgt.ts.
#[derive(Debug, Clone)]
pub struct BlockAccess<P: AdapterProvider> {
    config: Arc<Config>,
    provider: P,
    cache: Arc<CacheRepository>,
}

impl<P: AdapterProvider> BlockAccess<P> {
    pub fn new(config: Arc<Config>, provider: P, cache: Arc<CacheRepository>) -> Self {
        Self { config, provider, cache }
    }

    pub async fn allocation(
        &self, vehicle_id: &str, timestamp: i64,
    ) -> Result<Option<BlockInstance>> {
        let key = self.config.block_key(vehicle_id);
        if let Some(block) = self.cache.get_json::<BlockInstance>(&key)? {
            if block.trip_id.is_empty() && !block.has_error() {
                return Ok(None);
            }
            return Ok(Some(block));
        }

        match self.provider.fetch_block_allocation(vehicle_id, timestamp).await {
            Ok(Some(block)) => {
                self.cache.set_json_ex(&key, CACHE_TTL_BLOCK_SUCCESS, &block)?;
                Ok(Some(block))
            }
            Ok(None) => {
                self.cache.set_empty(&key, CACHE_TTL_BLOCK_SUCCESS)?;
                Ok(None)
            }
            Err(err) => {
                error!(vehicle_id = vehicle_id, error = %err, "block management API error");
                let placeholder = BlockInstance { error: true, ..BlockInstance::default() };
                self.cache.set_json_ex(&key, CACHE_TTL_BLOCK_FAILURE, &placeholder)?;
                Ok(Some(placeholder))
            }
        }
    }
}

pub fn parse_trip_time(timezone: Tz, service_date: &str, time: &str) -> Option<i64> {
    if service_date.len() != 8 {
        return None;
    }
    let year = service_date[..4].parse::<i32>().ok()?;
    let month = service_date[4..6].parse::<u32>().ok()?;
    let day = service_date[6..8].parse::<u32>().ok()?;

    let mut parts = time.split(':');
    let hours = parts.next()?.parse::<i64>().ok()?;
    let minutes = parts.next()?.parse::<i64>().ok()?;
    let seconds = parts.next()?.parse::<i64>().ok()?;

    let base = timezone.with_ymd_and_hms(year, month, day, 0, 0, 0).single()?;
    let dt = base
        + ChronoDuration::hours(hours)
        + ChronoDuration::minutes(minutes)
        + ChronoDuration::seconds(seconds);
    Some(dt.timestamp())
}

fn is_alpha_numeric(input: &str) -> bool {
    if input.is_empty() {
        return false;
    }
    let mut seen_digit = false;
    for ch in input.chars() {
        if ch.is_ascii_uppercase() {
            if seen_digit {
                return false;
            }
        } else if ch.is_ascii_digit() {
            seen_digit = true;
        } else {
            return false;
        }
    }
    seen_digit
}

fn split_alpha_numeric(input: &str) -> (&str, &str) {
    let idx = input.find(|ch: char| ch.is_ascii_digit()).unwrap_or(input.len());
    input.split_at(idx)
}

fn is_train_label(input: &str) -> bool {
    input.len() == 14 && input.contains("  ")
}
