#![allow(missing_docs)]

use std::any::Any;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use http::{Request, Response};
use dilax_adapter::{EnrichedEvent, HttpRequest, Identity, Message, Publisher, StateStore};

#[derive(Clone, Default)]
pub struct MockProvider {
    events: Arc<Mutex<Vec<EnrichedEvent>>>,
    kv: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl MockProvider {
    #[must_use]
    pub fn new() -> Self {
        // SAFETY: single-threaded test env
        unsafe {
            env::set_var("FLEET_URL", "http://mock.local");
            env::set_var("BLOCK_MGT_URL", "http://mock.local");
            env::set_var("CC_STATIC_URL", "http://mock.local");
            env::set_var("GTFS_STATIC_URL", "http://mock.local");
        }
        Self { events: Arc::new(Mutex::new(Vec::new())), kv: Arc::new(Mutex::new(HashMap::new())) }
    }

    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn events(&self) -> Vec<EnrichedEvent> {
        self.events.lock().expect("lock").clone()
    }
}

impl HttpRequest for MockProvider {
    async fn fetch<T>(&self, request: Request<T>) -> Result<Response<Bytes>>
    where
        T: http_body::Body + Any,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn Error + Send + Sync + 'static>>,
    {
        let path = request.uri().path();
        let data = if path.ends_with("/vehicles") {
            // fleet vehicles lookup
            serde_json::to_vec(&vec![serde_json::json!({
                "id": "veh-1",
                "capacity": {"seating": 200, "total": 300},
                "type": {"type": "train"}
            })])?
        } else if path.contains("/allocations/vehicles/") {
            // block allocation
            serde_json::to_vec(&serde_json::json!({
                "current": [
                    {
                        "operationalBlockId": "blk-1",
                        "tripId": "trip-123",
                        "serviceDate": "20250101",
                        "startTime": "08:00:00",
                        "vehicleId": "veh-1",
                        "vehicleLabel": "AMP       1005",
                        "routeId": "R1",
                        "directionId": 0,
                        "referenceId": "ref-1",
                        "endTime": "09:00:00",
                        "delay": 0,
                        "startDatetime": 0,
                        "endDatetime": 0,
                        "isCanceled": false,
                        "isCopied": false,
                        "timezone": "Pacific/Auckland",
                        "creationDatetime": "2025-01-01T00:00:00Z"
                    }
                ],
                "all": []
            }))?
        } else if path.contains("/gtfs/stops/geosearch") {
            // nearby stops
            serde_json::to_vec(&vec![serde_json::json!({
                "stop_id": "STOP1",
                "stop_code": "133"
            })])?
        } else if path.ends_with("/stopstypes/") {
            // stop types
            serde_json::to_vec(&vec![serde_json::json!({
                "parent_stop_code": "133",
                "route_type": 2,
                "stop_code": null
            })])?
        } else {
            return Err(anyhow!("unknown path: {path}"));
        };

        let body = Bytes::from(data);
        Response::builder().status(200).body(body).context("building response")
    }
}

impl Publisher for MockProvider {
    async fn send(&self, _topic: &str, message: &Message) -> Result<()> {
        let enriched: EnrichedEvent = serde_json::from_slice(&message.payload)?;
        println!("ENRICHED_EVENT: {}", serde_json::to_string_pretty(&enriched)?);
        self.events.lock().map_err(|e| anyhow!("{e}"))?.push(enriched);
        Ok(())
    }
}

impl StateStore for MockProvider {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.kv.lock().unwrap().get(key).cloned())
    }
    #[allow(clippy::significant_drop_tightening)]
    async fn set(&self, key: &str, value: &[u8], _ttl_secs: Option<u64>) -> Result<Option<Vec<u8>>> {
        let mut kv = self.kv.lock().unwrap();
        let prev = kv.insert(key.to_string(), value.to_vec());
        Ok(prev)
    }
    async fn delete(&self, key: &str) -> Result<()> {
        self.kv.lock().unwrap().remove(key);
        Ok(())
    }
}

impl Identity for MockProvider {
    async fn access_token(&self) -> Result<String> {
        Ok("mock_access_token".to_string())
    }
}
