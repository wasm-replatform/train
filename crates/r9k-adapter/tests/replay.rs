//! Tests for expected success and failure outputs from the R9k adapter for a
//! set of inputs captured as snapshots from the live system.

mod provider;

use std::fs::{self, File};

use chrono::{Timelike, Utc};
use chrono_tz::Pacific::Auckland;
use r9k_adapter::R9kMessage;
use warp_sdk::Client;

use crate::provider::{Replay, ReplayTransform, TestCase};

// Load each test case. For each, present the input to the adapter and compare
// the output expected.
#[tokio::test]
async fn run() {
    for entry in fs::read_dir("data/sessions").expect("should read directory") {
        let file = File::open(entry.expect("should read entry").path()).expect("should open file");
        let fixture: Replay = serde_json::from_reader(&file).expect("should deserialize session");
        replay(fixture).await;
    }
}

async fn replay(fixture: Replay) {
    let test_case = TestCase::new(fixture).prepare(shift_time);
    let provider = provider::MockProvider::new_replay2(test_case.clone());
    let client = Client::new("at").provider(provider.clone());

    let result = client.request(test_case.input).await;
    let curr_events = provider.events();

    let Some(expected_result) = &test_case.output else {
        assert!(curr_events.is_empty());
        return;
    };

    match expected_result {
        Ok(expected_events) => {
            let Some(orig_events) = expected_events else {
                assert!(curr_events.is_empty());
                return;
            };
            orig_events.iter().zip(curr_events).for_each(|(published, mut actual)| {
                // add 5 seconds to the actual message timestamp the adapter sleeps 5 seconds
                // before output the first round
                let now = Utc::now().with_timezone(&Auckland);
                let diff = now.timestamp() - actual.message_data.timestamp.timestamp();
                assert!(diff.abs() < 3, "expected vs actual too great: {diff}");

                // compare original published message to r9k event
                actual.received_at = published.received_at;
                actual.message_data.timestamp = published.message_data.timestamp;

                let json_actual = serde_json::to_value(&actual).unwrap();
                let json_expected = serde_json::to_value(published).unwrap();
                assert_eq!(json_expected, json_actual);
            });
        }
        Err(expected_error) => {
            // Was the error the one defined in the fixture?
            let actual_error = result.expect_err("should have error");
            assert_eq!(actual_error.code(), expected_error.code());
            assert_eq!(actual_error.description(), expected_error.description());
        }
    }
}

fn shift_time(input: R9kMessage, params: Option<&ReplayTransform>) -> R9kMessage {
    if params.is_none() {
        return input;
    }
    let delay = params.as_ref().map_or(0, |p| p.delay);
    let mut request = input;
    let Some(change) = request.train_update.changes.get_mut(0) else {
        return request;
    };

    let now = Utc::now().with_timezone(&Auckland);
    request.train_update.created_date = now.date_naive();

    #[allow(clippy::cast_possible_wrap)]
    let from_midnight = now.num_seconds_from_midnight() as i32;
    let adjusted_secs = from_midnight - delay;

    if change.has_departed {
        change.actual_departure_time = adjusted_secs;
    } else if change.has_arrived {
        change.actual_arrival_time = adjusted_secs;
    }
    request
}
