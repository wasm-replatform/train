//! Wiretap tests that compare recorded input and output of the Typescript adapter.
#![cfg(not(miri))]

mod provider;

use std::any::Any;
use std::error::Error;
use std::fs::{self, File};

use anyhow::{Context, Result, anyhow, bail};
use bytes::Bytes;
use chrono::{Local, Timelike};
use credibil_api::Client;
use http::{Request, Response};
use r9k_position::{HttpRequest, Provider, R9kMessage, SmarTrakEvent, StopInfo};
use serde::Deserialize;

/// This test runs through a folder of files that recorded the input and output
/// of the Typescript adapter.
#[tokio::test]
async fn wiretap() -> Result<()> {
    let wiretap_dir = Path::new("data/wiretap");

    assert!(wiretap_dir.exists(), "Wiretap data directory should exist");

    for entry in fs::read_dir(wiretap_dir).expect("Should be able to read wiretap directory") {
        let entry = entry.expect("Should be able to read directory entry");
        let path = entry.path();
        println!("Wiretap file: {path:?}");

        let reader = fs::File::open(&path).expect("Should be able to open file");

        // Parse the YAML content
        match serde_yaml::from_reader::<_, Wiretap>(&reader) {
            Ok(w) => run_wiretap(w).await?,
            Err(e) => panic!("Failed to parse YAML in file {path:?}: {e}"),
        }
    }
    Ok(())
}

async fn run_wiretap(wiretap: Wiretap) -> Result<()> {
    let api = ApiClient::new(wiretap.clone());
    let request = R9kMessage::try_from(wiretap.input)?;
    let response = match api.request(request).owner("owner").await {
        Ok(r) => r,
        Err(e) => {
            assert_eq!(e, wiretap.error.unwrap());
            return Ok(());
        }
    };
    let events = response.body.smartrak_events;

    // Expect to not emit events of typescript skipped an irrelevant station.
    if let Some(b) = wiretap.not_relevant_station
        && b
    {
        assert_eq!(events.len(), 0);
    }

    // Expect to not emit events of typescript skipped an irrelevant update type.
    if let Some(b) = wiretap.not_relevant_type
        && b
    {
        assert_eq!(events.len(), 0);
    }

    if events.is_empty() {
        // If we actually didn't emit events, Typescript must have skipped.
        assert!(
            wiretap.not_relevant_type == Some(true) || wiretap.not_relevant_station == Some(true)
        );
        assert!(wiretap.publishing.is_none_or(|p| p.is_empty()));
    } else {
        // There must be publishing events.
        let publishing = wiretap.publishing.unwrap();

        // If we did emit events, they must correspond to the first wave of publishing after
        // 5 secs.
        assert_eq!(events.len() * 2, publishing.len(), "Expecting 2 TS publish events per event");

        publishing.into_iter().zip(events).for_each(|(publish, mut actual)| {
            let expected: SmarTrakEvent = serde_json::from_str(&publish.value).unwrap();

            // Kafka key is derived from the event.
            assert_eq!(publish.key, expected.remote_data.external_id);

            // Add 5 seconds to the actual message timestamp because typescript sleeps 5 seconds
            // before publishing the first round.
            let diff =
                expected.message_data.timestamp - (actual.message_data.timestamp + 5.seconds());

            // Add 5 seconds of tolerance because Typescript waits for Api responses before
            // publishing.
            assert!(
                diff.compare(3.seconds()).unwrap() == Ordering::Less,
                "expected - actual has to be < 3secs, got: {diff}"
            );

            // Overwrite the timestamp with the expected one so that we get a clean diff if
            // above assert passes.
            actual.message_data.timestamp = expected.message_data.timestamp;

            assert_eq!(expected, actual);

            // Finally compare generated json. Compare ``Value`s instead of strings because of key
            // ordering.
            let json_actual = serde_json::to_value(&actual).unwrap();
            let json_expected: serde_json::Value = serde_json::from_str(&publish.value).unwrap();
            assert_eq!(json_expected, json_actual);
        });
    }

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
            std::env::set_var("BLOCK_MGT_ADDR", "http://localhost:8080");
            std::env::set_var("GTFS_API_ADDR", "http://localhost:8080");
        };

        Self { wiretap }
    }
}

/// One recording session of the Typescript adapter.
#[derive(Deserialize, Clone)]
// #[allow(dead_code)]
struct Wiretap {
    input: String,
    // now: Option<i64>,
    // event_seconds: Option<i32>,
    // event_date: Option<i32>,
    delay: Option<i32>,
    stop_info: Option<StopInfo>,
    allocated_vehicles: Option<Vec<String>>,
    error: Option<r9k_position::Error>,
    not_relevant_type: Option<bool>,
    not_relevant_station: Option<bool>,
    output: Option<Vec<String>>,
}

impl Provider for MockProvider {}

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

// impl Time for Wiretap {
//     #[allow(clippy::option_if_let_else)]
//     fn now(&self) -> Zoned {
//         match self.now {
//             Some(s) => Zoned::new(
//                 Timestamp::from_millisecond(s).unwrap(),
//                 TimeZone::get("Pacific/Auckland").unwrap(),
//             ),
//             None => panic!("Wiretap data is missing now field"),
//         }
//     }
// }
