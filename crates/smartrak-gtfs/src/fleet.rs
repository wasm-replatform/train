use std::env;
use std::sync::LazyLock;

use anyhow::{Context, Result};
use bytes::Bytes;
use http::header::{CACHE_CONTROL, IF_NONE_MATCH};
use http::{Method, StatusCode};
use http_body_util::Empty;
use regex::Regex;
use serde_json::Value;
use tracing::{debug, warn};
use urlencoding::encode;

use crate::models::{VehicleCapacity, VehicleInfo};
use crate::provider::{HttpRequest, Provider};

/// Fetches vehicle metadata by label from the Fleet API.
///
/// # Errors
///
/// Returns an error when the Fleet API call fails or the response cannot be deserialized.
pub async fn get_vehicle_by_label(
    provider: &impl Provider, label: &str,
) -> Result<Option<VehicleInfo>> {
    fetch_vehicle(provider, format!("label={}", encode(label))).await
}

/// Fetches vehicle metadata by identifier from the Fleet API.
///
/// # Errors
///
/// Returns an error when the Fleet API call fails or the response cannot be deserialized.
pub async fn get_vehicle_by_id(
    provider: &impl Provider, vehicle_id: &str,
) -> Result<Option<VehicleInfo>> {
    fetch_vehicle(provider, format!("id={}", encode(vehicle_id))).await
}

/// Fetches vehicle capacity for a specific route.
///
/// # Errors
///
/// Returns an error when the Fleet API call fails or the response cannot be deserialized.
pub async fn get_vehicle_capacity_for_route(
    provider: &impl Provider, vehicle_id: &str, route_id: &str,
) -> Result<Option<VehicleCapacity>> {
    Ok(fetch_vehicle(provider, format!("id={}&route_id={}", encode(vehicle_id), encode(route_id)))
        .await?
        .map(|info| info.capacity))
}

/// Attempts to resolve a vehicle using multiple heuristics (label, train pattern, fallback id).
///
/// # Errors
///
/// Returns an error when the Fleet API call fails or the response cannot be deserialized.
pub async fn get_vehicle_by_id_or_label(
    provider: &impl Provider, vehicle_id_or_label: &str,
) -> Result<Option<VehicleInfo>> {
    if is_alphanumeric_label(vehicle_id_or_label)
        && let Some(label) = padded_train_label(vehicle_id_or_label)
    {
        if let Some(vehicle) = get_vehicle_by_label(provider, &label).await? {
            return Ok(Some(vehicle));
        }
    }

    if looks_like_train(vehicle_id_or_label) {
        return get_vehicle_by_label(provider, vehicle_id_or_label).await;
    }

    get_vehicle_by_id(provider, vehicle_id_or_label).await
}

async fn fetch_vehicle(provider: &impl Provider, query: String) -> Result<Option<VehicleInfo>> {
    let base_url = env::var("FLEET_API_URL").context("getting `FLEET_API_URL`")?;
    let endpoint = format!("{}/vehicles?{}", base_url.trim_end_matches('/'), query);

    let request = http::Request::builder()
        .method(Method::GET)
        .uri(&endpoint)
        .header(CACHE_CONTROL, "max-age=20") // 20 seconds
        .header(IF_NONE_MATCH, &query)
        .body(Empty::<Bytes>::new())
        .context("building Fleet API request")?;

    let response = HttpRequest::fetch(provider, request).await.context("calling Fleet API")?;

    let status = response.status();
    let body = response.into_body();

    if status == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !status.is_success() {
        warn!(%status, endpoint, "Fleet API request failed");
        return Ok(None);
    }

    if body.is_empty() {
        return Ok(None);
    }

    let value: Value = serde_json::from_slice(&body).context("parsing fleet payload")?;
    extract_vehicle(value)
}

fn extract_vehicle(value: Value) -> Result<Option<VehicleInfo>> {
    match value {
        Value::Null => Ok(None),
        Value::Array(items) => {
            for item in items {
                if matches!(&item, Value::Null)
                    || matches!(&item, Value::Object(map) if map.is_empty())
                {
                    continue;
                }
                let vehicle: VehicleInfo = serde_json::from_value(item)?;
                return Ok(Some(vehicle));
            }
            Ok(None)
        }
        Value::Object(mut map) => {
            if let Some(data) = map.remove("data") {
                return extract_vehicle(data);
            }

            if map.is_empty() {
                return Ok(None);
            }

            let vehicle: VehicleInfo = serde_json::from_value(Value::Object(map))?;
            Ok(Some(vehicle))
        }
        other => {
            let vehicle: VehicleInfo = serde_json::from_value(other)?;
            Ok(Some(vehicle))
        }
    }
}

fn is_alphanumeric_label(value: &str) -> bool {
    static REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^[A-Z]+\d+$").expect("failed to compile vehicle label regex")
    });
    REGEX.is_match(value)
}

fn looks_like_train(value: &str) -> bool {
    value.len() == 14 && value.contains("  ")
}

fn padded_train_label(raw: &str) -> Option<String> {
    let index = raw.find(|c: char| c.is_ascii_digit())?;
    let (mut alpha, num) = raw.split_at(index);
    if alpha == "AM" {
        alpha = "AMP";
    }

    let mut vehicle_id = alpha.to_string();
    let padding = 14usize.saturating_sub(alpha.len() + num.len());
    vehicle_id.push_str(&" ".repeat(padding));
    vehicle_id.push_str(num);
    debug!(raw, vehicle_id, "calculated padded train label");
    Some(vehicle_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pads_train_labels() {
        let label = padded_train_label("AM484").expect("label");
        assert_eq!(label, "AMP        484");
    }

    #[test]
    fn detects_alphanumeric() {
        assert!(is_alphanumeric_label("AB123"));
        assert!(!is_alphanumeric_label("AB 123"));
    }
}
