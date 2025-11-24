use anyhow::{Context, Result};
use bytes::Bytes;
use http::Method;
use http::header::AUTHORIZATION;
use http_body_util::Empty;
use serde::Deserialize;

use crate::{HttpRequest, Identity, Provider};

#[derive(Debug, Clone, Deserialize)]
pub struct AllocationEnvelope { #[serde(default)] pub all: Vec<VehicleAllocation> }

#[derive(Debug, Clone, Deserialize)]
pub struct VehicleAllocation { #[serde(rename = "vehicleLabel")] pub vehicle_label: String }

pub async fn vehicles_by_external_ref_id(ref_id: &str, provider: &impl Provider) -> Result<Vec<String>> {
    let block_mgt_url = std::env::var("BLOCK_MANAGEMENT_URL").context("getting BLOCK_MANAGEMENT_URL")?;
    let url = format!("{block_mgt_url}/allocations/trips?externalRefId={}&closestTrip=true", urlencoding::encode(ref_id));
    let token = Identity::access_token(provider).await?;

    let request = http::Request::builder()
        .method(Method::GET)
        .uri(url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Empty::<Bytes>::new())
        .context("building block_mgt request")?;

    let response = HttpRequest::fetch(provider, request).await.context("block mgt request failed")?;
    let body = response.into_body();
    let envelope: AllocationEnvelope = serde_json::from_slice(&body).context("deserializing allocation envelope")?;
    Ok(envelope.all.into_iter().map(|v| v.vehicle_label).collect())
}
