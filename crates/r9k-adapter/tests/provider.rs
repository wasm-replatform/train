#![allow(missing_docs)]

use core::panic;
use std::any::Any;
use std::error::Error;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use augentic_test::fetch::Fetcher;
use augentic_test::testdef::{TestDef, TestResult};
use augentic_test::{Fixture, PreparedTestCase};
use bytes::Bytes;
use http::{Request, Response};
use qwasr_sdk::{Config, HttpRequest, Identity, Message, Publisher};
use r9k_adapter::{R9kMessage, SmarTrakEvent, StopInfo};
use serde::Deserialize;

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
    pub input: Option<R9kMessage>,
    pub params: Option<ReplayTransform>,
    pub output: Option<ReplayOutput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ReplayOutput {
    Events(Vec<SmarTrakEvent>),
    Error(qwasr_sdk::Error),
}

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct ReplayTransform {
    pub delay: i32,
}

impl Fixture for Replay {
    type Error = qwasr_sdk::Error;
    type Input = R9kMessage;
    type Output = Vec<SmarTrakEvent>;
    type TransformParams = ReplayTransform;

    fn from_data(data_def: &TestDef<Self::Error>) -> Self {
        let input_str: Option<String> = data_def.input.as_ref().and_then(|v| {
            serde_json::from_value(v.clone()).expect("should deserialize input as XML String")
        });
        let input = input_str.map(|s| {
            let msg: R9kMessage =
                quick_xml::de::from_str(&s).expect("should deserialize R9kMessage");
            msg
        });
        let params: Option<Self::TransformParams> = data_def.params.as_ref().and_then(|v| {
            serde_json::from_value(v.clone()).expect("should deserialize transform parameters")
        });
        let Some(output_def) = &data_def.output else {
            return Self { input, params, output: None };
        };
        let output = match output_def {
            TestResult::Success(value) => serde_json::from_value(value.clone()).map_or_else(
                |_| panic!("should deserialize output as SmarTrak events"),
                |events| Some(ReplayOutput::Events(events)),
            ),
            TestResult::Failure(err) => Some(ReplayOutput::Error(err.clone())),
        };
        Self { input, params, output }
    }

    fn input(&self) -> Option<Self::Input> {
        self.input.clone()
    }

    fn params(&self) -> Option<Self::TransformParams> {
        self.params.clone()
    }

    fn transform<F>(&self, f: F) -> Self::Input
    where
        F: FnOnce(&Self::Input, Option<&Self::TransformParams>) -> Self::Input,
    {
        let Some(input) = &self.input else {
            return Self::Input::default();
        };
        f(input, self.params.as_ref())
    }

    fn output(&self) -> Option<Result<Self::Output, Self::Error>> {
        let output = self.output.as_ref()?;
        match output {
            ReplayOutput::Error(error) => Some(Err(error.clone())),
            ReplayOutput::Events(events) => {
                if events.is_empty() {
                    return None;
                }
                Some(Ok(events.clone()))
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
        match &self.session {
            // TODO: use test definition for static too.
            Session::Static(Static { stops, vehicles }) => {
                let data = match request.uri().path() {
                    "/gtfs/stops" => {
                        serde_json::to_vec(&stops).context("failed to serialize static stops")?
                    }
                    "/allocations/trips" => {
                        let query = request.uri().query().unwrap_or("");
                        let vehicles =
                            if query.contains("externalRefId=445") { &vec![] } else { vehicles };
                        serde_json::to_vec(&vehicles).expect("failed to serialize static vehicles")
                    }
                    _ => {
                        return Err(anyhow!("unknown path: {}", request.uri().path()));
                    }
                };
                let body = Bytes::from(data);
                Response::builder().status(200).body(body).context("failed to build response")
            }
            Session::Replay(PreparedTestCase { http_requests, .. }) => {
                let Some(http_requests) = http_requests else {
                    return Err(anyhow!("no http requests defined in replay session"));
                };
                let fetcher = Fetcher::new(http_requests);
                fetcher.fetch(&request)
            }
        }
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
