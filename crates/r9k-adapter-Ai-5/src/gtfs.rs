use crate::HttpRequest;
use anyhow::{Context, Result};
use bytes::Bytes;
use http::Method;
use http_body_util::Empty;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct StopInfo {
    #[serde(rename = "stop_code")]
    pub stop_code: String,
    #[serde(rename = "stop_lat")]
    pub stop_lat: f64,
    #[serde(rename = "stop_lon")]
    pub stop_lon: f64,
}

pub async fn stops(http: &impl HttpRequest) -> Result<Vec<StopInfo>> {
    let static_url = std::env::var("GTFS_CC_STATIC_URL")
        .or_else(|_| std::env::var("CC_STATIC_URL"))
        .context("getting static CC url")?;
    let url = format!("{static_url}/gtfs/stops?fields=stop_code,stop_lon,stop_lat");
    let request = http::Request::builder()
        .method(Method::GET)
        .uri(url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .body(Empty::<Bytes>::new())
        .context("building gtfs stops request")?;
    let response = http.fetch(request).await.context("gtfs stops request failed")?;
    let body = response.into_body();
    let stops: Vec<StopInfo> = serde_json::from_slice(&body).context("decoding gtfs stops")?;
    Ok(stops)
}
