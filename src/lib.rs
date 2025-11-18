#![cfg(target_arch = "wasm32")]
#![allow(clippy::future_not_send)]

mod provider;

use std::env;
use std::str::FromStr;
use std::sync::LazyLock;

use anyhow::{Context, Result};
use axum::routing::{get, post};
use axum::{Json, Router};
use credibil_api::Client;
use dilax_adapter::{DetectionRequest, DilaxMessage};
use r9k_adapter::R9kMessage;
use r9k_connector::R9kRequest;
use serde_json::{Value, json};
use tracing::{Level, debug, error, info, warn};
use wasi_http::Result as HttpResult;
use wasi_messaging::types;
use wasi_messaging::types::Message;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

use crate::provider::Provider;


const R9K_TOPIC: &str = "realtime-r9k.v1";
const DILAX_TOPIC: &str = "realtime-dilax-adapter-apc.v1";

static ENV: LazyLock<String> =
    LazyLock::new(|| env::var("ENV").unwrap_or_else(|_| "dev".to_string()));

pub struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    #[wasi_otel::instrument(name = "http_guest_handle", level = Level::INFO)]
    async fn handle(request: Request) -> HttpResult<Response, ErrorCode> {
        let router = Router::new()
            .route("/jobs/detector", get(detector))
            .route("/inbound/xml", post(receive_message));
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
async fn receive_message(req: String) -> HttpResult<String> {
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

#[wasi_otel::instrument]
async fn process_r9k(message: &[u8]) -> Result<()> {
    let api_client = Client::new(Provider);
    let request = R9kMessage::try_from(message).context("parsing message")?;
    api_client.request(request).owner("owner").await?;
    Ok(())
}

#[wasi_otel::instrument]
async fn process_dilax(payload: &[u8]) -> Result<()> {
    let api_client = Client::new(Provider);
    let request: DilaxMessage = serde_json::from_slice(payload).context("deserializing event")?;
    api_client.request(request).owner("owner").await?;
    Ok(())
}
