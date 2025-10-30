use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;

use http::Method;
use serde::Deserialize;
use serde_json::from_slice;
use tracing::{debug, warn};


use crate::store::KvStore;
use crate::types::{
    FleetVehicle, StopInfo, StopType, StopTypeEntry, VehicleAllocation, VehicleCapacity,
};
use crate::provider::HttpRequest;

const FLEET_SUCCESS_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const FLEET_FAILURE_TTL: Duration = Duration::from_secs(3 * 60);
const GTFS_SUCCESS_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const GTFS_FAILURE_TTL: Duration = Duration::from_secs(60);


#[async_trait]
pub trait FleetProvider: Send + Sync {
    async fn train_by_label(&self, label: &str) -> Result<Option<FleetVehicle>>;
}

#[async_trait]
pub trait BlockMgtProvider: Send + Sync {
    async fn allocation_by_vehicle(&self, vehicle_id: &str) -> Result<Option<VehicleAllocation>>;
    async fn all_allocations(&self) -> Result<Vec<VehicleAllocation>>;
}

#[async_trait]
pub trait GtfsStaticProvider: Send + Sync {
    async fn train_stop_types(&self) -> Result<Vec<StopTypeEntry>>;
}

#[async_trait]
pub trait CcStaticProvider: Send + Sync {
    async fn stops_by_location(&self, lat: &str, lon: &str, distance: u32)
    -> Result<Vec<StopInfo>>;
}

pub trait Clock: Send + Sync {
    fn now_utc(&self) -> DateTime<Utc>;
    fn timezone(&self) -> Tz;
}

#[derive(Clone)]
pub struct FleetApiProvider {
    cache: KvStore,
    base_url: String,
    cache_prefix: String,
    http: Arc<dyn HttpRequest>,
}

impl FleetApiProvider {
    pub fn new(
        cache: KvStore, base_url: String, cache_prefix: String, http: Arc<dyn HttpRequest>,
    ) -> Self {
        Self { cache, base_url, cache_prefix, http }
    }

    fn cache_key(&self, label: &str) -> String {
        format!("{}:{}", self.cache_prefix, label)
    }

    fn read_cache(&self, key: &str) -> Option<Vec<u8>> {
        match self.cache.get_with_ttl(key) {
            Ok(value) => value,
            Err(err) => {
                warn!(cache_key = key, error = %err, "Failed to read fleet cache");
                None
            }
        }
    }

    fn cache_vehicle(&self, key: &str, vehicle: &FleetVehicle) {
        if let Err(err) = self.cache.set_json_with_ttl(key, vehicle, FLEET_SUCCESS_TTL) {
            warn!(cache_key = key, error = %err, "Failed to persist fleet cache entry");
        }
    }

    fn cache_miss(&self, key: &str) {
        if let Err(err) = self.cache.set_string_with_ttl(key, "null", FLEET_FAILURE_TTL) {
            warn!(cache_key = key, error = %err, "Failed to persist fleet cache miss");
        }
    }
}

#[async_trait]
impl FleetProvider for FleetApiProvider {
    async fn train_by_label(&self, label: &str) -> Result<Option<FleetVehicle>> {
        let cache_key = self.cache_key(label);
        if let Some(bytes) = self.read_cache(&cache_key) {
            if bytes == b"null" {
                debug!(label = %label, "Fleet API cache hit: empty");
                return Ok(None);
            }
            match from_slice::<FleetVehicle>(&bytes) {
                Ok(vehicle) => {
                    debug!(label = %label, "Fleet API cache hit: vehicle");
                    return Ok(Some(vehicle));
                }
                Err(err) => {
                    warn!(label = %label, error = %err, "Failed to decode cached Fleet API payload");
                }
            }
        }

        let url = format!("{}/vehicles?label={label}", self.base_url);
        let request = http::Request::builder()
            .method(Method::GET)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(Vec::new())
            .context("building train_by_label request")?;

        let response = match self.http.fetch(request).await {
            Ok(resp) => resp,
            Err(err) => {
                warn!(label = %label, error = %err, "Fleet API request failed");
                self.cache_miss(&cache_key);
                return Ok(None);
            }
        };

        let records: Vec<FleetVehicleRecord> = match serde_json::from_slice(response.body()) {
            Ok(body) => body,
            Err(err) => {
                warn!(label = %label, error = %err, "Failed to deserialize Fleet API response");
                self.cache_miss(&cache_key);
                return Ok(None);
            }
        };

        let vehicle = records.into_iter().find(FleetVehicleRecord::is_train).map(|record| {
            FleetVehicle { id: record.id, label: record.label, capacity: record.capacity }
        });

        vehicle.map_or_else(
            || {
                self.cache_miss(&cache_key);
                Ok(None)
            },
            |vehicle| {
                self.cache_vehicle(&cache_key, &vehicle);
                Ok(Some(vehicle))
            },
        )
    }
}

