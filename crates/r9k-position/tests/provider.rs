#![allow(missing_docs)]

use std::any::Any;
use std::env;
use std::error::Error;

use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use http::{Request, Response};
use r9k_position::{HttpRequest, Provider, StopInfo};

#[derive(Clone, Default)]
pub struct AppContext {
    stops: HashMap<&'static str, StopInfo>,
    vehicles: HashMap<&'static str, String>,
}

impl AppContext {
    #[must_use]
    pub fn new() -> Self {
        let stops = HashMap::from([
            (
                "133",
                StopInfo { stop_code: "133".to_string(), stop_lat: -36.12345, stop_lon: 174.12345 },
            ),
            (
                "134",
                StopInfo { stop_code: "134".to_string(), stop_lat: -36.20299, stop_lon: 174.76915 },
            ),
            (
                "9218",
                StopInfo { stop_code: "9218".to_string(), stop_lat: -36.567, stop_lon: 174.44444 },
            ),
        ]);
        let vehicles = HashMap::from([("5226", "vehicle 1".to_string())]);

        Self { stops, vehicles }
    }
}

impl Provider for AppContext {}

impl HttpRequest for MockProvider {
    async fn fetch<T>(&self, request: Request<T>) -> Result<Response<Bytes>>
    where
        T: http_body::Body + Any,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn Error + Send + Sync + 'static>>,
    {
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

        let body = Bytes::from(data);
        Response::builder().status(200).body(body).context("failed to build response")
    }
}
