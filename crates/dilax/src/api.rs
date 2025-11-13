use std::env;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use http::Method;
use http_body_util::Empty;
use serde::Deserialize;
use serde_json::from_slice;
use tracing::debug;

use crate::error::Error;
use crate::provider::HttpRequest;
use crate::store::KvStore;
use crate::types::{
    FleetVehicle, StopInfo, StopType, StopTypeEntry, VehicleAllocation, VehicleCapacity,
};
use crate::Result;

const FLEET_SUCCESS_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const FLEET_FAILURE_TTL: Duration = Duration::from_secs(3 * 60);
const GTFS_SUCCESS_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const GTFS_FAILURE_TTL: Duration = Duration::from_secs(60);

pub trait FleetProvider: Send + Sync {
    fn train_by_label(
        &self, label: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<FleetVehicle>>> + Send + '_>>;
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

    fn read_cache(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.cache
            .get_with_ttl(key)
            .map_err(|err| Error::CachingError(format!("fleet cache unavailable: {err}")))
    }

    fn cache_vehicle(&self, key: &str, vehicle: &FleetVehicle) -> Result<()> {
        self.cache
            .set_json_with_ttl(key, vehicle, FLEET_SUCCESS_TTL)
            .map_err(|err| Error::CachingError(format!("fleet cache update failed: {err}")))
    }

    fn cache_miss(&self, key: &str) -> Result<()> {
        self.cache
            .set_string_with_ttl(key, "null", FLEET_FAILURE_TTL)
            .map_err(|err| Error::CachingError(format!("fleet cache update failed: {err}")))
    }

    async fn train_by_label_async(&self, label: String) -> Result<Option<FleetVehicle>> {
        let cache_key = self.cache_key(&label);
        if let Some(bytes) = self.read_cache(&cache_key)? {
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
                    return Err(Error::InvalidFormat(format!(
                        "fleet cache payload invalid: {err}"
                    )));
                }
            }
        }
        let fleet_api_url = env::var("FLEET_API_URL")
            .context("getting `FLEET_API_URL`")
            .map_err(Error::from)?;
        let url = format!("{fleet_api_url}/vehicles?label={}", urlencoding::encode(&label));
        let request = http::Request::builder()
            .method(Method::GET)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(Empty::<Bytes>::new())
            .context("building train_by_label request")
            .map_err(Error::from)?;

        let response = self
            .http
            .fetch(request)
            .await
            .map_err(|err| Error::ServerError(format!("fleet API unavailable: {err}")))?;

        let body = response.into_body();
        let records: Vec<FleetVehicleRecord> = serde_json::from_slice(&body)
            .map_err(|err| Error::InvalidFormat(format!("fleet API payload invalid: {err}")))?;

        let vehicle = records.into_iter().find(FleetVehicleRecord::is_train).map(|record| {
            FleetVehicle { id: record.id, label: record.label, capacity: record.capacity }
        });

        vehicle.map_or_else(
            || {
                self.cache_miss(&cache_key)?;
                Ok(None)
            },
            |vehicle| {
                self.cache_vehicle(&cache_key, &vehicle)?;
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

    fn read_cache(&self, key: &str) -> Result<Option<Vec<StopTypeEntry>>> {
        self.cache
            .get_json_with_ttl(key)
            .map_err(|err| Error::CachingError(format!("gtfs cache unavailable: {err}")))
    }

    fn write_cache(&self, key: &str, entries: &[StopTypeEntry], ttl: Duration) -> Result<()> {
        self.cache
            .set_json_with_ttl(key, entries, ttl)
            .map_err(|err| Error::CachingError(format!("gtfs cache update failed: {err}")))
    }

    async fn train_stop_types_async(&self) -> Result<Vec<StopTypeEntry>> {
        const CACHE_KEY: &str = "gtfs:trainStops";

        if let Some(entries) = self.read_cache(CACHE_KEY)? {
            return Ok(entries);
        }
        let gtfs_static_url = env::var("GTFS_STATIC_URL")
            .context("getting `GTFS_STATIC_URL`")
            .map_err(Error::from)?;
        let url = format!("{gtfs_static_url}/stopstypes/");
        let request = http::Request::builder()
            .method(Method::GET)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(Empty::<Bytes>::new())
            .context("building train_stop_types request")
            .map_err(Error::from)?;

        let response = self
            .http
            .fetch(request)
            .await
            .map_err(|err| {
                if let Err(cache_err) = self.write_cache(CACHE_KEY, &[], GTFS_FAILURE_TTL) {
                    return cache_err;
                }
                Error::ServerError(format!("gtfs static unavailable: {err}"))
            })?;

        let body = response.into_body();
        let payload: StopTypesResponse = serde_json::from_slice(&body).map_err(|err| {
            if let Err(cache_err) = self.write_cache(CACHE_KEY, &[], GTFS_FAILURE_TTL) {
                return cache_err;
            }
            Error::InvalidFormat(format!("gtfs static payload invalid: {err}"))
        })?;

        let train_stops: Vec<StopTypeEntry> = payload
            .into_iter()
            .filter(|entry| entry.route_type == Some(StopType::TrainStop as u32))
            .collect();

        self.write_cache(CACHE_KEY, &train_stops, GTFS_SUCCESS_TTL)?;

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
        let cc_static_addr = env::var("CC_STATIC_API_URL")
            .context("getting `CC_STATIC_API_URL`")
            .map_err(Error::from)?;
        let url = format!(
            "{cc_static_addr}/gtfs/stops/geosearch?lat={lat}&lng={lon}&distance={distance}"
        );

        let request = http::Request::builder()
            .method(Method::GET)
            .uri(url)
            .header("Accept", "application/json; charset=utf-8")
            .header("Content-Type", "application/json")
            .body(Empty::<Bytes>::new())
            .context("building cc stops_by_location request")
            .map_err(Error::from)?;

        let response = self
            .http
            .fetch(request)
            .await
            .map_err(|err| Error::ServerError(format!("cc static unavailable: {err}")))?;

        let body = response.into_body();
        let stops: Vec<CcStopResponse> = serde_json::from_slice(&body)
            .map_err(|err| Error::InvalidFormat(format!("cc static payload invalid: {err}")))?;

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
