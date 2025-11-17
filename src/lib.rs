#![cfg(target_arch = "wasm32")]
#![allow(clippy::future_not_send)]

mod provider;

use std::str::FromStr;
use std::sync::LazyLock;
use std::{env};

use anyhow::{Context, Result, anyhow};
use axum::routing::{get, post};
use axum::{Json, Router};
use credibil_api::Client;
use dilax::{DetectionRequest, DilaxMessage};
use r9k_adapter::R9kMessage;
use r9k_connector::R9kRequest;
use serde_json::{Value, json};
use tracing::debug;
use tracing::{Level, error, info, warn};
use wasi_http::Result as HttpResult;
use wasi_messaging::types::{Client as MsgClient, Message};
use wasi_messaging::{producer, types};
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

use crate::provider::Provider;

const SERVICE: &str = "train";
const R9K_TOPIC: &str = "realtime-r9k.v1";
// const SMARTRAK_TOPIC: &str = "realtime-r9k-to-smartrak.v1";
const DILAX_TOPIC: &str = "realtime-dilax-apc.v1";
const DILAX_ENRICHED_TOPIC: &str = "realtime-dilax-apc-enriched.v1";

static ENV: LazyLock<String> =
    LazyLock::new(|| env::var("ENV").unwrap_or_else(|_| "dev".to_string()));

pub struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    #[wasi_otel::instrument(name = "http_guest_handle", level = Level::INFO)]
    async fn handle(request: Request) -> HttpResult<Response, ErrorCode> {
        let router = Router::new()
            .route("/jobs/detector", get(detector))
            .route("/inbound/xml", post(r9k_message));
        wasi_http::serve(router, request).await
    }
}

#[axum::debug_handler]
async fn detector() -> HttpResult<Json<Value>> {
    let api = Client::new(provider::Provider);
    let router = api.request(DetectionRequest).owner("owner");

    let response = router.await.context("Issue running connection detector")?;

    Ok(Json(json!({
        "status": "job detection triggered",
        "detections": response.detections.len()
    })))
}

#[axum::debug_handler]
async fn r9k_message(req: String) -> HttpResult<String> {
    info!(monotonic_counter.message_counter = 1, service = "train");

    let api_client = Client::new(Provider);
    let request = R9kRequest::from_str(&req).context("parsing envelope")?;
    let result = api_client.request(request).owner("owner").await;

    let response = match result {
        Ok(ok) => ok,
        Err(err) => {
            error!(
                monotonic_counter.processing_errors = 1,
                error = %err,
                service = "train"
            );
            return Ok(err.description());
        }
    };

    Ok(response.body.to_string())
}

pub struct Messaging;

wasi_messaging::export!(Messaging with_types_in wasi_messaging);

#[allow(clippy::future_not_send)]
impl wasi_messaging::incoming_handler::Guest for Messaging {
    #[wasi_otel::instrument(name = "messaging_guest_handle")]
    async fn handle(message: Message) -> Result<(), types::Error> {
        let topic = message.topic().unwrap_or_default();
        debug!("received message on topic: {topic}");

        if topic == format!("{}-{R9K_TOPIC}", ENV.as_str()) {
            if let Err(e) = process_r9k(&message.data()).await {
                error!(
                    monotonic_counter.processing_errors = 1,
                    error = %e,
                    topic = %topic,
                    service = "train"
                );
            }
        } else if topic == format!("{}-{DILAX_TOPIC}", ENV.as_str()) {
            if let Err(e) = process_dilax(&message.data()).await {
                error!(
                    monotonic_counter.processing_errors = 1,
                    error = %e,
                    topic = %topic,
                    service = "train"
                );
            }
        } else {
            warn!(monotonic_counter.unhandled_topics = 1, topic = %topic, service = "train");
        }

        Ok(())
    }
}

// Process incoming R9k messages, consolidating error handling.
#[wasi_otel::instrument]
async fn process_r9k(message: &[u8]) -> Result<()> {
    let api_client = Client::new(Provider);
    let request = R9kMessage::try_from(message).context("parsing message")?;
    let _ = api_client.request(request).owner("owner").await?;

    // let Some(events) = response.body.smartrak_events.as_ref() else { return Ok(()) };

    // // publish events 2x in order to properly signal departure from the station
    // // (for schedule adherence)
    // let dest_topic = format!("{}-{SMARTRAK_TOPIC}", ENV.as_str());

    // for _ in 0..2 {
    //     thread::sleep(Duration::from_secs(5));

    //     for evt in events {
    //         let external_id = &evt.remote_data.external_id;
    //         let msg = serde_json::to_vec(&evt).context("serializing event")?;
    //         let message = Message::new(&msg);
    //         message.add_metadata("key", external_id);

    //         let client = MsgClient::connect("").context("connecting to broker")?;
    //         let topic = dest_topic.clone();

    //         wit_bindgen::spawn(async move {
    //             if let Err(e) = producer::send(&client, topic, message).await {
    //                 error!(
    //                     monotonic_counter.processing_errors = 1, error = %e, service = %SERVICE
    //                 );
    //             }
    //         });

    //         info!(
    //             monotonic_counter.messages_sent = 1, external_id = %external_id, service = %SERVICE
    //         );
    //     }
    // }

    Ok(())
}

#[wasi_otel::instrument]
async fn process_dilax(payload: &[u8]) -> Result<()> {
    let event: DilaxMessage = serde_json::from_slice(payload).context("deserializing event")?;

    let api = Client::new(provider::Provider);
    let response = api.request(event).owner("owner").await?;
    let enriched = response.body;

    let client = MsgClient::connect("<not used>").context("connecting to broker")?;
    let payload = serde_json::to_vec(&enriched).context("serializing event")?;
    let message = Message::new(&payload);

    if let Some(key) = &enriched.trip_id {
        message.add_metadata("key", key);
    }

    producer::send(&client, format!("{}-{DILAX_ENRICHED_TOPIC}", ENV.as_str()), message)
        .await
        .map_err(|err| anyhow!("failed to publish event: {err}"))?;

    info!(monotonic_counter.messages_sent = 1, service = %SERVICE, event = "dilax");

    Ok(())
}
