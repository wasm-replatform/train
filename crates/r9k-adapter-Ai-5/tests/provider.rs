#![allow(missing_docs)]

use std::any::Any;
use std::env;
use std::error::Error;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use http::{Request, Response};
use r9k_adapter_ai_5::{HttpRequest, Identity, Message, Publisher};
use serde::{Deserialize, Serialize};

/// Stop information from GTFS static data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopInfo {
    pub stop_code: String,
    pub stop_lat: f64,
    pub stop_lon: f64,
}

/// A vehicle allocation response item.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VehicleAllocation {
    #[serde(rename = "vehicleLabel")]
    vehicle_label: String,
}

/// Envelope for vehicle allocations.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AllocationsResponse {
    all: Vec<VehicleAllocation>,
}

#[derive(Clone, Default)]
pub struct MockProvider {
    stops: Vec<StopInfo>,
    vehicles: Vec<String>,
    events: Arc<Mutex<Vec<serde_json::Value>>>,
}

impl MockProvider {
    #[allow(unused)]
    #[must_use]
    pub fn new() -> Self {
        // SAFETY: This is safe in a test context as tests are run sequentially.
        unsafe {
            env::set_var("BLOCK_MANAGEMENT_URL", "http://localhost:8080");
            env::set_var("GTFS_CC_STATIC_URL", "http://localhost:8080");
            env::set_var("STATIONS", "0,19,40");
            env::set_var("TIMEZONE", "Pacific/Auckland");
            env::set_var("MAX_MESSAGE_DELAY_IN_SECONDS", "60");
            env::set_var("MIN_MESSAGE_DELAY_IN_SECONDS", "-30");
            env::set_var("R9K_TWO_TAP_DELAY_MS", "0");
        };

        let stops = vec![
            StopInfo { stop_code: "133".to_string(), stop_lat: -36.84448, stop_lon: 174.76915 },
            StopInfo { stop_code: "134".to_string(), stop_lat: -37.20299, stop_lon: 174.90990 },
            StopInfo { stop_code: "9218".to_string(), stop_lat: -36.99412, stop_lon: 174.87700 },
        ];
        let vehicles = vec!["vehicle1".to_string()];

        Self { stops, vehicles, events: Arc::new(Mutex::new(Vec::new())) }
    }

    #[allow(clippy::missing_panics_doc, unused)]
    #[must_use]
    pub fn events(&self) -> Vec<serde_json::Value> {
        self.events.lock().expect("should lock").clone()
    }
}

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
            path if path.contains("/allocations/trips") => {
                let query = request.uri().query().unwrap_or("");
                let vehicles = if query.contains("externalRefId=5226") {
                    self.vehicles.clone()
                } else {
                    Vec::<String>::new()
                };
                
                let allocations: Vec<VehicleAllocation> = vehicles
                    .into_iter()
                    .map(|label| VehicleAllocation { vehicle_label: label })
                    .collect();
                let response = AllocationsResponse { all: allocations };
                serde_json::to_vec(&response).context("failed to serialize allocations")?
            }
            _ => {
                return Err(anyhow!("unknown path: {}", request.uri().path()));
            }
        };

        let body = Bytes::from(data);
        Response::builder().status(200).body(body).context("failed to build response")
    }
}

impl Publisher for MockProvider {
    async fn send(&self, _topic: &str, message: &Message) -> Result<()> {
        let event: serde_json::Value =
            serde_json::from_slice(&message.payload).context("deserializing event")?;
        self.events.lock().map_err(|e| anyhow!("{e}"))?.push(event);
        Ok(())
    }
}

impl Identity for MockProvider {
    async fn access_token(&self) -> Result<String> {
        Ok("mock_access_token".to_string())
    }
}
