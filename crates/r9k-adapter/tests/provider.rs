#![allow(missing_docs)]

use std::any::Any;
use std::error::Error;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use http::{Request, Response};
// use quick_xml::reader::Config;
use r9k_adapter::{Config, HttpRequest, Identity, Publisher, SmarTrakEvent, StopInfo};

#[derive(Clone, Default)]
pub struct MockProvider {
    stops: Vec<StopInfo>,
    vehicles: Vec<String>,
    events: Arc<Mutex<Vec<SmarTrakEvent>>>,
}

impl MockProvider {
    #[allow(unused)]
    #[must_use]
    pub fn new() -> Self {
        let stops = vec![
            StopInfo { stop_code: "133".to_string(), stop_lat: -36.12345, stop_lon: 174.12345 },
            StopInfo { stop_code: "134".to_string(), stop_lat: -36.54321, stop_lon: 174.54321 },
            StopInfo { stop_code: "9218".to_string(), stop_lat: -36.567, stop_lon: 174.44444 },
        ];
        let vehicles = vec!["vehicle 1".to_string()];

        Self { stops, vehicles, events: Arc::new(Mutex::new(Vec::new())) }
    }

    #[allow(clippy::missing_panics_doc, unused)]
    #[must_use]
    pub fn events(&self) -> Vec<SmarTrakEvent> {
        self.events.lock().expect("should lock").clone()
    }
}

impl Config for MockProvider {
    async fn get(&self, _key: &str) -> Result<String> {
        // BLOCK_MGT_URL, CC_STATIC_URL
        Ok("http://localhost:8080".to_string())
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

impl Publisher for MockProvider {
    async fn send(&self, _topic: &str, message: &r9k_adapter::Message) -> Result<()> {
        let event: SmarTrakEvent =
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
