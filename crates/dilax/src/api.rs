use std::env;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use http::Method;
use http_body_util::Empty;
use serde::Deserialize;
use serde_json::from_slice;
use tracing::{debug, warn};

use crate::provider::HttpRequest;
use crate::store::KvStore;
use crate::types::{
    FleetVehicle, StopInfo, StopType, StopTypeEntry, VehicleAllocation, VehicleCapacity,
};

const FLEET_SUCCESS_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const FLEET_FAILURE_TTL: Duration = Duration::from_secs(3 * 60);
const GTFS_SUCCESS_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const GTFS_FAILURE_TTL: Duration = Duration::from_secs(60);

pub trait FleetProvider: Send + Sync {
    fn train_by_label(
        &self, label: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<FleetVehicle>>> + Send + '_>>;
}

pub trait BlockMgtProvider: Send + Sync {
    fn allocation_by_vehicle(
        &self, vehicle_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<VehicleAllocation>>> + Send + '_>>;
    fn all_allocations(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<VehicleAllocation>>> + Send + '_>>;
}

pub trait GtfsStaticProvider: Send + Sync {
    fn train_stop_types(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<StopTypeEntry>>> + Send + '_>>;
}

pub trait CcStaticProvider: Send + Sync {
    fn stops_by_location(
        &self, lat: &str, lon: &str, distance: u32,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<StopInfo>>> + Send + '_>>;
}

pub trait Clock: Send + Sync {
    fn now_utc(&self) -> DateTime<Utc>;
    fn timezone(&self) -> Tz;
}

#[derive(Clone)]
pub struct FleetApiProvider<H>
where
    H: HttpRequest + ?Sized,
{
    cache: KvStore,
    cache_prefix: String,
    http: Arc<H>,
}

#[allow(clippy::missing_const_for_fn)]
impl<H> FleetApiProvider<H>
where
    H: HttpRequest + ?Sized,
{
    pub fn new(cache: KvStore, cache_prefix: String, http: Arc<H>) -> Self {
        Self { cache, cache_prefix, http }
    }

    fn cache_key(&self, label: &str) -> String {
        format!("{}:{label}", self.cache_prefix)
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

    async fn train_by_label_async(&self, label: String) -> Result<Option<FleetVehicle>> {
        let cache_key = self.cache_key(&label);
        if let Some(bytes) = self.read_cache(&cache_key) {
            if bytes.as_slice() == b"null" {
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
        let fleet_api_url = env::var("FLEET_API_URL").context("getting `FLEET_API_URL`")?;
        let url = format!("{fleet_api_url}/vehicles?label={}", urlencoding::encode(&label));
        let request = http::Request::builder()
            .method(Method::GET)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(Empty::<Bytes>::new())
            .context("building train_by_label request")?;

        let response = match self.http.fetch(request).await {
            Ok(resp) => resp,
            Err(err) => {
                warn!(label = %label, error = %err, "Fleet API request failed");
                self.cache_miss(&cache_key);
                return Ok(None);
            }
        };

        let body = response.into_body();
        let records: Vec<FleetVehicleRecord> = match serde_json::from_slice(&body) {
            Ok(payload) => payload,
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

impl<H> FleetProvider for FleetApiProvider<H>
where
    H: HttpRequest + ?Sized,
{
    fn train_by_label(
        &self, label: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<FleetVehicle>>> + Send + '_>> {
        let this = self;
        let owned_label = label.to_owned();
        Box::pin(async move { this.train_by_label_async(owned_label).await })
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
pub struct BlockMgtClient<H>
where
    H: HttpRequest + ?Sized,
{
    http: Arc<H>,
}

#[allow(clippy::missing_const_for_fn)]
impl<H> BlockMgtClient<H>
where
    H: HttpRequest + ?Sized,
{
    pub fn new(http: Arc<H>) -> Self {
        Self { http }
    }

    async fn allocation_by_vehicle_async(
        &self, vehicle_id: String,
    ) -> Result<Option<VehicleAllocation>> {
        let block_mgt_url = env::var("BLOCK_MGT_URL").context("getting `BLOCK_MGT_URL`")?;
        let url = format!("{block_mgt_url}/allocations/vehicles/{vehicle_id}?currentTrip=true");
        let mut builder = http::Request::builder()
            .method(Method::GET)
            .uri(url)
            .header("Content-Type", "application/json");

        if env::var("ENVIRONMENT").unwrap_or_default() == "dev" {
            let authorization = env::var("BLOCK_MGT_AUTHORIZATION").ok();
            if let Some(token) = authorization {
                builder = builder.header("Authorization", token.as_str());
            }
        }

        let request = builder
            .body(Empty::<Bytes>::new())
            .context("building allocation_by_vehicle request")?;

        let response = match self.http.fetch(request).await {
            Ok(resp) => resp,
            Err(err) => {
                warn!(vehicle_id = %vehicle_id, error = %err, "Block management allocation request failed");
                return Ok(None);
            }
        };

        let body = response.into_body();
        let envelope: AllocationEnvelope = match serde_json::from_slice(&body) {
            Ok(payload) => payload,
            Err(err) => {
                warn!(vehicle_id = %vehicle_id, error = %err, "Failed to decode allocation response");
                return Ok(None);
            }
        };

        Ok(envelope.current.into_iter().next())
    }

    async fn all_allocations_async(&self) -> Result<Vec<VehicleAllocation>> {
        let block_mgt_url = env::var("BLOCK_MGT_URL").context("getting `BLOCK_MGT_URL`")?;
        let url = format!("{block_mgt_url}/allocations");
        let mut builder = http::Request::builder()
            .method(Method::GET)
            .uri(url)
            .header("Content-Type", "application/json");

        if env::var("ENVIRONMENT").unwrap_or_default() == "dev" {
            let authorization = env::var("BLOCK_MGT_AUTHORIZATION").ok();
            if let Some(token) = authorization {
                builder = builder.header("Authorization", token.as_str());
            }
        }

        let request =
            builder.body(Empty::<Bytes>::new()).context("building all_allocations request")?;

        let response = match self.http.fetch(request).await {
            Ok(resp) => resp,
            Err(err) => {
                warn!(error = %err, "Block management list request failed");
                return Ok(Vec::new());
            }
        };

        let body = response.into_body();
        let envelope: AllocationEnvelope = match serde_json::from_slice(&body) {
            Ok(payload) => payload,
            Err(err) => {
                warn!(error = %err, "Failed to decode allocations response");
                return Ok(Vec::new());
            }
        };

        Ok(envelope.all)
    }
}

impl<H> BlockMgtProvider for BlockMgtClient<H>
where
    H: HttpRequest + ?Sized,
{
    fn allocation_by_vehicle(
        &self, vehicle_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<VehicleAllocation>>> + Send + '_>> {
        let this = self;
        let owned_vehicle_id = vehicle_id.to_owned();
        Box::pin(async move { this.allocation_by_vehicle_async(owned_vehicle_id).await })
    }

    fn all_allocations(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<VehicleAllocation>>> + Send + '_>> {
        let this = self;
        Box::pin(async move { this.all_allocations_async().await })
    }
}

#[derive(Clone, Default, Deserialize)]
struct AllocationEnvelope {
    #[serde(default)]
    current: Vec<VehicleAllocation>,
    #[serde(default)]
    all: Vec<VehicleAllocation>,
}

/*#[derive(Default, Deserialize)]
struct StopTypesResponse {
    #[serde(default)]
    data: Vec<StopTypeEntry>,
}*/

type StopTypesResponse = Vec<StopTypeEntry>;

#[derive(Clone)]
pub struct GtfsStaticProviderImpl<H>
where
    H: HttpRequest + ?Sized,
{
    cache: KvStore,
    http: Arc<H>,
}

#[allow(clippy::missing_const_for_fn)]
impl<H> GtfsStaticProviderImpl<H>
where
    H: HttpRequest + ?Sized,
{
    pub fn new(cache: KvStore, http: Arc<H>) -> Self {
        Self { cache, http }
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

    async fn train_stop_types_async(&self) -> Result<Vec<StopTypeEntry>> {
        const CACHE_KEY: &str = "gtfs:trainStops";

        if let Some(entries) = self.read_cache(CACHE_KEY) {
            return Ok(entries);
        }
        let gtfs_static_url =
            env::var("GTFS_STATIC_URL").context("getting `GTFS_STATIC_URL`")?;
        let url = format!("{gtfs_static_url}/stopstypes/");
        let request = http::Request::builder()
            .method(Method::GET)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(Empty::<Bytes>::new())
            .context("building train_stop_types request")?;

        let response = match self.http.fetch(request).await {
            Ok(resp) => resp,
            Err(err) => {
                warn!(error = %err, "GTFS Static request failed");
                self.write_cache(CACHE_KEY, &[], GTFS_FAILURE_TTL);
                return Ok(Vec::new());
            }
        };

        let body = response.into_body();
        let payload: StopTypesResponse = match serde_json::from_slice(&body) {
            Ok(data) => data,
            Err(err) => {
                warn!(error = %err, "Failed to decode GTFS Static response");
                self.write_cache(CACHE_KEY, &[], GTFS_FAILURE_TTL);
                return Ok(Vec::new());
            }
        };

        let train_stops: Vec<StopTypeEntry> = payload
            .into_iter()
            .filter(|entry| entry.route_type == Some(StopType::TrainStop as u32))
            .collect();

        self.write_cache(CACHE_KEY, &train_stops, GTFS_SUCCESS_TTL);

        Ok(train_stops)
    }
}

impl<H> GtfsStaticProvider for GtfsStaticProviderImpl<H>
where
    H: HttpRequest + ?Sized,
{
    fn train_stop_types(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<StopTypeEntry>>> + Send + '_>> {
        let this = self;
        Box::pin(async move { this.train_stop_types_async().await })
    }
}

#[derive(Clone)]
pub struct CcStaticProviderImpl<H>
where
    H: HttpRequest + ?Sized,
{
    http: Arc<H>,
}

#[allow(clippy::missing_const_for_fn)]
impl<H> CcStaticProviderImpl<H>
where
    H: HttpRequest + ?Sized,
{
    pub fn new(http: Arc<H>) -> Self {
        Self { http }
    }

    async fn stops_by_location_async(
        &self, lat: String, lon: String, distance: u32,
    ) -> Result<Vec<StopInfo>> {
        let cc_static_addr = env::var("CC_STATIC_API_URL").context("getting `CC_STATIC_API_URL`")?;
        let url = format!(
            "{cc_static_addr}/gtfs/stops/geosearch?lat={lat}&lng={lon}&distance={distance}"
        );

        let request = http::Request::builder()
            .method(Method::GET)
            .uri(url)
            .header("Accept", "application/json; charset=utf-8")
            .header("Content-Type", "application/json")
            .body(Empty::<Bytes>::new())
            .context("building cc stops_by_location request")?;

        let response = match self.http.fetch(request).await {
            Ok(resp) => resp,
            Err(err) => {
                warn!(lat = %lat, lon = %lon, error = %err, "CC Static request failed");
                return Ok(Vec::new());
            }
        };

        let body = response.into_body();
        let stops: Vec<CcStopResponse> = match serde_json::from_slice(&body) {
            Ok(payload) => payload,
            Err(err) => {
                warn!(lat = %lat, lon = %lon, error = %err, "Failed to decode CC Static response");
                return Ok(Vec::new());
            }
        };

        Ok(stops
            .into_iter()
            .map(|stop| StopInfo { stop_id: stop.stop_id, stop_code: stop.stop_code })
            .collect())
    }
}

impl<H> CcStaticProvider for CcStaticProviderImpl<H>
where
    H: HttpRequest + ?Sized,
{
    fn stops_by_location(
        &self, lat: &str, lon: &str, distance: u32,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<StopInfo>>> + Send + '_>> {
        let this = self;
        let owned_lat = lat.to_owned();
        let owned_lon = lon.to_owned();
        Box::pin(async move { this.stops_by_location_async(owned_lat, owned_lon, distance).await })
    }
}

#[derive(Deserialize)]
struct CcStopResponse {
    #[serde(rename = "stop_id")]
    stop_id: String,
    #[serde(rename = "stop_code")]
    stop_code: Option<String>,
}
