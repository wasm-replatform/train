#![cfg(target_arch = "wasm32")]
#![allow(clippy::future_not_send)]

mod provider;

use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::Path;
use axum::http::header::USER_AGENT;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use credibil_api::Client;
use dilax_adapter::{DetectionRequest, DilaxMessage};
use dilax_apc_connector::DilaxRequest;
use r9k_adapter::R9kMessage;
use r9k_connector::R9kRequest;
use realtime::HttpError;
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
const DILAX_TOPIC: &str = "realtime-dilax-apc.v2";
const SMARTRAK_TOPIC: &str = "realtime-smartrak-train-avl.v1,realtime-r9k-to-smartrak.v1,realtime-passenger-count.v1,realtime-caf-avl.v1";

pub struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    #[wasi_otel::instrument(name = "http_guest_handle", level = Level::INFO)]
    async fn handle(request: Request) -> HttpResult<Response, ErrorCode> {
        let router = Router::new()
            .route("/", get(index))
            .route("/jobs/detector", get(detector))
            .route("/inbound/xml", post(receive_message))
            .route("/info/{vehicle_id}", get(vehicle_info))
            .route("/god-mode/set-trip/{vehicle_id}/{trip_id}", get(god_mode_set_trip))
            .route("/god-mode/reset/{vehicle_id}", get(god_mode_reset))
            .route("/api/apc", post(receive_dilax_message));
        wasi_http::serve(router, request).await
    }
}

#[axum::debug_handler]
async fn index(headers: HeaderMap) -> Result<&'static str, HttpError> {
    let user_agent = headers.get(USER_AGENT).and_then(|v| v.to_str().ok());
    rest::log_root(user_agent);
    Ok("OK")
}

#[axum::debug_handler]
async fn detector() -> Result<Json<Value>, HttpError> {
    let api = Client::new(Provider::new());
    let router = api.request(DetectionRequest).owner("at");
    let response = router.await.context("Issue running connection detector")?;

    Ok(Json(json!({
        "status": "job detection triggered",
        "detections": response.detections.len()
    })))
}

#[axum::debug_handler]
async fn receive_message(req: String) -> Result<String, HttpError> {
    info!(monotonic_counter.message_counter = 1, service = SERVICE);

    let api_client = Client::new(Provider::new());
    let request = R9kRequest::from_str(&req).context("parsing envelope")?;
    let result = api_client.request(request).owner("at").await;

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
async fn vehicle_info(
    Path(vehicle_id): Path<String>,
) -> Result<Json<VehicleInfoResponse>, HttpError> {
    let provider = Provider::new();
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

#[axum::debug_handler]
async fn receive_dilax_message(Json(req): Json<DilaxRequest>) -> Result<String, HttpError> {
    info!(monotonic_counter.message_counter = 1, service = SERVICE);

    let client = Client::new(Provider::new());
    let response =
        client.request(req).owner("at").await.context("Error receiving Dilax request")?;
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
        let smartrak_topics_list = SMARTRAK_TOPIC.split(',').collect::<Vec<&str>>();

        let env = Provider::new().config.environment.clone();

        if topic == format!("{env}-{R9K_TOPIC}") {
            if let Err(e) = process_r9k(&message.data()).await {
                error!(
                    monotonic_counter.processing_errors = 1,
                    error = %e,
                    topic = %topic,
                    service = SERVICE
                );
            }
        } else if topic == format!("{env}-{DILAX_TOPIC}") {
            if let Err(e) = process_dilax(&message.data()).await {
                error!(
                    monotonic_counter.processing_errors = 1,
                    error = %e,
                    topic = %topic,
                    service = SERVICE
                );
            } else {
                info!(monotonic_counter.dilax_topic = 1, topic = %topic, service = SERVICE);
            }
        } else if smartrak_topics_list.iter().any(|t| topic == format!("{}-{t}", env.as_str())) {
            let payload = message.data();
            let provider = Provider::new();
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
    let api_client = Client::new(Provider::new());
    let request = R9kMessage::try_from(message).context("parsing message")?;
    api_client.request(request).owner("at").await?;
    Ok(())
}

#[wasi_otel::instrument]
async fn process_dilax(payload: &[u8]) -> Result<()> {
    let api_client = Client::new(Provider::new());
    let request: DilaxMessage = serde_json::from_slice(payload).context("deserializing event")?;
    api_client.request(request).owner("at").await?;
    Ok(())
}

#[allow(clippy::unused_async)]
async fn publish_smartrak_messages(messages: Vec<SerializedMessage>) -> Result<()> {
    let env = Provider::new().config.environment;
    let client = Arc::new(
        MsgClient::connect("kafka".to_string()).await.context("connecting to message broker")?,
    );

    for message in messages {
        let outgoing = Message::new(&message.payload);
        outgoing.add_metadata("key", &message.key);

        let topic = format!("{env}-{}", message.topic);
        let client = Arc::clone(&client);
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
