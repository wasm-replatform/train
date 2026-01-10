#![cfg(target_arch = "wasm32")]

use anyhow::Result;
use axum::Router;
use axum::extract::Path;
use axum::routing::{get, post};
use bytes::Bytes;
use dilax_adapter::{DetectionReply, DetectionRequest, DilaxMessage};
use dilax_apc_connector::{DilaxReply, DilaxRequest};
use r9k_adapter::R9kMessage;
use r9k_connector::{R9kReply, R9kRequest};
use smartrak_gtfs::{
    CafAvlMessage, PassengerCountMessage, ResetReply, ResetRequest, SetTripReply, SetTripRequest,
    SmarTrakMessage, TrainAvlMessage, VehicleInfoReply, VehicleInfoRequest,
};
use tracing::Level;
use warp_sdk::{
    Config, Handler, HttpRequest, HttpResult, Identity, Publisher, Reply, StateStore, ensure_env,
};
use wasi_messaging::types::{Error, Message};
use wasip3::exports::http::handler::Guest;
use wasip3::http::types as p3;

pub struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    #[wasi_otel::instrument(name = "http_guest_handle", level = Level::INFO)]
    async fn handle(request: p3::Request) -> Result<p3::Response, p3::ErrorCode> {
        let router = Router::new()
            .route("/api/apc", post(dilax_message))
            .route("/inbound/xml", post(r9k_message))
            .route("/jobs/detector", get(detector))
            .route("/info/{vehicle_id}", get(vehicle_info))
            .route("/god-mode/set-trip/{vehicle_id}/{trip_id}", get(set_trip))
            .route("/god-mode/reset/{vehicle_id}", get(reset));
        wasi_http::serve(router, request).await
    }
}

async fn dilax_message(body: Bytes) -> HttpResult<Reply<DilaxReply>> {
    DilaxRequest::handler(body.to_vec())?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map_err(Into::into)
}

async fn r9k_message(body: Bytes) -> HttpResult<Reply<R9kReply>> {
    R9kRequest::handler(body.to_vec())?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map_err(Into::into)
}

async fn detector() -> HttpResult<Reply<DetectionReply>> {
    DetectionRequest::handler(())?.provider(&Provider::new()).owner("at").await.map_err(Into::into)
}

async fn vehicle_info(Path(vehicle_id): Path<String>) -> HttpResult<Reply<VehicleInfoReply>> {
    VehicleInfoRequest::handler(vehicle_id)?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map_err(Into::into)
}

async fn set_trip(
    Path((vehicle_id, trip_id)): Path<(String, String)>,
) -> HttpResult<Reply<SetTripReply>> {
    SetTripRequest::handler((vehicle_id, trip_id))?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map_err(Into::into)
}

async fn reset(Path(vehicle_id): Path<String>) -> HttpResult<Reply<ResetReply>> {
    ResetRequest::handler(vehicle_id)?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map_err(Into::into)
}

pub struct Messaging;
wasi_messaging::export!(Messaging with_types_in wasi_messaging);

impl wasi_messaging::incoming_handler::Guest for Messaging {
    #[wasi_otel::instrument(name = "messaging_guest_handle")]
    async fn handle(message: Message) -> Result<(), Error> {
        if let Err(e) = match &message.topic().unwrap_or_default() {
            t if t.contains("realtime-r9k.v1") => r9k(message.data()).await,
            t if t.contains("realtime-r9k-to-smartrak.v1") => smartrak(message.data()).await,
            t if t.contains("realtime-dilax-apc.v2") => dilax(message.data()).await,
            t if t.contains("realtime-caf-avl.v1") => caf_avl(message.data()).await,
            t if t.contains("realtime-train-avl.v1") => train_avl(message.data()).await,
            t if t.contains("realtime-passenger-count.v1") => passenger_count(message.data()).await,
            _ => {
                return Err(Error::Other("Unhandled topic".to_string()));
            }
        } {
            return Err(Error::Other(e.to_string()));
        }
        Ok(())
    }
}

#[wasi_otel::instrument]
async fn r9k(payload: Vec<u8>) -> Result<()> {
    R9kMessage::handler(payload)?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map(|_| ())
        .map_err(Into::into)
}

#[wasi_otel::instrument]
async fn smartrak(payload: Vec<u8>) -> Result<()> {
    SmarTrakMessage::handler(payload)?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map(|_| ())
        .map_err(Into::into)
}

#[wasi_otel::instrument]
async fn dilax(payload: Vec<u8>) -> Result<()> {
    DilaxMessage::handler(payload)?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map(|_| ())
        .map_err(Into::into)
}

#[wasi_otel::instrument]
async fn caf_avl(payload: Vec<u8>) -> Result<()> {
    CafAvlMessage::handler(payload)?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map(|_| ())
        .map_err(Into::into)
}

#[wasi_otel::instrument]
async fn train_avl(payload: Vec<u8>) -> Result<()> {
    TrainAvlMessage::handler(payload)?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map(|_| ())
        .map_err(Into::into)
}

#[wasi_otel::instrument]
async fn passenger_count(payload: Vec<u8>) -> Result<()> {
    PassengerCountMessage::handler(payload)?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map(|_| ())
        .map_err(Into::into)
}

#[derive(Clone, Default)]
pub struct Provider;

impl Provider {
    #[must_use]
    pub fn new() -> Self {
        ensure_env!(
            "BLOCK_MGT_URL",
            "CC_STATIC_URL",
            "FLEET_URL",
            "GTFS_STATIC_URL",
            "TRIP_MANAGEMENT_URL",
            "AZURE_IDENTITY"
        );
        Self
    }
}

impl Config for Provider {}
impl HttpRequest for Provider {}
impl Identity for Provider {}
impl Publisher for Provider {}
impl StateStore for Provider {}
