//! Dilax adapter request handlers.

use anyhow::Context;
use credibil_api::{Handler, Request, Response};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::config::Config;
use crate::error::{Error, Result};
use crate::logic::{detect_lost_connections, fetch_allocations_for_today, process_event};
use crate::provider::{Provider, ProviderWrapper};
use crate::types::{DilaxEvent, DilaxEventEnriched, LostConnectionCandidate, UnixTimestamp};

use realtime::Message;

const APC_ENRICHED_TOPIC: &str = "realtime-dilax-adapter-apc-enriched.v1";

/// Incoming Dilax passenger counting message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DilaxMessage {
    #[serde(flatten)]
    pub event: DilaxEvent,
}

impl DilaxMessage {
    #[must_use]
    pub fn into_event(self) -> DilaxEvent {
        self.event
    }
}

/// Empty response for message processing.
#[derive(Debug, Clone, Default)]
pub struct DilaxResponse;

/// Detection job request payload.
#[derive(Debug, Clone, Copy, Default)]
pub struct DetectionRequest;

/// Detection job response containing lost connection candidates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResponse {
    pub detections: Vec<LostConnectionCandidate>,
}

/// # Errors
/// Returns an error when configuration loading, event processing, or publishing fails.
async fn handle_message<P>(
    _owner: &str, request: DilaxMessage, provider: &P,
) -> Result<Response<DilaxResponse>>
where
    P: Provider,
{
    let config = Config::from_env().context("loading adapter config")?;
    let wrapper = ProviderWrapper::new(provider, &config);

    let event = request.into_event();
    let maybe_enriched = process_event(&wrapper, event).await?;

    if let Some(enriched) = maybe_enriched {
        publish_enriched_event(provider, &enriched).await?;
    }

    Ok(DilaxResponse.into())
}

/// # Errors
/// Returns an error when the enriched payload cannot be serialized or published.
async fn publish_enriched_event<P>(provider: &P, enriched: &DilaxEventEnriched) -> Result<()>
where
    P: Provider,
{
    let payload = serde_json::to_vec(enriched).context("serializing enriched Dilax event")?;
    let mut message = Message::new(&payload);

    if let Some(trip_id) = enriched.trip_id.as_ref() {
        let key: String = trip_id.clone().into();
        message.headers.insert("key".to_string(), key);
    }

    provider.send(APC_ENRICHED_TOPIC, &message).await.map_err(Error::from)
}

/// # Errors
/// Returns an error when configuration loading or detection logic fails.
async fn handle_detection<P>(
    _owner: &str, _request: DetectionRequest, provider: &P,
) -> Result<Response<DetectionResponse>>
where
    P: Provider,
{
    let config = Config::from_env().context("loading adapter config")?;
    let wrapper = ProviderWrapper::new(provider, &config);

    let allocations = fetch_allocations_for_today(&wrapper).await?;
    if allocations.is_empty() {
        warn!("no allocations available for current service date");
    }

    let detection_time = UnixTimestamp::now();
    let detections =
        detect_lost_connections(&wrapper, &config, &allocations, detection_time).await?;

    info!("lost connection candidates found = {}", detections.len());

    Ok(DetectionResponse { detections }.into())
}

impl<P> Handler<DilaxResponse, P> for Request<DilaxMessage>
where
    P: Provider,
{
    type Error = Error;

    async fn handle(self, owner: &str, provider: &P) -> Result<Response<DilaxResponse>> {
        handle_message(owner, self.body, provider).await
    }
}

impl<P> Handler<DetectionResponse, P> for Request<DetectionRequest>
where
    P: Provider,
{
    type Error = Error;

    async fn handle(self, owner: &str, provider: &P) -> Result<Response<DetectionResponse>> {
        handle_detection(owner, self.body, provider).await
    }
}
