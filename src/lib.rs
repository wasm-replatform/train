#![cfg(target_arch = "wasm32")]
#![allow(clippy::future_not_send)]

mod provider;

use std::str::FromStr;

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
use smartrak_gtfs::{CafAvlMessage, PassengerCountMessage, SmarTrakMessage, TrainAvlMessage};
use tracing::{Level, debug, error, info, warn};
use wasi_http::Result as HttpResult;
use wasi_messaging::types;
use wasi_messaging::types::Message;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

use crate::provider::Provider;

const SERVICE: &str = "train";

// --------------------------------------------------------
// HTTP Handler
// --------------------------------------------------------
pub struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    #[wasi_otel::instrument(name = "http_guest_handle", level = Level::INFO)]
    async fn handle(request: Request) -> HttpResult<Response, ErrorCode> {
        let router = Router::new()
            .route("/", get(index))
            .route("/jobs/detector", get(detector))
            .route("/inbound/xml", post(r9k_message))
            .route("/info/{vehicle_id}", get(vehicle_info))
            .route("/god-mode/set-trip/{vehicle_id}/{trip_id}", get(god_mode_set_trip))
            .route("/god-mode/reset/{vehicle_id}", get(god_mode_reset))
            .route("/api/apc", post(dilax_message));
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
async fn r9k_message(req: String) -> Result<String, HttpError> {
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
async fn dilax_message(Json(req): Json<DilaxRequest>) -> Result<String, HttpError> {
    info!(monotonic_counter.message_counter = 1, service = SERVICE);

    let client = Client::new(Provider::new());
    let response =
        client.request(req).owner("at").await.context("Error receiving Dilax request")?;
    Ok(response.body.to_string())
}

// --------------------------------------------------------
// Messaging Handler
// --------------------------------------------------------

pub struct Messaging;

wasi_messaging::export!(Messaging with_types_in wasi_messaging);
#[allow(clippy::future_not_send)]
impl wasi_messaging::incoming_handler::Guest for Messaging {
    #[wasi_otel::instrument(name = "messaging_guest_handle")]
    async fn handle(message: Message) -> Result<(), types::Error> {
        let topic = message.topic().unwrap_or_default();
        debug!("received message on topic: {topic}");

        // check we're processing topics for the correct environment
        let env = &Provider::new().config.environment;
        let Some(topic) = topic.strip_prefix(&format!("{env}-")) else {
            warn!(
                monotonic_counter.unhandled_topics = 1,
                topic = %topic,
                service = SERVICE,
                "Incorrect environment {env}"
            );
            return Ok(());
        };

        // process message based on topic
        if let Err(e) = match &topic {
            t if t.contains("realtime-r9k.v1") => process_r9k(&message.data()).await,
            t if t.contains("realtime-dilax-apc.v2") => process_dilax(&message.data()).await,
            // t if t.contains("realtime-r9k-to-smartrak.v1")
            //     || t.contains("realtime-caf-avl.v1")
            //     || t.contains("realtime-smartrak-train-avl.v1") =>
            // {
            //     process_smartrak(&message.data()).await
            // }
            t if t.contains("realtime-r9k-to-smartrak.v1") => {
                process_smartrak(&message.data()).await
            }
            t if t.contains("realtime-caf-avl.v1") => process_caf_avl(&message.data()).await,
            t if t.contains("realtime-train-avl.v1") => process_train_avl(&message.data()).await,
            t if t.contains("realtime-passenger-count.v1") => {
                process_passenger_count(&message.data()).await
            }
            _ => {
                warn!(monotonic_counter.unhandled_topics = 1, topic = %topic, service = SERVICE);
                Ok(())
            }
        } {
            error!(
                monotonic_counter.processing_errors = 1,
                error = %e,
                topic = %topic,
                service = SERVICE
            );
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

#[wasi_otel::instrument]
async fn process_passenger_count(payload: &[u8]) -> Result<()> {
    let api_client = Client::new(Provider::new());
    let request: PassengerCountMessage =
        serde_json::from_slice(payload).context("deserializing event")?;
    api_client.request(request).owner("at").await?;
    Ok(())
}

#[wasi_otel::instrument]
async fn process_smartrak(payload: &[u8]) -> Result<()> {
    let api_client = Client::new(Provider::new());
    let request: SmarTrakMessage =
        serde_json::from_slice(payload).context("deserializing event")?;
    api_client.request(request).owner("at").await?;
    Ok(())
}

#[wasi_otel::instrument]
async fn process_caf_avl(payload: &[u8]) -> Result<()> {
    let api_client = Client::new(Provider::new());
    let request: CafAvlMessage = serde_json::from_slice(payload).context("deserializing event")?;
    api_client.request(request).owner("at").await?;
    Ok(())
}

#[wasi_otel::instrument]
async fn process_train_avl(payload: &[u8]) -> Result<()> {
    let api_client = Client::new(Provider::new());
    let request: TrainAvlMessage =
        serde_json::from_slice(payload).context("deserializing event")?;
    api_client.request(request).owner("at").await?;
    Ok(())
}
