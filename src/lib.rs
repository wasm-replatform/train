#![cfg(target_arch = "wasm32")]
#![allow(clippy::future_not_send)]

mod provider;

use std::env;
use std::str::FromStr;
use std::sync::LazyLock;

use anyhow::{Context, Result};
use axum::extract::Path;
use axum::http::header::USER_AGENT;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use credibil_api::Client;
use dilax_adapter::{DetectionRequest, DilaxMessage};
use r9k_adapter::R9kMessage;
use r9k_connector::R9kRequest;
use serde_json::{Value, json};
use smartrak_gtfs::rest::{self, ApiResponse, GodModeOutcome, VehicleInfoResponse};
use smartrak_gtfs::workflow::{self, SerializedMessage};
use tracing::{Level, debug, error, info, warn};
use wasi_http::Result as HttpResult;
use wasi_messaging::types::{Client as MsgClient, Message};
use wasi_messaging::{producer, types};
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

use crate::provider::Provider;

const SERVICE: &str = "train";
const R9K_TOPIC: &str = "realtime-r9k.v1";
const DILAX_TOPIC: &str = "realtime-dilax-adapter-apc.v1";
const SMARTRAK_TOPIC: &str = "realtime-smartrak-bus-avl.v1,realtime-smartrak-bus-avl.v2,realtime-smartrak-train-avl.v1,realtime-r9k-to-smartrak.v1,realtime-passenger-count.v1,realtime-caf-avl.v1";

static ENV: LazyLock<String> =
    LazyLock::new(|| env::var("ENV").unwrap_or_else(|_| "dev".to_string()));

pub struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    #[wasi_otel::instrument(name = "http_guest_handle", level = Level::INFO)]
    async fn handle(request: Request) -> HttpResult<Response, ErrorCode> {
        let router = Router::new()
            .route("/jobs/detector", get(detector))
            .route("/inbound/xml", post(receive_message))
            .route("/", get(index))
            .route("/info/{vehicle_id}", get(vehicle_info))
            .route("/god-mode/set-trip/{vehicle_id}/{trip_id}", get(god_mode_set_trip))
            .route("/god-mode/reset/{vehicle_id}", get(god_mode_reset));
        wasi_http::serve(router, request).await
    }
}

#[axum::debug_handler]
async fn index(headers: HeaderMap) -> HttpResult<&'static str> {
    let user_agent = headers.get(USER_AGENT).and_then(|v| v.to_str().ok());
    rest::log_root(user_agent);
    Ok("OK")
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
    info!(monotonic_counter.message_counter = 1, service = SERVICE);

    let api_client = Client::new(Provider);
    let request = R9kRequest::from_str(&req).context("parsing envelope")?;
    let result = api_client.request(request).owner("owner").await;

    let response = match result {
        Ok(ok) => ok,
        Err(err) => {
            error!(
                monotonic_counter.processing_errors = 1,
                error = %err,
                service = SERVICE
            );
            return Ok(err.description());
        }
    };

    Ok(response.body.to_string())
}

#[axum::debug_handler]
async fn vehicle_info(Path(vehicle_id): Path<String>) -> HttpResult<Json<VehicleInfoResponse>> {
    let provider = Provider;
    let response = rest::vehicle_info(&provider, &vehicle_id).await;
    Ok(Json(response))
}

#[axum::debug_handler]
async fn god_mode_set_trip(
    Path((vehicle_id, trip_id)): Path<(String, String)>,
) -> HttpResult<(StatusCode, Json<ApiResponse>)> {
    match rest::god_mode_set_trip(&vehicle_id, &trip_id) {
        GodModeOutcome::Enabled(response) => Ok((StatusCode::OK, Json(response))),
        GodModeOutcome::Disabled(response) => Ok((StatusCode::NOT_FOUND, Json(response))),
    }
}

#[axum::debug_handler]
async fn god_mode_reset(
    Path(vehicle_id): Path<String>,
) -> HttpResult<(StatusCode, Json<ApiResponse>)> {
    match rest::god_mode_reset(&vehicle_id) {
        GodModeOutcome::Enabled(response) => Ok((StatusCode::OK, Json(response))),
        GodModeOutcome::Disabled(response) => Ok((StatusCode::NOT_FOUND, Json(response))),
    }
}

pub struct Messaging;

wasi_messaging::export!(Messaging with_types_in wasi_messaging);
#[allow(clippy::future_not_send)]
impl wasi_messaging::incoming_handler::Guest for Messaging {
    #[wasi_otel::instrument(name = "messaging_guest_handle")]
    async fn handle(message: Message) -> Result<(), types::Error> {
        let topic = message.topic().unwrap_or_default();
        debug!("received message on topic: {topic}");
        let smartrak_topics_list = SMARTRAK_TOPIC.split(',').collect::<Vec<&str>>();

        if topic == format!("{}-{R9K_TOPIC}", ENV.as_str()) {
            if let Err(e) = process_r9k(&message.data()).await {
                error!(
                    monotonic_counter.processing_errors = 1,
                    error = %e,
                    topic = %topic,
                    service = SERVICE
                );
            }
        } else if topic == format!("{}-{DILAX_TOPIC}", ENV.as_str()) {
            if let Err(e) = process_dilax(&message.data()).await {
                error!(
                    monotonic_counter.processing_errors = 1,
                    error = %e,
                    topic = %topic,
                    service = SERVICE
                );
            }
        } else if smartrak_topics_list.iter().any(|t| topic == format!("{}-{t}", ENV.as_str())) {
            let payload = message.data();
            let provider = Provider;
            match workflow::process(&provider, &topic, &payload).await {
                Ok(workflow::WorkflowOutcome::NoOp) => {
                    debug!(topic = %topic, "no operation from smartrak workflow");
                }
                Ok(workflow::WorkflowOutcome::Messages(messages)) => {
                    debug!(
                        "Publishing Smartrak messages for topic: {}",
                        message.topic().unwrap_or_default()
                    );
                    if let Err(err) = publish_smartrak_messages(messages).await {
                        error!(
                            monotonic_counter.processing_errors = 1,
                            error = %err,
                            topic = %topic,
                            service = SERVICE,
                            "failed to publish smartrak output"
                        );
                    }
                }
                Err(err) => {
                    error!(
                        monotonic_counter.processing_errors = 1,
                        error = %err,
                        topic = %topic,
                        service = SERVICE,
                        "processing smartrak kafka message failed"
                    );
                }
            }
        } else {
            warn!(monotonic_counter.unhandled_topics = 1, topic = %topic, service = SERVICE);
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

#[allow(clippy::unused_async)]
async fn publish_smartrak_messages(messages: Vec<SerializedMessage>) -> Result<()> {
    for message in messages {
        let client = MsgClient::connect("").context("connecting to message broker")?;
        let outgoing = Message::new(&message.payload);
        outgoing.add_metadata("key", &message.key);

        let topic = format!("{}-{}", *ENV, message.topic);
        wit_bindgen::spawn(async move {
            if let Err(err) = producer::send(&client, topic, outgoing).await {
                error!(
                    monotonic_counter.processing_errors = 1,
                    error = %err,
                    service = SERVICE,
                    "failed to publish smartrak output"
                );
            }
        });
    }

    Ok(())
}