#[derive(Deserialize)]
struct FleetVehicleRecord {
    id: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    capacity: Option<VehicleCapacity>,
    #[serde(default, rename = "type")]
    type_info: Option<FleetVehicleType>,
}

impl FleetVehicleRecord {
    fn is_train(&self) -> bool {
        self.type_info
            .as_ref()
            .and_then(|info| info.kind.as_deref())
            .is_some_and(|value| value.eq_ignore_ascii_case("train"))
    }
}

#[derive(Deserialize)]
struct FleetVehicleType {
    #[serde(rename = "type")]
    kind: Option<String>,
}

#[derive(Clone)]
pub struct BlockMgtClient {
    base_url: String,
    authorization: Option<String>,
    http: Arc<dyn HttpRequest>,
}

impl BlockMgtClient {
    pub fn new(
        base_url: String, bearer_token: Option<String>, http: Arc<dyn HttpRequest>,
    ) -> Self {
        let authorization =
            bearer_token.filter(|token| !token.is_empty()).map(|token| format!("Bearer {token}"));
        Self { base_url, authorization, http }
    }
}

#[async_trait]
impl BlockMgtProvider for BlockMgtClient {
    async fn allocation_by_vehicle(&self, vehicle_id: &str) -> Result<Option<VehicleAllocation>> {
        let url = format!("{}/allocations/vehicles/{}?currentTrip=true", self.base_url, vehicle_id);
        let mut builder = http::Request::builder()
            .method(Method::GET)
            .uri(url)
            .header("Content-Type", "application/json");

        if let Some(token) = &self.authorization {
            builder = builder.header("Authorization", token.as_str());
        }

        let request = builder
            .body(Vec::new())
            .context("building allocation_by_vehicle request")?;

        let response = match self.http.fetch(request).await {
            Ok(resp) => resp,
            Err(err) => {
                warn!(vehicle_id = %vehicle_id, error = %err, "Block management allocation request failed");
                return Ok(None);
            }
        };

        let envelope: AllocationEnvelope = match serde_json::from_slice(response.body()) {
            Ok(body) => body,
            Err(err) => {
                warn!(vehicle_id = vehicle_id, error = %err, "Failed to decode allocation response");
                return Ok(None);
            }
        };

        Ok(envelope.current.into_iter().next())
    }

    async fn all_allocations(&self) -> Result<Vec<VehicleAllocation>> {
        let url = format!("{}/allocations", self.base_url);
        let mut builder = http::Request::builder()
            .method(Method::GET)
            .uri(url)
            .header("Content-Type", "application/json");

        if let Some(token) = &self.authorization {
            builder = builder.header("Authorization", token.as_str());
        }

        let request = builder
            .body(Vec::new())
            .context("building all_allocations request")?;

        let response = match self.http.fetch(request).await {
            Ok(resp) => resp,
            Err(err) => {
                warn!(error = %err, "Block management list request failed");
                return Ok(Vec::new());
            }
        };

        let envelope: AllocationEnvelope = match serde_json::from_slice(response.body()) {
            Ok(body) => body,
            Err(err) => {
                warn!(error = %err, "Failed to decode allocations response");
                return Ok(Vec::new());
            }
        };

        Ok(envelope.all)
    }
}

#[derive(Clone, Default, Deserialize)]
struct AllocationEnvelope {
    #[serde(default)]
    current: Vec<VehicleAllocation>,
    #[serde(default)]
    all: Vec<VehicleAllocation>,
}

#[derive(Clone)]
pub struct GtfsStaticProviderImpl {
    cache: KvStore,
    base_url: String,
    http: Arc<dyn HttpRequest>,
}

