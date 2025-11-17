use std::env;

use anyhow::{Context, Result};
use bytes::Bytes;
use http::header::{CACHE_CONTROL, IF_NONE_MATCH};
use http::{Method, StatusCode};
use http_body_util::Empty;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use tracing::{debug, warn};
use urlencoding::encode;

use crate::models::{VehicleCapacity, VehicleInfo};
use crate::provider::{HttpRequest,Provider};

const CACHE_DIRECTIVE_PRIMARY: &str = "max-age=600, stale-if-error=60";

pub async fn get_vehicle_by_label(
    provider: &impl Provider, label: &str,
) -> Result<Option<VehicleInfo>> {
    fetch_vehicle(provider, format!("label={}", encode(label))).await
}

pub async fn get_vehicle_by_id(
    provider: &impl Provider, vehicle_id: &str,
) -> Result<Option<VehicleInfo>> {
    fetch_vehicle(provider, format!("id={}", encode(vehicle_id))).await
}

pub async fn get_vehicle_capacity_for_route(
    provider: &impl Provider, vehicle_id: &str, route_id: &str,
) -> Result<Option<VehicleCapacity>> {
    Ok(fetch_vehicle(provider, format!("id={}&route_id={}", encode(vehicle_id), encode(route_id)))
        .await?
        .map(|info| info.capacity))
}

pub async fn get_vehicle_by_id_or_label(
    provider: &impl Provider, vehicle_id_or_label: &str,
) -> Result<Option<VehicleInfo>> {
    if is_alphanumeric_label(vehicle_id_or_label) {
        if let Some(label) = padded_train_label(vehicle_id_or_label) {
            if let Some(vehicle) = get_vehicle_by_label(provider, &label).await? {
                return Ok(Some(vehicle));
            }
        }
    }

    if looks_like_train(vehicle_id_or_label) {
        if let Some(vehicle) = get_vehicle_by_label(provider, vehicle_id_or_label).await? {
            return Ok(Some(vehicle));
        }
    }

    get_vehicle_by_id(provider, vehicle_id_or_label).await
}

pub fn with_default_capacity(vehicle: &mut VehicleInfo) {
    if vehicle.vehicle_type.is_train() {
        let capacity = vehicle.capacity.total.unwrap_or(env_i64("DEFAULT_TRAIN_TOTAL_CAPACITY", 373));
        let seating = vehicle.capacity.seating.unwrap_or(env_i64("DEFAULT_TRAIN_SEATING_CAPACITY", 230));
        vehicle.capacity.total = Some(capacity);
        vehicle.capacity.seating = Some(seating);
    }
}

fn env_i64(key: &str, default: i64) -> i64 {
    env::var(key).ok().and_then(|value| value.parse::<i64>().ok()).unwrap_or(default)
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
    static REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^[A-Z]+\d+$").expect("failed to compile vehicle label regex"));
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
    vehicle_id.extend(std::iter::repeat(' ').take(padding));
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
