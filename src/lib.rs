#![cfg(target_arch = "wasm32")]
#![allow(clippy::future_not_send)]

mod provider;

use std::sync::LazyLock;
use std::time::Duration;
use std::{env, thread};

use anyhow::{Context, Result, anyhow};
use axum::routing::get;
use axum::{Json, Router};
use credibil_api::Client;
use dilax::{DetectionRequest, DilaxEnrichedEvent, DilaxMessage};
use r9k_position::R9kMessage;
use serde_json::Value;
use smartrak_gtfs::workflow;
use tracing::{Level, error, info};
use wasi_http::Result as HttpResult;
use wasi_messaging::types::{Client as MsgClient, Message};
use wasi_messaging::{producer, types};
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

use crate::provider::Provider;

const SERVICE: &str = "r9k-position-adapter";
const SMARTRAK_TOPIC: &str = "realtime-r9k-to-smartrak.v1";
const R9K_TOPIC: &str = "realtime-r9k.v1";
const DILAX_TOPIC: &str = "realtime-dilax-apc.v1";
const DILAX_ENRICHED_TOPIC: &str = "realtime-dilax-apc-enriched.v1";

static ENV: LazyLock<String> =
    LazyLock::new(|| env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string()));

pub struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    #[wasi_otel::instrument(name = "http_guest_handle", level = Level::INFO)]
    async fn handle(request: Request) -> HttpResult<Response, ErrorCode> {
        let router = Router::new().route("/jobs/detector", get(jobs_detector));
        wasi_http::serve(router, request).await
    }
}

#[axum::debug_handler]
async fn jobs_detector() -> HttpResult<Json<Value>> {
    let api = Client::new(provider::Provider);
    let builder = api.request(DetectionRequest).owner("owner");

    let response = builder
        .await
        .map_err(|err| anyhow!("failed to run Dilax lost connection detector event: {err}"))?;

    Ok(Json(serde_json::json!({
        "status": "job detection triggered",
        "detections": response.detections.len()
    })))
}

pub struct Messaging;

wasi_messaging::export!(Messaging with_types_in wasi_messaging);

#[allow(clippy::future_not_send)]
impl wasi_messaging::incoming_handler::Guest for Messaging {
    #[wasi_otel::instrument(name = "messaging_guest_handle")]
    async fn handle(message: Message) -> Result<(), types::Error> {
        let topic = message.topic().unwrap_or_default();

        if topic == format!("{}-{R9K_TOPIC}", *ENV) {
            if let Err(e) = r9k_message(&message.data()).await {
                error!(
                    monotonic_counter.processing_errors = 1,
                    error = %e,
                    topic = %topic,
                    service = %SERVICE
                );
            }
        } else if topic == format!("{}-{DILAX_TOPIC}", *ENV) {
            if let Err(e) = process_dilax(&message.data()).await {
                error!(
                    monotonic_counter.processing_errors = 1,
                    error = %e,
                    topic = %topic,
                    service = %SERVICE
                );
            }
        } else {
            let payload = message.data();
            match workflow::process(&topic, &payload).await {
                Ok(workflow::WorkflowOutcome::NoOp) => {}
                Ok(workflow::WorkflowOutcome::Messages(messages)) => {
                    if let Err(err) = publish_smartrak_messages(messages).await {
                        error!(
                            monotonic_counter.processing_errors = 1,
                            error = %err,
                            topic = %topic,
                            service = %SERVICE,
                            "failed to publish smartrak output"
                        );
                    }
                }
                Err(err) => {
                    error!(
                        monotonic_counter.processing_errors = 1,
                        error = %err,
                        topic = %topic,
                        service = %SERVICE,
                        "processing smartrak kafka message failed"
                    );
                }
            }
        }

        Ok(())
    }
}

// Process incoming R9k messages, consolidating error handling.
#[wasi_otel::instrument]
async fn r9k_message(message: &[u8]) -> Result<()> {
    let api_client = Client::new(Provider);
    let request = R9kMessage::try_from(message).context("parsing message")?;
    let response = api_client.request(request).owner("owner").await?;

    let Some(events) = response.body.smartrak_events.as_ref() else { return Ok(()) };

    // publish events 2x in order to properly signal departure from the station
    // (for schedule adherence)
    let dest_topic = format!("{}-{SMARTRAK_TOPIC}", *ENV);

    for _ in 0..2 {
        thread::sleep(Duration::from_secs(5));

        for evt in events {
            let external_id = &evt.remote_data.external_id;
            let msg = serde_json::to_vec(&evt).context("serializing event")?;
            let message = Message::new(&msg);
            message.add_metadata("key", external_id);

            let client = MsgClient::connect("").context("connecting to message broker")?;
            let topic = dest_topic.clone();

            wit_bindgen::spawn(async move {
                if let Err(e) = producer::send(&client, topic, message).await {
                    error!(
                        monotonic_counter.processing_errors = 1, error = %e, service = %SERVICE
                    );
                }
            });

            info!(
                monotonic_counter.messages_sent = 1, external_id = %external_id, service = %SERVICE
            );
        }
    }

    Ok(())
}

#[wasi_otel::instrument]
async fn process_dilax(payload: &[u8]) -> Result<()> {
    let event: DilaxMessage =
        serde_json::from_slice(payload).context("deserializing Dilax event")?;

    let api = Client::new(provider::Provider);
    let response = api.request(event).owner("owner").await?;

    let enriched = response.body;
    publish_dilax(&enriched).await
}

#[wasi_otel::instrument]
async fn publish_dilax(event: &DilaxEnrichedEvent) -> Result<()> {
    let client = MsgClient::connect("<not used>").context("connecting to message broker")?;
    let payload = serde_json::to_vec(event).context("serializing Dilax enriched event")?;
    let message = Message::new(&payload);
    if let Some(key) = event.trip_id.as_deref() {
        message.add_metadata("key", key);
    }

    producer::send(&client, format!("{}-{DILAX_ENRICHED_TOPIC}", *ENV), message)
        .await
        .map_err(|err| anyhow!("failed to publish Dilax event: {err}"))?;

    info!(monotonic_counter.messages_sent = 1, service = %SERVICE, event = "dilax");

    Ok(())
}

#[allow(clippy::unused_async)]
async fn publish_smartrak_messages(messages: Vec<SerializedMessage>) -> Result<()> {
    for message in messages {
        let client = MsgClient::connect("").context("connecting to message broker")?;
        let outgoing = Message::new(&message.payload);
        outgoing.add_metadata("key", &message.key);

            let topic = message.topic.clone();
            wit_bindgen::spawn(async move {
                if let Err(err) = producer::send(&client, topic, outgoing).await {
                error!(monotonic_counter.processing_errors = 1, error = %err, service = %SERVICE, "failed to publish smartrak output");
            }
        });
    }

    Ok(())
}
