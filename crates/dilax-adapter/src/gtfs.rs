use std::env;

use anyhow::{Context, Result};
use bytes::Bytes;
use http::Method;
use http::header::{CACHE_CONTROL, IF_NONE_MATCH};
use http_body_util::Empty;
use serde::{Deserialize, Serialize};

use crate::HttpRequest;

const KEY_TRAIN_STOPS: &str = "gtfs:trainStops";

type StopTypesResponse = Vec<StopTypeEntry>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CcStopResponse {
    #[serde(rename = "stop_id")]
    pub stop_id: String,
    #[serde(rename = "stop_code")]
    pub stop_code: Option<String>,
    #[serde(rename = "platform_code")]
    pub platform_code: Option<String>,
    #[serde(rename = "stop_name")]
    pub stop_name: Option<String>,
    #[serde(rename = "stop_desc")]
    pub stop_desc: Option<String>,
    #[serde(rename = "stop_lat")]
    pub stop_lat: Option<f64>,
    #[serde(rename = "stop_lon")]
    pub stop_lon: Option<f64>,
    #[serde(rename = "zone_id")]
    pub zone_id: Option<String>,
    #[serde(rename = "stop_url")]
    pub stop_url: Option<String>,
    #[serde(rename = "location_type")]
    pub location_type: Option<u8>,
    #[serde(rename = "parent_station")]
    pub parent_station: Option<String>,
    #[serde(rename = "stop_timezone")]
    pub stop_timezone: Option<String>,
    #[serde(rename = "wheelchair_boarding")]
    pub wheelchair_boarding: Option<u8>,
    #[serde(rename = "start_date")]
    pub start_date: Option<String>,
    #[serde(rename = "end_date")]
    pub end_date: Option<String>,
}

pub async fn location_stops(
    lat: &str, lon: &str, distance: u32, http: &impl HttpRequest,
) -> Result<Vec<StopInfo>> {
    let cc_static_addr = env::var("CC_STATIC_URL").context("getting `CC_STATIC_URL`")?;
    let url =
        format!("{cc_static_addr}/gtfs/stops/geosearch?lat={lat}&lng={lon}&distance={distance}");

    let request = http::Request::builder()
        .method(Method::GET)
        .uri(url)
        .header("Accept", "application/json; charset=utf-8")
        .header("Content-Type", "application/json")
        .body(Empty::<Bytes>::new())
        .context("building cc stops_by_location request")?;

    let response = http.fetch(request).await.context("CC Static  request failed")?;

    let body = response.into_body();
    let stops: Vec<CcStopResponse> =
        serde_json::from_slice(&body).context("Failed to decode CC Static response")?;

    Ok(stops
        .into_iter()
        .map(|stop| StopInfo { stop_id: stop.stop_id, stop_code: stop.stop_code })
        .collect())
}

pub async fn stop_types(http: &impl HttpRequest) -> Result<Vec<StopTypeEntry>> {
    let gtfs_static_url = env::var("GTFS_STATIC_URL").context("getting `GTFS_STATIC_URL`")?;
    let url = format!("{gtfs_static_url}/stopstypes/");

    let request = http::Request::builder()
        .method(Method::GET)
        .uri(url)
        .header(CACHE_CONTROL, "max-age=300") // 5 minutes
        .header(IF_NONE_MATCH, KEY_TRAIN_STOPS)
        .header("Content-Type", "application/json")
        .body(Empty::<Bytes>::new())
        .context("building train_stop_types request")?;

    let response = http.fetch(request).await.context("GTFS Static request failed")?;

    let body = response.into_body();
    let payload: StopTypesResponse =
        serde_json::from_slice(&body).context("Failed to decode GTFS Static response")?;

    let train_stops: Vec<StopTypeEntry> = payload
        .into_iter()
        .filter(|entry| entry.route_type == Some(StopType::Train as u32))
        .collect();

    Ok(train_stops)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum StopType {
    #[serde(rename = "2")]
    Train = 2,
    #[serde(rename = "3")]
    Bus = 3,
    #[serde(rename = "4")]
    Ferry = 4,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StopInfo {
    #[serde(rename = "stopId")]
    pub stop_id: String,
    #[serde(rename = "stopCode")]
    pub stop_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StopTypeEntry {
    #[serde(rename = "parent_stop_code")]
    pub parent_stop_code: Option<String>,
    #[serde(rename = "route_type")]
    pub route_type: Option<u32>,
    #[serde(rename = "stop_code")]
    pub stop_code: Option<String>,
}
