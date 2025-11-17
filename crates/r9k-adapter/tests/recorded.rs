//! Session tests that compare recorded input and output of the Typescript adapter.
#![cfg(not(miri))]

mod provider;

use std::any::Any;
use std::error::Error;
use std::fs::{self, File};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow, bail};
use bytes::Bytes;
use chrono::{Timelike, Utc};
use chrono_tz::Pacific::Auckland;
use credibil_api::Client;
use http::{Request, Response};
use r9k_adapter::{HttpRequest, Identity, Publisher, R9kMessage, SmarTrakEvent, StopInfo};
use serde::Deserialize;

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
async fn replay(session: Session) -> Result<()> {
    let provider = MockProvider::new(session.clone());
    let client = Client::new(provider.clone());
    let mut request = R9kMessage::try_from(session.input)?;

    let Some(change) = request.train_update.changes.get_mut(0) else {
        bail!("no changes in input message");
    };

    // correct event time to 'now' (+ originally recorded delay)
    let now = Utc::now().with_timezone(&Auckland);
    request.train_update.created_date = now.date_naive();
    #[allow(clippy::cast_possible_wrap)]
    let from_midnight = now.num_seconds_from_midnight() as i32;
    let adjusted_secs = session.delay.map_or(from_midnight, |delay| from_midnight - delay);

    if change.has_departed {
        change.actual_departure_time = adjusted_secs;
    } else if change.has_arrived {
        change.actual_arrival_time = adjusted_secs;
    }

    if let Err(e) = client.request(request).owner("owner").await {
        assert_eq!(e, session.error.unwrap());
        return Ok(());
    }

    let curr_events = provider.events();

    let Some(orig_events) = &session.output else {
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

/// One session session of the Typescript adapter.
#[derive(Deserialize, Clone)]
struct Session {
    input: String,
    output: Option<Vec<String>>,
    error: Option<r9k_adapter::Error>,
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
    #[allow(unused)]
    #[must_use]
    fn new(session: Session) -> Self {
        // SAFETY: This is safe in a test context as tests are run sequentially.
        unsafe {
            std::env::set_var("BLOCK_MGT_URL", "http://localhost:8080");
            std::env::set_var("CC_STATIC_URL", "http://localhost:8080");
        };

        Self { session, events: Arc::new(Mutex::new(Vec::new())) }
    }

    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn events(&self) -> Vec<SmarTrakEvent> {
        self.events.lock().expect("should lock").clone()
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
                let stops =
                    self.session.stop_info.as_ref().map(|s| vec![s.clone()]).unwrap_or_default();
                serde_json::to_vec(&stops).context("failed to serialize stops")?
            }
            "/allocations/trips" => {
                let vehicles =
                    self.session.vehicles.clone().unwrap_or_default();
                serde_json::to_vec(&vehicles)
                    .context("failed to serialize vehicles")?
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