impl GtfsStaticProviderImpl {
    pub fn new(cache: KvStore, base_url: String, http: Arc<dyn HttpRequest>) -> Self {
        Self { cache, base_url, http }
    }

    fn read_cache(&self, key: &str) -> Option<Vec<StopTypeEntry>> {
        match self.cache.get_json_with_ttl(key) {
            Ok(value) => value,
            Err(err) => {
                warn!(cache_key = key, error = %err, "Failed to read GTFS cache");
                None
            }
        }
    }

    fn write_cache(&self, key: &str, entries: &[StopTypeEntry], ttl: Duration) {
        if let Err(err) = self.cache.set_json_with_ttl(key, &entries.to_vec(), ttl) {
            warn!(cache_key = key, error = %err, "Failed to persist GTFS cache entry");
        }
    }
}

#[async_trait]
impl GtfsStaticProvider for GtfsStaticProviderImpl {
    async fn train_stop_types(&self) -> Result<Vec<StopTypeEntry>> {
        const CACHE_KEY: &str = "gtfs:trainStops";

        if let Some(entries) = self.read_cache(CACHE_KEY) {
            return Ok(entries);
        }

        let url = format!("{}/stopstypes/", self.base_url);
        let request = http::Request::builder()
            .method(Method::GET)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(Vec::new())
            .context("building train_stop_types request")?;

        let response = match self.http.fetch(request).await {
            Ok(resp) => resp,
            Err(err) => {
                warn!(error = %err, "GTFS Static request failed");
                self.write_cache(CACHE_KEY, &[], GTFS_FAILURE_TTL);
                return Ok(Vec::new());
            }
        };

        let payload: StopTypesResponse = match serde_json::from_slice(response.body()) {
            Ok(body) => body,
            Err(err) => {
                warn!(error = %err, "Failed to decode GTFS Static response");
                self.write_cache(CACHE_KEY, &[], GTFS_FAILURE_TTL);
                return Ok(Vec::new());
            }
        };

        let train_stops: Vec<StopTypeEntry> = payload
            .data
            .into_iter()
            .filter(|entry| entry.route_type == StopType::TrainStop as u32)
            .collect();

        self.write_cache(CACHE_KEY, &train_stops, GTFS_SUCCESS_TTL);

        Ok(train_stops)
    }
}

#[derive(Default, Deserialize)]
struct StopTypesResponse {
    #[serde(default)]
    data: Vec<StopTypeEntry>,
}

#[derive(Clone)]
pub struct CcStaticProviderImpl {
    base_url: String,
    http: Arc<dyn HttpRequest>,
}

impl CcStaticProviderImpl {
    pub fn new(base_url: String, http: Arc<dyn HttpRequest>) -> Self {
        Self { base_url, http }
    }
}

#[async_trait]
impl CcStaticProvider for CcStaticProviderImpl {
    async fn stops_by_location(
        &self, lat: &str, lon: &str, distance: u32,
    ) -> Result<Vec<StopInfo>> {
        let url = format!(
            "{}/gtfs/stops/geosearch?lat={lat}&lng={lon}&distance={distance}",
            self.base_url
        );

        let request = http::Request::builder()
            .method(Method::GET)
            .uri(url)
            .header("Accept", "application/json; charset=utf-8")
            .header("Content-Type", "application/json")
            .body(Vec::new())
            .context("building train_stop_types request")?;

        let response = match self.http.fetch(request).await {
            Ok(resp) => resp,
            Err(err) => {
                warn!(lat = %lat, lon = %lon, error = %err, "CC Static request failed");
                return Ok(Vec::new());
            }
        };

        let stops: Vec<CcStopResponse> = match serde_json::from_slice(response.body()) {
            Ok(body) => body,
            Err(err) => {
                warn!(lat = %lat, lon = %lon, error = %err, "Failed to decode CC Static response");
                return Ok(Vec::new());
            }
        };

        let results = stops
            .into_iter()
            .map(|stop| StopInfo { stop_id: stop.stop_id, stop_code: stop.stop_code })
            .collect();
        Ok(results)
    }
}

#[derive(Deserialize)]
struct CcStopResponse {
    #[serde(rename = "stop_id")]
    stop_id: String,
    #[serde(rename = "stop_code")]
    stop_code: Option<String>,
}
