use anyhow::{Context, Result};
use bytes::Bytes;
use http::Method;
use http::header::AUTHORIZATION;
use http::header::{CACHE_CONTROL, IF_NONE_MATCH};
use http_body_util::Empty;
use realtime::Config;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use urlencoding::encode;

use crate::models::VehicleInfo;
use crate::{HttpRequest, Identity, Provider};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VehicleIdentifier {
    Label(String),
    Id(String),
}

impl Default for VehicleIdentifier {
    fn default() -> Self {
        Self::Id(String::new())
    }
}

impl VehicleIdentifier {
    pub fn to_query(&self) -> String {
        match self {
            Self::Label(label) => format!("label={}", encode(label)),
            Self::Id(id) => format!("id={}", encode(id)),
        }
    }
}

impl FromStr for VehicleIdentifier {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let Some((prefix, suffix)) = s
            .strip_prefix("AMP")
            .map(|suffix| ("AMP", suffix))
            .or_else(|| s.strip_prefix("AM").map(|suffix| ("AM", suffix)))
        else {
            // not a train label, use as is
            return Ok(Self::Id(s.to_string()));
        };

        // train label: format as 14 characters
        let width = 14usize.saturating_sub(prefix.len());
        Ok(Self::Label(format!("{prefix}{suffix:>width$}")))
    }
}

// Attempts to resolve a vehicle using multiple heuristics (label, train
// pattern, fallback id).
pub async fn vehicle(
    identifier: &VehicleIdentifier, provider: &impl Provider,
) -> Result<Option<VehicleInfo>> {
    //     let query = maybe_train_label(vehicle_id).map_or_else(
    //         || format!("id={}", encode(vehicle_id)),
    //         |label| format!("label={}", encode(&label)),
    //     );
    //     fetch_vehicle(query, provider).await
    // }

    // async fn fetch_vehicle(query: String, provider: &impl Provider) -> Result<Option<VehicleInfo>> {
    let url = Config::get(provider, "FLEET_URL").await.context("getting `FLEET_URL`")?;
    let url = url.trim_end_matches('/');
    let query = identifier.to_query();

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

/// Retrieves the cached block allocation for a specific vehicle.
pub async fn allocation(
    vehicle_id: &str, timestamp: i64, provider: &impl Provider,
) -> Result<Option<BlockInstance>> {
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

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BlockInstance {
    pub trip_id: String,
    pub start_time: String,
    pub service_date: String,
    #[serde(default)]
    pub vehicle_ids: Vec<String>,
    #[serde(default)]
    pub error: bool,
}

impl BlockInstance {
    #[must_use]
    pub const fn has_error(&self) -> bool {
        self.error
    }
}

#[cfg(test)]
mod tests {
    use super::VehicleIdentifier;

    #[test]
    fn am_label() {
        let identifier = "AM123".parse().expect("valid label");
        assert_eq!(identifier, VehicleIdentifier::Label("AM         123".to_string()));
        let VehicleIdentifier::Label(s) = identifier else {
            panic!("Expected VehicleIdentifier::Label");
        };
        assert_eq!(s.len(), 14);
    }

    #[test]
    fn amp_label() {
        let identifier = "AMP123".parse().expect("valid label");
        assert_eq!(identifier, VehicleIdentifier::Label("AMP        123".to_string()));
        let VehicleIdentifier::Label(s) = identifier else {
            panic!("Expected VehicleIdentifier::Label");
        };
        assert_eq!(s.len(), 14);
    }

    #[test]
    fn already_padded() {
        let identifier = "AMP        123".parse().expect("valid label");
        assert_eq!(identifier, VehicleIdentifier::Label("AMP        123".to_string()));
        let VehicleIdentifier::Label(s) = identifier else {
            panic!("Expected VehicleIdentifier::Label");
        };
        assert_eq!(s.len(), 14);
    }

    #[test]
    fn invalid_label() {
        assert_eq!(
            "TRAIN".parse::<VehicleIdentifier>().unwrap(),
            VehicleIdentifier::Id("TRAIN".to_string())
        );
    }
}
