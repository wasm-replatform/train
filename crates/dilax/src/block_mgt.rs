use std::env;

use anyhow::{Context, Result};
use bytes::Bytes;
use http::Method;
use http::header::{AUTHORIZATION, CACHE_CONTROL, IF_NONE_MATCH};
use http_body_util::Empty;
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::provider::{HttpRequest, Identity, Provider};

// const TTL_FLEET_SUCCESS: Duration = Duration::from_secs(24 * 60 * 60);
// const TTL_FLEET_FAILURE: Duration = Duration::from_secs(3 * 60);

pub async fn vehicle(label: &str, http: &impl HttpRequest) -> Result<Option<FleetVehicle>> {
    let fleet_api_url = env::var("FLEET_URL").context("getting `FLEET_URL`")?;
    let url = format!("{fleet_api_url}/vehicles?label={}", urlencoding::encode(label));

    let request = http::Request::builder()
        .method(Method::GET)
        .uri(url)
        .header(CACHE_CONTROL, "max-age=300") // 5 minutes
        .header(IF_NONE_MATCH, label)
        .header("Content-Type", "application/json")
        .body(Empty::<Bytes>::new())
        .context("building train_by_label request")?;

    let response = http.fetch(request).await.context("Fleet API request failed")?;

    let body = response.into_body();
    let records: Vec<FleetVehicleRecord> =
        serde_json::from_slice(&body).context("Failed to deserialize Fleet API response")?;

    let vehicle = records
        .into_iter()
        .find(FleetVehicleRecord::is_train)
        .map(|record| FleetVehicle { id: record.id, capacity: record.capacity });

    Ok(vehicle)
}

async fn builder_helper(url: String, provider: &impl Provider) -> Result<http::request::Builder> {
    let mut builder = http::Request::builder()
        .method(Method::GET)
        .uri(url)
        .header("Content-Type", "application/json");

    if env::var("ENVIRONMENT").unwrap_or_default() == "dev" {
        let authorization = env::var("BLOCK_MGT_AUTHORIZATION").ok();
        if let Some(token) = authorization {
            builder = builder.header(AUTHORIZATION, token.as_str());
        }
    } else {
        let token = Identity::access_token(provider).await?;
        builder = builder.header(AUTHORIZATION, format!("Bearer {token}"));
    }

    Ok(builder)
}

pub async fn vehicle_allocation(
    vehicle_id: &str, provider: &impl Provider,
) -> Result<Option<VehicleAllocation>> {
    let block_mgt_url = env::var("BLOCK_MGT_URL").context("getting `BLOCK_MGT_URL`")?;
    let url = format!("{block_mgt_url}/allocations/vehicles/{vehicle_id}?currentTrip=true");

    let builder = builder_helper(url, provider).await?;

    let request =
        builder.body(Empty::<Bytes>::new()).context("building allocation_by_vehicle request")?;

    let response = HttpRequest::fetch(provider, request).await.map_err(|err| {
        Error::ServerError(format!(
            "failed to fetch block allocation for vehicle {vehicle_id}: {err}"
        ))
    })?;

    let body = response.into_body();
    let envelope: AllocationEnvelope =
        serde_json::from_slice(&body).context("Failed to decode allocation response")?;

    Ok(envelope.current.into_iter().next())
}

pub async fn allocations(provider: &impl Provider) -> Result<Vec<VehicleAllocation>> {
    let block_mgt_url = env::var("BLOCK_MGT_URL").context("getting `BLOCK_MGT_URL`")?;
    let url = format!("{block_mgt_url}/allocations");

    let builder = builder_helper(url, provider).await?;

    let request =
        builder.body(Empty::<Bytes>::new()).context("building all_allocations request")?;
    let response = HttpRequest::fetch(provider, request)
        .await
        .context("Block management list request failed")?;

    let body = response.into_body();
    let envelope: AllocationEnvelope =
        serde_json::from_slice(&body).context("Failed to decode allocations response")?;

    Ok(envelope.all)
}

#[derive(Clone, Default, Deserialize)]
struct AllocationEnvelope {
    #[serde(default)]
    current: Vec<VehicleAllocation>,

    #[serde(default)]
    all: Vec<VehicleAllocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleAllocation {
    #[serde(rename = "operationalBlockId")]
    pub operational_block_id: String,
    #[serde(rename = "tripId")]
    pub trip_id: String,
    #[serde(rename = "serviceDate")]
    pub service_date: String,
    #[serde(rename = "startTime")]
    pub start_time: String,
    #[serde(rename = "vehicleId")]
    pub vehicle_id: String,
    #[serde(rename = "vehicleLabel")]
    pub vehicle_label: String,
    #[serde(rename = "routeId")]
    pub route_id: String,
    #[serde(rename = "directionId")]
    pub direction_id: Option<u32>,
    #[serde(rename = "referenceId")]
    pub reference_id: String,
    #[serde(rename = "endTime")]
    pub end_time: String,
    pub delay: i64,
    #[serde(rename = "startDatetime")]
    pub start_datetime: i64,
    #[serde(rename = "endDatetime")]
    pub end_datetime: i64,
    #[serde(rename = "isCanceled")]
    pub is_canceled: bool,
    #[serde(rename = "isCopied")]
    pub is_copied: bool,
    pub timezone: String,
    #[serde(rename = "creationDatetime")]
    pub creation_datetime: String,
}

#[derive(Deserialize)]
struct FleetVehicleRecord {
    id: String,

    // #[serde(default)]
    // label: Option<String>,
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

#[derive(Debug, Clone, Deserialize)]
pub struct FleetVehicle {
    pub id: String,
    // #[serde(default)]
    // pub label: Option<String>,
    #[serde(default)]
    pub capacity: Option<VehicleCapacity>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VehicleCapacity {
    pub seating: i64,
    // pub standing: Option<i64>,
    pub total: i64,
}
