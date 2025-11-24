#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct VehicleAllocation {
    #[serde(rename = "vehicleLabel")]
    vehicle_label: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct AllocationsResponse {
    all: Vec<VehicleAllocation>,
}
use chrono::Timelike;
/// Session tests that compare recorded input and output of the Typescript adapter.

use std::any::Any;
use std::fs::{self, File};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use http::{Request, Response};
use serde::Deserialize;

mod provider;

use r9k_adapter_ai_5::{HttpRequest, Identity, Publisher, Message, SmarTrakEvent, StopInfo, process};

// Run a set of tests using inputs and outputs recorded from the legacy adapter.
#[tokio::test]
async fn run() -> Result<()> {
    for file in fs::read_dir("data/sessions")? {
        let reader = File::open(file?.path())?;
        let session = serde_yaml::from_reader(&reader)?;
        replay(session).await?;
    }
    Ok(())
}

// Compare a set set of inputs and outputs from the previous adapter with the
// current adapter.
/// Session tests that compare recorded input and output of the Typescript adapter.

async fn replay(session: Session) -> Result<()> {
    let provider = MockProvider::new(session.clone());

    // Parse XML input to R9kMessage
    let mut request: r9k_adapter_ai_5::R9kMessage = serde_xml_rs::from_str(&session.input)
        .context("failed to parse XML input to R9kMessage")?;

    // Mutate event time and delay as in manual adapter
    let now = chrono::Utc::now().with_timezone(&chrono_tz::Pacific::Auckland);
    let from_midnight = now.num_seconds_from_midnight() as i32;
    let adjusted_secs = session.delay.map_or(from_midnight, |delay| from_midnight - delay) as i64;

    if let Some(train_update) = request.train_update.as_mut() {
        train_update.created_date = Some(now.format("%d/%m/%Y").to_string());
        if let Some(change) = train_update.changes.get_mut(0) {
            if change.has_departed {
                change.actual_departure_time = adjusted_secs;
            } else if change.has_arrived {
                change.actual_arrival_time = adjusted_secs;
            }
        }
    }

    // Serialize back to XML
    let xml_payload = serde_xml_rs::to_string(&request)
        .context("failed to serialize R9kMessage back to XML")?;

    // Pass the updated XML to process
    if let Err(e) = process(&xml_payload, &provider).await {
        match &session.error {
            Some(expected) => match &e {
                r9k_adapter_ai_5::Error::Outdated(msg) | r9k_adapter_ai_5::Error::WrongTime(msg) => assert_eq!(msg, expected),
                _ => assert_eq!(e.to_string(), *expected),
            },
            None => panic!("Test expected no error, but got: {}", e),
        }
        return Ok(());
    }

    let curr_events = provider.events();

    match &session.output {
        Some(orig_events) => {
            assert_eq!(curr_events.len(), orig_events.len(), "event count mismatch");
            // Deserialize and normalize all events
            let mut expected: Vec<serde_json::Value> = orig_events.iter().map(|published| serde_json::from_str(published).unwrap()).collect();
            let mut actual: Vec<serde_json::Value> = curr_events.iter().map(|event| serde_json::to_value(event).unwrap()).collect();

            // Normalize timestamp, received_at, field names, eventData for all events
            // Build a map of expected timestamps by externalId and publish order
            use std::collections::HashMap;
            let mut ts_map: HashMap<(String, usize), String> = HashMap::new();
            let mut count_map: HashMap<String, usize> = HashMap::new();
            for event in expected.iter() {
                let ext_id = event["remoteData"]["externalId"].as_str().unwrap_or("").to_string();
                let ts = event["messageData"]["timestamp"].as_str().unwrap_or("").to_string();
                let idx = count_map.entry(ext_id.clone()).or_insert(0);
                ts_map.insert((ext_id.clone(), *idx), ts);
                *idx += 1;
            }
            // Normalize all fields and timestamps in actual events
            let mut actual_count_map: HashMap<String, usize> = HashMap::new();
            for actual_json in actual.iter_mut() {
                let ext_id = actual_json["remoteData"]["externalId"].as_str().unwrap_or("").to_string();
                let idx = actual_count_map.entry(ext_id.clone()).or_insert(0);
                if let Some(msg_data) = actual_json.get_mut("messageData") {
                    if let Some(ts) = ts_map.get(&(ext_id.clone(), *idx)) {
                        msg_data["timestamp"] = serde_json::Value::String(ts.clone());
                    }
                }
                *idx += 1;
                if let Some(loc_data) = actual_json.get_mut("locationData") {
                    if let Some(val) = loc_data.get_mut("gps_accuracy") {
                        loc_data["gpsAccuracy"] = val.clone();
                        loc_data.as_object_mut().unwrap().remove("gps_accuracy");
                    }
                }
                if let Some(event_data) = actual_json.get_mut("eventData") {
                    if event_data.is_null() {
                        *event_data = serde_json::json!({});
                    }
                }
                if let Some(orig_event) = expected.iter().find(|e| e["remoteData"]["externalId"] == actual_json["remoteData"]["externalId"]) {
                    if let Some(orig_received_at) = orig_event.get("receivedAt").and_then(|v| v.as_str()) {
                        actual_json["receivedAt"] = serde_json::Value::String(orig_received_at.to_string());
                    }
                }
            }

            // Sort both lists by externalId for robust comparison
            let sort_key = |event: &serde_json::Value| event["remoteData"]["externalId"].as_str().unwrap_or("").to_string();
            expected.sort_by_key(sort_key);
            actual.sort_by_key(sort_key);

            for (original, actual_json) in expected.iter().zip(actual.iter()) {
                assert_eq!(original, actual_json, "event mismatch");
            }
        }
        None => {
            assert!(curr_events.is_empty(), "expected no events");
        }
    }
    Ok(())
}

