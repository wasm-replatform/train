#![allow(missing_docs)]

use std::any::Any;
use std::error::Error;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use http::{Request, Response};
use r9k_adapter::{R9kMessage, SmarTrakEvent, StopInfo};
use serde::Deserialize;
use test_utils::{Fixture, PreparedTestCase};
use warp_sdk::{Config, HttpRequest, Identity, Message, Publisher};

#[allow(dead_code)]
#[derive(Clone)]
pub enum Session {
    Static(Static),
    Replay(PreparedTestCase<Replay>),
}

#[derive(Clone)]
pub struct Static {
    pub stops: Vec<StopInfo>,
    pub vehicles: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Replay {
    pub input: String,
    pub params: Option<ReplayTransform>,
    pub extension: Option<ReplayExtension>,
    pub output: Option<ReplayOutput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ReplayOutput {
    Events(Vec<String>),
    Error(warp_sdk::Error),
}

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct ReplayTransform {
    pub delay: i32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReplayExtension {
    pub stop_info: Option<StopInfo>,
    pub vehicles: Option<Vec<String>>,
}

impl Fixture for Replay {
    type Error = warp_sdk::Error;
    type Extension = ReplayExtension;
    type Input = R9kMessage;
    type Output = Option<Vec<SmarTrakEvent>>;
    type TransformParams = ReplayTransform;

    fn input(&self) -> Self::Input {
        quick_xml::de::from_reader(self.input.as_bytes()).expect("should deserialize input")
    }

    fn params(&self) -> Option<Self::TransformParams> {
        self.params.clone()
    }

    fn extension(&self) -> Option<Self::Extension> {
        self.extension.clone()
    }

    fn output(&self) -> Option<Result<Self::Output, Self::Error>> {
        let output = self.output.as_ref()?;
        match output {
            ReplayOutput::Error(error) => Some(Err(error.clone())),
            ReplayOutput::Events(events) => {
                if events.is_empty() {
                    return Some(Ok(None));
                }
                let smartrak_events: Vec<SmarTrakEvent> = events
                    .iter()
                    .map(|e| serde_json::from_str(e).expect("should deserialize smartrak event"))
                    .collect();
                Some(Ok(Some(smartrak_events)))
            }
        }
    }
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

    #[allow(clippy::missing_panics_doc)]
    #[allow(dead_code)]
    #[must_use]
    pub fn events(&self) -> Vec<SmarTrakEvent> {
        self.events.lock().expect("should lock").clone()
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn new_replay(replay: PreparedTestCase<Replay>) -> Self {
        Self { session: Session::Replay(replay), events: Arc::new(Mutex::new(Vec::new())) }
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
                let stops: Vec<StopInfo> = match &self.session {
                    Session::Static(Static { stops, .. }) => stops.clone(),
                    Session::Replay(PreparedTestCase { extension, .. }) => {
                        extension.as_ref().and_then(|e| e.stop_info.clone()).into_iter().collect()
                    }
                };
                serde_json::to_vec(&stops).context("failed to serialize stops")?
            }
            "/allocations/trips" => {
                let query = request.uri().query().unwrap_or("");
                let vehicles = match &self.session {
                    Session::Static(Static { vehicles, .. }) => {
                        if query.contains("externalRefId=445") { &vec![] } else { vehicles }
                    }
                    Session::Replay(PreparedTestCase { extension, .. }) => {
                        extension.as_ref().and_then(|ext| ext.vehicles.as_deref()).unwrap_or(&[])
                    }
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
    async fn access_token(&self, _identity: String) -> Result<String> {
        Ok("mock_access_token".to_string())
    }
}
