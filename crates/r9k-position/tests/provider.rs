#![allow(missing_docs)]

use std::env;

use anyhow::{Context, Result, anyhow};
use http::{Request, Response};
use r9k_position::{HttpRequest, Provider, StopInfo};
use serde::de::DeserializeOwned;

#[derive(Clone, Default)]
pub struct MockProvider {
    stops: Vec<StopInfo>,
    vehicles: Vec<String>,
}

impl MockProvider {
    #[allow(unused)]
    #[must_use]
    pub fn new() -> Self {
        // SAFETY:
        // This is safe in a test context as tests are run sequentially.
        unsafe {
            env::set_var("BLOCK_MGT_ADDR", "http://localhost:8080");
            env::set_var("GTFS_API_ADDR", "http://localhost:8080");
        };

        let stops = vec![
            StopInfo { stop_code: "133".to_string(), stop_lat: -36.12345, stop_lon: 174.12345 },
            StopInfo { stop_code: "134".to_string(), stop_lat: -36.54321, stop_lon: 174.54321 },
            StopInfo { stop_code: "9218".to_string(), stop_lat: -36.567, stop_lon: 174.44444 },
        ];
        let vehicles = vec!["vehicle 1".to_string()];

        Self { stops, vehicles }
    }
}

impl Provider for MockProvider {}

impl HttpRequest for MockProvider {
    async fn fetch<B: Sync, U: DeserializeOwned>(
        &self, request: &Request<B>,
    ) -> Result<Response<U>> {
        let data = match request.uri().path() {
            "/gtfs/stops" => {
                serde_json::to_vec(&self.stops).context("failed to serialize stops")?
            }
            "/allocations/trips" => {
                let query = request.uri().query().unwrap_or("");
                if query.contains("externalRefId=5226") {
                    serde_json::to_vec(&self.vehicles).context("failed to serialize")?
                } else {
                    serde_json::to_vec(&Vec::<String>::new()).context("failed to serialize")?
                }
            }
            _ => {
                return Err(anyhow!("unknown path: {}", request.uri().path()));
            }
        };

        let body = serde_json::from_slice::<U>(&data)?;
        Response::builder().status(200).body(body).context("failed to build response")
    }
}