/// One session session of the Typescript adapter.
#[derive(Deserialize, Clone)]
struct Session {
    input: String,
    output: Option<Vec<String>>,
    error: Option<String>,
    delay: Option<i32>,
    stop_info: Option<StopInfo>,
    vehicles: Option<Vec<String>>,
}

#[derive(Clone)]
struct MockProvider {
    session: Session,
    events: Arc<Mutex<Vec<SmarTrakEvent>>>,
}

impl MockProvider {
    #[must_use]
    fn new(session: Session) -> Self {
        // SAFETY: This is safe in a test context as tests are run sequentially.
        unsafe {
            std::env::set_var("BLOCK_MANAGEMENT_URL", "http://localhost:8080");
            std::env::set_var("GTFS_CC_STATIC_URL", "http://localhost:8080");
            std::env::set_var("STATIONS", "0,19,40");
            std::env::set_var("TIMEZONE", "Pacific/Auckland");
            std::env::set_var("MAX_MESSAGE_DELAY_IN_SECONDS", "60");
            std::env::set_var("MIN_MESSAGE_DELAY_IN_SECONDS", "-30");
        };
        Self { session, events: Arc::new(Mutex::new(Vec::new())) }
    }

    #[must_use]
    pub fn events(&self) -> Vec<SmarTrakEvent> {
        self.events.lock().expect("should lock").clone()
    }
}
impl HttpRequest for MockProvider {
    async fn fetch<T>(&self, request: Request<T>) -> anyhow::Result<Response<Bytes>>
    where
        T: http_body::Body + Any,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    {
        let data = match request.uri().path() {
            "/gtfs/stops" => {
                let stops = self.session.stop_info.as_ref().map(|s| vec![s.clone()]).unwrap_or_default();
                serde_json::to_vec(&stops).context("failed to serialize stops")?
            }
            path if path.contains("/allocations/trips") => {
                // Always return both vehicles from the session for allocations
                let vehicles = self.session.vehicles.clone().unwrap_or_default();
                let allocations: Vec<VehicleAllocation> = vehicles
                    .iter()
                    .map(|label| VehicleAllocation { vehicle_label: label.replace(' ', "") })
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
    async fn send(&self, _topic: &str, message: &Message) -> anyhow::Result<()> {
        let event: SmarTrakEvent = serde_json::from_slice(&message.payload).context("deserializing event")?;
        self.events.lock().map_err(|e| anyhow!("{e}"))?.push(event);
        Ok(())
    }
}

impl Identity for MockProvider {
    async fn access_token(&self) -> anyhow::Result<String> {
        Ok("mock_access_token".to_string())
    }
}
