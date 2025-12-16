use anyhow::{Context, Result};
use bytes::Bytes;
use http::Method;
use http::header::{CACHE_CONTROL, IF_NONE_MATCH};
use http_body_util::Empty;
use realtime::Config;
use urlencoding::encode;

use crate::models::VehicleInfo;
use crate::{HttpRequest, Provider};

// Attempts to resolve a vehicle using multiple heuristics (label, train
// pattern, fallback id).
pub async fn get_vehicle(
    vehicle_id: &str, provider: &impl Provider,
) -> Result<Option<VehicleInfo>> {
    let query = maybe_train_label(vehicle_id).map_or_else(
        || format!("id={}", encode(vehicle_id)),
        |label| format!("label={}", encode(&label)),
    );
    fetch_vehicle(query, provider).await
}

async fn fetch_vehicle(query: String, provider: &impl Provider) -> Result<Option<VehicleInfo>> {
    let url = Config::get(provider, "FLEET_URL").await.context("getting `FLEET_URL`")?;
    let url = url.trim_end_matches('/');

    let request = http::Request::builder()
        .method(Method::GET)
        .uri(format!("{url}/vehicles?{query}"))
        .header(CACHE_CONTROL, "max-age=20")
        .header(IF_NONE_MATCH, &query)
        .body(Empty::<Bytes>::new())
        .context("building Fleet API request")?;

    let response = HttpRequest::fetch(provider, request).await.context("calling Fleet API")?;
    if !response.status().is_success() {
        return Ok(None);
    }

    // deserialize
    let body = response.into_body();
    let vehicles: Vec<VehicleInfo> =
        serde_json::from_slice(&body).context("deserializing fleet payload")?;

    // return first vehicle, if any
    Ok(vehicles.into_iter().next())
}

fn maybe_train_label(label: &str) -> Option<String> {
    let (prefix, suffix) = label
        .strip_prefix("AMP")
        .map(|suffix| ("AMP", suffix))
        .or_else(|| label.strip_prefix("AM").map(|suffix| ("AM", suffix)))?;
    let width = 14usize.saturating_sub(prefix.len());
    Some(format!("{prefix}{suffix:>width$}"))
}

#[cfg(test)]
mod tests {
    use super::maybe_train_label;

    #[test]
    fn am_label() {
        let got = maybe_train_label("AM123").expect("label should be padded");
        assert_eq!(got, "AM         123");
        assert_eq!(got.len(), 14);
    }

    #[test]
    fn amp_label() {
        let got = maybe_train_label("AMP123").expect("label should be padded");
        assert_eq!(got, "AMP        123");
        assert_eq!(got.len(), 14);
    }

    #[test]
    fn already_padded() {
        let got = maybe_train_label("AMP        123").expect("label should be padded");
        assert_eq!(got, "AMP        123");
        assert_eq!(got.len(), 14);
    }

    #[test]
    fn invalid_label() {
        assert_eq!(maybe_train_label("TRAIN"), None);
        assert_eq!(maybe_train_label(""), None);
    }
}
