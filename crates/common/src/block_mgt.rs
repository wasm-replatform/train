use anyhow::{Context, Result};
use bytes::Bytes;
use http::Method;
use http::header::{AUTHORIZATION, CACHE_CONTROL, IF_NONE_MATCH};
use http_body_util::Empty;
use serde::{Deserialize, Serialize};
use warp_sdk::{Config, HttpRequest, Identity};

/// Retrieves the block allocation for a specific vehicle.
///
/// # Errors
///
/// Returns an error when the block management API request fails or the
/// response cannot be deserialized.
pub async fn allocation<P>(vehicle_id: &str, provider: &P) -> Result<Option<Allocation>>
where
    P: Config + HttpRequest + Identity,
{
    let block_mgt_url =
        Config::get(provider, "BLOCK_MGT_URL").await.context("getting `BLOCK_MGT_URL`")?;
    let url = format!("{block_mgt_url}/allocations/vehicles/{vehicle_id}?currentTrip=true");
    let token = Identity::access_token(provider).await?;

    let request = http::Request::builder()
        .method(Method::GET)
        .uri(url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Empty::<Bytes>::new())
        .context("building allocation_by_vehicle request")?;

    let response = HttpRequest::fetch(provider, request)
        .await
        .with_context(|| format!("failed to fetch block allocation for vehicle {vehicle_id}"))?;

    let body = response.into_body();
    let envelope: AllocationResponse =
        serde_json::from_slice(&body).context("Failed to decode allocation response")?;

    Ok(envelope.current.into_iter().next())
}

/// Retrieves the cached block allocation for a specific vehicle.
///
/// # Errors
///
/// Returns an error when the block management API request fails or the
/// response cannot be deserialized.
pub async fn cached_allocation<P>(
    vehicle_id: &str, timestamp: i64, provider: &P,
) -> Result<Option<BlockInstance>>
where
    P: Config + HttpRequest + Identity,
{
    let url = Config::get(provider, "BLOCK_MGT_URL").await.context("getting `BLOCK_MGT_URL`")?;

    let token = Identity::access_token(provider).await?;
    let endpoint = format!(
        "{url}/allocations/vehicles/{vehicle_id}?currentTrip=true&siblings=true&nowUnixTimeSeconds={timestamp}"
    );

    let request = http::Request::builder()
        .uri(&endpoint)
        .method(Method::GET)
        .header(CACHE_CONTROL, "max-age=20") // 20 seconds
        .header(IF_NONE_MATCH, vehicle_id)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .body(Empty::<Bytes>::new())
        .context("building block management request")?;
    let response = HttpRequest::fetch(provider, request).await.context("fetching allocations")?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let body = response.into_body();
    let allocation: Option<BlockInstance> =
        serde_json::from_slice(&body).context("deserializing allocations")?;

    Ok(allocation)
}

/// Retrieves all block allocations.
///
/// # Errors
///
/// Returns an error when the block management API request fails or the
/// response cannot be deserialized.
pub async fn allocations<P>(provider: &P) -> Result<Vec<Allocation>>
where
    P: Config + HttpRequest + Identity,
{
    let block_mgt_url =
        Config::get(provider, "BLOCK_MGT_URL").await.context("getting `BLOCK_MGT_URL`")?;

    let url = format!("{block_mgt_url}/allocations");
    let token = Identity::access_token(provider).await?;

    let request = http::Request::builder()
        .method(Method::GET)
        .uri(url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Empty::<Bytes>::new())
        .context("building all_allocations request")?;

    let response = HttpRequest::fetch(provider, request)
        .await
        .context("Block management list request failed")?;

    let body = response.into_body();
    let envelope: AllocationResponse =
        serde_json::from_slice(&body).context("Failed to decode allocations response")?;

    Ok(envelope.all)
}

#[derive(Clone, Default, Deserialize)]
#[serde(default)]
struct AllocationResponse {
    current: Vec<Allocation>,
    all: Vec<Allocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Allocation {
    pub operational_block_id: String,
    pub trip_id: String,
    pub service_date: String,
    pub start_time: String,
    pub vehicle_id: String,
    pub vehicle_label: String,
    pub route_id: String,
    pub direction_id: Option<u32>,
    pub reference_id: String,
    pub end_time: String,
    pub delay: i64,
    pub start_datetime: i64,
    pub end_datetime: i64,
    pub is_canceled: bool,
    pub is_copied: bool,
    pub timezone: String,
    pub creation_datetime: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct BlockInstance {
    pub trip_id: String,
    pub start_time: String,
    pub service_date: String,
    pub vehicle_ids: Vec<String>,
    pub error: bool,
}

impl BlockInstance {
    #[must_use]
    pub const fn has_error(&self) -> bool {
        self.error
    }
}
