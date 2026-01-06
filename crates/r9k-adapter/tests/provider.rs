#![allow(missing_docs)]

use std::any::Any;
use std::error::Error;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use http::{Request, Response};
use r9k_adapter::{SmarTrakEvent, StopInfo};
use serde::{Serialize, Deserialize};
use warp_sdk::{Config, HttpRequest, Identity, Message, Publisher};

#[allow(dead_code)]
#[derive(Clone)]
pub enum Session {
    Static(Static),
    Replay(Replay),
}

#[derive(Clone)]
pub struct Static {
    pub stops: Vec<StopInfo>,
    pub vehicles: Vec<String>,
}

#[allow(dead_code)]
#[derive(Clone, Serialize, Deserialize)]
pub struct Replay {
    pub input: String,
    pub output: Option<Vec<String>>,
    pub error: Option<warp_sdk::Error>,
    pub delay: Option<i32>,
    pub stop_info: Option<StopInfo>,
    pub vehicles: Option<Vec<String>>,
}

#[derive(Clone)]
pub struct MockProvider {
    session: Session,
    events: Arc<Mutex<Vec<SmarTrakEvent>>>,
}

impl MockProvider {
    #[allow(dead_code)]
    #[must_use]
    pub fn new_static() -> Self {
        let session = Session::Static(Static {
            stops: vec![
                StopInfo { stop_code: "133".to_string(), stop_lat: -36.12345, stop_lon: 174.12345 },
                StopInfo { stop_code: "134".to_string(), stop_lat: -36.54321, stop_lon: 174.54321 },
                StopInfo { stop_code: "9218".to_string(), stop_lat: -36.567, stop_lon: 174.44444 },
            ],
            vehicles: vec!["vehicle 1".to_string()],
        });

        Self { session, events: Arc::new(Mutex::new(Vec::new())) }
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn new_replay(replay: Replay) -> Self {
        Self { session: Session::Replay(replay), events: Arc::new(Mutex::new(Vec::new())) }
    }

    #[allow(clippy::missing_panics_doc)]
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
                let stops = match &self.session {
                    Session::Static(Static { stops, .. }) => stops,
                    Session::Replay(Replay { stop_info, .. }) => {
                        &stop_info.as_ref().map(|s| vec![s.clone()]).unwrap_or_default()
                    }
                };
                serde_json::to_vec(stops).context("failed to serialize stops")?
            }
            "/allocations/trips" => {
                let query = request.uri().query().unwrap_or("");
                let vehicles = match &self.session {
                    Session::Static(Static { vehicles, .. }) => {
                        if query.contains("externalRefId=445") { &vec![] } else { vehicles }
                    }
                    Session::Replay(Replay { vehicles, .. }) => vehicles.as_deref().unwrap_or(&[]),
                };

                serde_json::to_vec(&vehicles).context("failed to serialize")?
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

#[cfg(test)]
mod tests {
    use warp_sdk::Error;

    use super::*;

    #[test]
    fn test_new_replay() {
        let replay = Replay {
            input: "test".to_string(),
            output: None,
            error: Some(Error::BadRequest {
                code: "bad_time".to_string(),
                description: "outdated by 506 seconds".to_string(),
            }),
            delay: None,
            stop_info: None,
            vehicles: None,
        };

        let ser_str = serde_json::to_string_pretty(&replay).unwrap();
        println!("{}", ser_str);
    }
}
