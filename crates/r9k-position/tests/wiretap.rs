//! Wiretap tests that compare recorded input and output of the Typescript adapter.
#![cfg(not(miri))]

mod provider;

use std::any::Any;
use std::error::Error;
use std::fs::{self, File};

use anyhow::{Context, Result, anyhow, bail};
use bytes::Bytes;
use chrono::{Timelike, Utc};
use chrono_tz::Pacific::Auckland;
use credibil_api::Client;
use http::{Request, Response};
use r9k_position::{HttpRequest, Identity, R9kMessage, SmarTrakEvent, StopInfo};
use serde::Deserialize;

/// This test runs through a folder of files that recorded the input and output
/// of the Typescript adapter.
#[tokio::test]
async fn wiretap() -> Result<()> {
    for entry in fs::read_dir("data/wiretap").expect("should read wiretap directory") {
        let entry = entry.expect("should read directory entry");
        let path = entry.path();
        let reader = File::open(&path).expect("should open file");

        match serde_yaml::from_reader::<_, Wiretap>(&reader) {
            Ok(w) => compare(w).await?,
            Err(e) => panic!("Failed to parse YAML in file {path:?}: {e}"),
        }
    }
    Ok(())
}

async fn compare(wiretap: Wiretap) -> Result<()> {
    let provider = MockProvider::new(wiretap.clone());
    let client = Client::new(provider);
    let mut request = R9kMessage::try_from(wiretap.input)?;

    let Some(change) = request.train_update.changes.get_mut(0) else {
        bail!("no changes in input message");
    };

    // correct event time to 'now' (+ originally recorded delay)
    let now = Utc::now().with_timezone(&Auckland);

    request.train_update.created_date = now.date_naive();
    #[allow(clippy::cast_possible_wrap)]
    let from_midnight = now.num_seconds_from_midnight() as i32;
    let adjusted_secs = wiretap.delay.map_or(from_midnight, |delay| from_midnight - delay);

    if change.has_departed {
        change.actual_departure_time = adjusted_secs;
    } else if change.has_arrived {
        change.actual_arrival_time = adjusted_secs;
    }

    let response = match client.request(request).owner("owner").await {
        Ok(r) => r,
        Err(e) => {
            assert_eq!(e, wiretap.error.unwrap());
            return Ok(());
        }
    };

    let Some(curr_events) = response.body.smartrak_events else {
        bail!("no SmarTrak events in response");
    };

    if wiretap.not_relevant_station.unwrap_or_default() {
        assert!(curr_events.is_empty());
    }
    if wiretap.not_relevant_type.unwrap_or_default() {
        assert!(curr_events.is_empty());
    }
    if curr_events.is_empty() {
        assert!(wiretap.output.is_none_or(|p| p.is_empty()));
        return Ok(());
    }

    let orig_events = wiretap.output.unwrap();
    assert_eq!(curr_events.len() * 2, orig_events.len(), "should be 2 publish events per message");

    orig_events.into_iter().zip(curr_events).for_each(|(published, mut actual)| {
        let original: SmarTrakEvent = serde_json::from_str(&published).unwrap();

        // add 5 seconds to the actual message timestamp the adapter sleeps 5 seconds
        // before output the first round
        let diff = now.timestamp() - actual.message_data.timestamp.timestamp();
        assert!(diff.abs() < 3, "expected vs actual too great: {diff}");

        // compare original published message to r9k event
        actual.received_at = original.received_at;
        actual.message_data.timestamp = original.message_data.timestamp;

        let json_actual = serde_json::to_value(&actual).unwrap();
        let json_expected: serde_json::Value = serde_json::from_str(&published).unwrap();
        assert_eq!(json_expected, json_actual);
    });

    Ok(())
}

struct MockProvider {
    wiretap: Wiretap,
}

impl MockProvider {
    #[allow(unused)]
    #[must_use]
    fn new(wiretap: Wiretap) -> Self {
        // SAFETY:
        // This is safe in a test context as tests are run sequentially.
        unsafe {
            std::env::set_var("BLOCK_MGT_URL", "http://localhost:8080");
            std::env::set_var("CC_STATIC_API_URL", "http://localhost:8080");
        };

        Self { wiretap }
    }
}

/// One recording session of the Typescript adapter.
#[derive(Deserialize, Clone)]
// #[allow(dead_code)]
struct Wiretap {
    input: String,
    delay: Option<i32>,
    stop_info: Option<StopInfo>,
    allocated_vehicles: Option<Vec<String>>,
    error: Option<r9k_position::Error>,
    not_relevant_type: Option<bool>,
    not_relevant_station: Option<bool>,
    output: Option<Vec<String>>,
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
                let stops =
                    self.wiretap.stop_info.as_ref().map(|s| vec![s.clone()]).unwrap_or_default();
                serde_json::to_vec(&stops).context("failed to serialize stops")?
            }
            "/allocations/trips" => {
                let vehicles = self.wiretap.allocated_vehicles.clone().unwrap_or_default();
                serde_json::to_vec(&vehicles).context("failed to serialize vehicles")?
            }
            _ => {
                return Err(anyhow!("unknown path: {}", request.uri().path()));
            }
        };

        let body = Bytes::from(data);
        Response::builder().status(200).body(body).context("failed to build response")
    }
}

impl Identity for MockProvider {
    async fn access_token(&self) -> Result<String> {
        Ok("mock_access_token".to_string())
    }
}
