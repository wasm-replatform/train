#![cfg(target_arch = "wasm32")]
#![allow(clippy::future_not_send)]

mod provider;

use anyhow::{Context, Result};
use axum::extract::Path;
use axum::routing::{get, post};
use axum::{Json, Router};
use bytes::Bytes;
use credibil_api::Client;
use dilax_adapter::{DetectionRequest, DetectionResponse, DilaxMessage};
use dilax_apc_connector::DilaxRequest;
use fabric::HttpError;
use r9k_adapter::R9kMessage;
use r9k_connector::R9kRequest;
use smartrak_gtfs::{
    CafAvlMessage, PassengerCountMessage, ResetRequest, ResetResponse, SetTripRequest,
    SetTripResponse, SmarTrakMessage, TrainAvlMessage, VehicleInfoRequest, VehicleInfoResponse,
};
use tracing::{Level, debug};
use wasi_http::Result as HttpResult;
use wasi_messaging::types;
use wasi_messaging::types::Message;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

use crate::provider::Provider;

// --------------------------------------------------------
// HTTP Handler
// --------------------------------------------------------
pub struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    #[wasi_otel::instrument(name = "http_guest_handle", level = Level::INFO)]
    async fn handle(request: Request) -> HttpResult<Response, ErrorCode> {
        let router = Router::new()
            .route("/jobs/detector", get(detector))
            .route("/inbound/xml", post(r9k_message))
            .route("/api/apc", post(dilax_message))
            .route("/info/{vehicle_id}", get(vehicle_info))
            .route("/god-mode/set-trip/{vehicle_id}/{trip_id}", get(set_trip))
            .route("/god-mode/reset/{vehicle_id}", get(reset));
        wasi_http::serve(router, request).await
    }
}

#[axum::debug_handler]
async fn detector() -> Result<Json<DetectionResponse>, HttpError> {
    let client = Client::new(Provider::new());
    let response =
        client.request(DetectionRequest).owner("at").await.context("processing request")?;
    Ok(Json(response.body))
}

#[axum::debug_handler]
async fn r9k_message(body: Bytes) -> Result<String, HttpError> {
    let api_client = Client::new(Provider::new());
    let request = R9kRequest::try_from(body.as_ref()).context("parsing request")?;
    let result = api_client.request(request).owner("at").await;

    let response = match result {
        Ok(ok) => ok,
        Err(err) => {
            return Ok(err.description());
        }
    };

    Ok(response.body.to_string())
}

#[axum::debug_handler]
async fn dilax_message(body: Bytes) -> Result<String, HttpError> {
    let client = Client::new(Provider::new());
    let request = DilaxRequest::try_from(body.as_ref()).context("parsing request")?;
    let response = client.request(request).owner("at").await.context("processing request")?;
    Ok(response.body.to_string())
}

#[axum::debug_handler]
async fn vehicle_info(
    Path(vehicle_id): Path<String>,
) -> Result<Json<VehicleInfoResponse>, HttpError> {
    let client = Client::new(Provider::new());
    let request = VehicleInfoRequest::try_from(vehicle_id).context("parsing vehicle id")?;
    let response = client.request(request).owner("at").await.context("processing request")?;
    Ok(Json(response.body))
}

#[axum::debug_handler]
async fn set_trip(
    Path((vehicle_id, trip_id)): Path<(String, String)>,
) -> Result<Json<SetTripResponse>, HttpError> {
    let client = Client::new(Provider::new());
    let request = SetTripRequest::try_from((vehicle_id, trip_id)).context("parsing vehicle id")?;
    let response = client.request(request).owner("at").await.context("processing request")?;
    Ok(Json(response.body))
}

#[axum::debug_handler]
async fn reset(Path(vehicle_id): Path<String>) -> Result<Json<ResetResponse>, HttpError> {
    let client = Client::new(Provider::new());
    let request = ResetRequest::try_from(vehicle_id).context("parsing vehicle id")?;
    let response = client.request(request).owner("at").await.context("processing request")?;
    Ok(Json(response.body))
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
            return Err(types::Error::Other("Incorrect environment".to_string()));
        };

        // process message based on topic
        if let Err(e) = match &topic {
            t if t.contains("realtime-r9k.v1") => process_r9k(&message.data()).await,
            t if t.contains("realtime-dilax-apc.v2") => process_dilax(&message.data()).await,
            t if t.contains("realtime-r9k-to-smartrak.v1") => {
                process_smartrak(&message.data()).await
            }
            t if t.contains("realtime-caf-avl.v1") => process_caf_avl(&message.data()).await,
            t if t.contains("realtime-train-avl.v1") => process_train_avl(&message.data()).await,
            t if t.contains("realtime-passenger-count.v1") => {
                process_passenger_count(&message.data()).await
            }
            _ => {
                return Err(types::Error::Other("Unhandled topic".to_string()));
            }
        } {
            return Err(types::Error::Other(e.to_string()));
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
    let request = DilaxMessage::try_from(payload).context("deserializing event")?;
    api_client.request(request).owner("at").await?;
    Ok(())
}

#[wasi_otel::instrument]
async fn process_passenger_count(payload: &[u8]) -> Result<()> {
    let api_client = Client::new(Provider::new());
    let request = PassengerCountMessage::try_from(payload).context("deserializing event")?;
    api_client.request(request).owner("at").await?;
    Ok(())
}

#[wasi_otel::instrument]
async fn process_smartrak(payload: &[u8]) -> Result<()> {
    let api_client = Client::new(Provider::new());
    let request = SmarTrakMessage::try_from(payload).context("deserializing event")?;
    api_client.request(request).owner("at").await?;
    Ok(())
}

#[wasi_otel::instrument]
async fn process_caf_avl(payload: &[u8]) -> Result<()> {
    let api_client = Client::new(Provider::new());
    let request = CafAvlMessage::try_from(payload).context("deserializing event")?;
    api_client.request(request).owner("at").await?;
    Ok(())
}

#[wasi_otel::instrument]
async fn process_train_avl(payload: &[u8]) -> Result<()> {
    let api_client = Client::new(Provider::new());
    let request = TrainAvlMessage::try_from(payload).context("deserializing event")?;
    api_client.request(request).owner("at").await?;
    Ok(())
}
