use std::convert::Infallible;
use std::str::FromStr;

use anyhow::{Context, Result};
use bytes::Bytes;
use http::Method;
use http::header::{CACHE_CONTROL, IF_NONE_MATCH};
use http_body_util::Empty;
use serde::{Deserialize, Serialize};
use warp_sdk::{Config, HttpRequest, Identity};

/// Retrieves a vehicle (train) by label.
///
/// # Errors
///
/// Returns an error when the fleet API request fails or the
/// response cannot be deserialized.
pub async fn vehicle<P>(vehicle_id: &str, provider: &P) -> Result<Option<Vehicle>>
where
    P: Config + HttpRequest + Identity,
{
    let identifier = Identifier::from_str(vehicle_id)?;
    let query = identifier.to_query();
    let fleet_url = Config::get(provider, "FLEET_URL").await.context("getting `FLEET_URL`")?;

    let request = http::Request::builder()
        .method(Method::GET)
        .uri(format!("{fleet_url}/vehicles?{query}"))
        .header(CACHE_CONTROL, "max-age=300") // 5 minutes
        .header(IF_NONE_MATCH, query)
        .header("Content-Type", "application/json")
        .body(Empty::<Bytes>::new())
        .context("building train_by_label request")?;

    let response =
        HttpRequest::fetch(provider, request).await.context("Fleet API request failed")?;

    let body = response.into_body();
    let records: Vec<Vehicle> =
        serde_json::from_slice(&body).context("Failed to deserialize Fleet API response")?;

    // get first vehicle that is a train
    let vehicle = records.into_iter().find(Vehicle::is_train);
    Ok(vehicle)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Vehicle {
    pub id: String,
    pub label: Option<String>,
    pub registration: Option<String>,
    pub capacity: Option<Capacity>,

    #[serde(rename = "type")]
    pub type_: Option<VehicleType>,
    pub tag: Option<String>,
}

impl Vehicle {
    #[must_use]
    pub fn is_train(&self) -> bool {
        self.type_
            .as_ref()
            .and_then(|t| t.kind.as_deref())
            .is_some_and(|t| t.eq_ignore_ascii_case("train"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capacity {
    pub seating: i64,
    pub standing: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VehicleType {
    #[serde(rename = "type")]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identifier {
    Label(String),
    Id(String),
}

impl Default for Identifier {
    fn default() -> Self {
        Self::Id(String::new())
    }
}

impl Identifier {
    #[must_use]
    pub fn to_query(&self) -> String {
        match self {
            Self::Label(label) => format!("label={}", urlencoding::encode(label)),
            Self::Id(id) => format!("id={}", urlencoding::encode(id)),
        }
    }
}

impl FromStr for Identifier {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Infallible> {
        let Some((prefix, suffix)) = ["AMP", "AM", "ADL", "AD"]
            .into_iter()
            .find_map(|prefix| s.strip_prefix(prefix).map(|suffix| (prefix, suffix)))
        else {
            return Ok(Self::Id(s.to_string()));
        };

        // train label: format as 14 characters
        let width = 14usize.saturating_sub(prefix.len());
        Ok(Self::Label(format!("{prefix}{suffix:>width$}")))
    }
}

#[cfg(test)]
mod tests {
    use super::Identifier;

    #[test]
    fn am_label() {
        let identifier = "AM123".parse().expect("valid label");
        assert_eq!(identifier, Identifier::Label("AM         123".to_string()));
        let Identifier::Label(s) = identifier else {
            panic!("Expected Identifier::Label");
        };
        assert_eq!(s.len(), 14);
    }

    #[test]
    fn amp_label() {
        let identifier = "AMP123".parse().expect("valid label");
        assert_eq!(identifier, Identifier::Label("AMP        123".to_string()));
        let Identifier::Label(s) = identifier else {
            panic!("Expected Identifier::Label");
        };
        assert_eq!(s.len(), 14);
    }

    #[test]
    fn ad_label() {
        let identifier = "AD123".parse().expect("valid label");
        assert_eq!(identifier, Identifier::Label("AD         123".to_string()));
        let Identifier::Label(s) = identifier else {
            panic!("Expected Identifier::Label");
        };
        assert_eq!(s.len(), 14);
    }

    #[test]
    fn adl_label() {
        let identifier = "ADL123".parse().expect("valid label");
        assert_eq!(identifier, Identifier::Label("ADL        123".to_string()));
        let Identifier::Label(s) = identifier else {
            panic!("Expected Identifier::Label");
        };
        assert_eq!(s.len(), 14);
    }

    #[test]
    fn already_padded() {
        let identifier = "AMP        123".parse().expect("valid label");
        assert_eq!(identifier, Identifier::Label("AMP        123".to_string()));
        let Identifier::Label(s) = identifier else {
            panic!("Expected Identifier::Label");
        };
        assert_eq!(s.len(), 14);
    }

    #[test]
    fn invalid_label() {
        assert_eq!("TRAIN".parse::<Identifier>().unwrap(), Identifier::Id("TRAIN".to_string()));
    }
}
