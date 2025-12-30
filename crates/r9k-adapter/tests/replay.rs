//! Session tests that compare recorded input and output of the Typescript adapter.
#![cfg(not(miri))]

mod provider;

use std::fs::{self, File};

use anyhow::{Result, bail};
use chrono::{Timelike, Utc};
use chrono_tz::Pacific::Auckland;
use fabric::api::Client;
use r9k_adapter::{R9kMessage, SmarTrakEvent};

use self::provider::MockProvider;
use crate::provider::Replay;

// Run a set of tests using inputs and outputs recorded from the legacy adapter.
#[tokio::test]
async fn run() -> Result<()> {
    for entry in fs::read_dir("data/sessions")? {
        let file = File::open(entry?.path())?;
        let session: Replay = serde_yaml::from_reader(&file)?;
        replay(session).await?;
    }

    Ok(())
}

// Compare a set set of inputs and outputs from the previous adapter with the
// current adapter.
async fn replay(replay: Replay) -> Result<()> {
    let provider = MockProvider::new_replay(replay.clone());
    let client = Client::new("at").provider(provider.clone());
    let mut request = R9kMessage::try_from(replay.input)?;

    let Some(change) = request.train_update.changes.get_mut(0) else {
        bail!("no changes in input message");
    };

    // correct event time to 'now' (+ originally recorded delay)
    let now = Utc::now().with_timezone(&Auckland);
    request.train_update.created_date = now.date_naive();
    #[allow(clippy::cast_possible_wrap)]
    let from_midnight = now.num_seconds_from_midnight() as i32;
    let adjusted_secs = replay.delay.map_or(from_midnight, |delay| from_midnight - delay);

    if change.has_departed {
        change.actual_departure_time = adjusted_secs;
    } else if change.has_arrived {
        change.actual_arrival_time = adjusted_secs;
    }

    if let Err(e) = client.request(request).await {
        assert_eq!(e.to_string(), replay.error.unwrap().to_string());
    }

    let curr_events = provider.events();

    let Some(orig_events) = &replay.output else {
        assert!(curr_events.is_empty());
        return Ok(());
    };

    assert_eq!(curr_events.len(), orig_events.len(), "should be 2 publish events per message");

    orig_events.iter().zip(curr_events).for_each(|(published, mut actual)| {
        let original: SmarTrakEvent = serde_json::from_str(published).unwrap();

        // add 5 seconds to the actual message timestamp the adapter sleeps 5 seconds
        // before output the first round
        let diff = now.timestamp() - actual.message_data.timestamp.timestamp();
        assert!(diff.abs() < 3, "expected vs actual too great: {diff}");

        // compare original published message to r9k event
        actual.received_at = original.received_at;
        actual.message_data.timestamp = original.message_data.timestamp;

        let json_actual = serde_json::to_value(&actual).unwrap();
        let json_expected: serde_json::Value = serde_json::from_str(published).unwrap();
        assert_eq!(json_expected, json_actual);
    });

    Ok(())
}
