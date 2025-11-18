use std::env;

use anyhow::{Context, Result};
use bytes::Bytes;
use http::{Method, StatusCode};
use http::header::{AUTHORIZATION, CACHE_CONTROL, IF_NONE_MATCH};
use http_body_util::Empty;
use tracing::warn;

use crate::models::BlockInstance;
use crate::provider::{HttpRequest, Identity, Provider};

//const TTL_SUCCESS: Duration = Duration::seconds(20);
//const TTL_FAILURE: Duration = Duration::seconds(10);

pub async fn get_allocation_by_vehicle(
    provider: &impl Provider, vehicle_id: &str, timestamp: i64,
) -> Result<Option<BlockInstance>> {
    let block_mgt_url = env::var("BLOCK_MGT_URL").context("getting `BLOCK_MGT_URL`")?;
    let token = Identity::access_token(provider).await?;
    let endpoint = format!(
        "{block_mgt_url}/allocations/vehicles/{vehicle_id}?currentTrip=true&siblings=true&nowUnixTimeSeconds={timestamp}"
    );

    let request = http::Request::builder()
        .uri(&endpoint)
        .method(Method::GET)
        .header(CACHE_CONTROL, "max-age=20") // 20 seconds
        .header(IF_NONE_MATCH, vehicle_id)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .body(Empty::<Bytes>::new())
        .context("building block management request")?;

    let response =
        HttpRequest::fetch(provider, request).await.context("fetching train allocations")?;

    let status = response.status();
    let body = response.into_body();

    if status == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !status.is_success() {
        warn!(%status, endpoint, "Block Management API request failed");
        return Ok(None);
    }

    if body.is_empty() {
        return Ok(None);
    }

    let allocation: Option<BlockInstance> =
        serde_json::from_slice(&body).context("deserializing block allocation response")?;

    Ok(allocation)
}
