use anyhow::Result;
use axum::Router;
use axum::extract::Path;
use axum::routing::{get, post};
use bytes::Bytes;
use dilax_adapter::{DetectionReply, DetectionRequest};
use dilax_apc_connector::{DilaxReply, DilaxRequest};
use r9k_connector::{R9kReply, R9kRequest};
use smartrak_gtfs::{
    ResetReply, ResetRequest, SetTripReply, SetTripRequest, VehicleInfoReply, VehicleInfoRequest,
};
use tracing::Level;
use warp_sdk::{Handler, HttpResult, Reply};
use wasip3::exports::http::handler::Guest;
use wasip3::http::types as p3;

use crate::provider::Provider;

pub struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    #[wasi_otel::instrument(name = "http_guest_handle", level = Level::INFO)]
    async fn handle(request: p3::Request) -> Result<p3::Response, p3::ErrorCode> {
        let router = Router::new()
            .route("/inbound/xml", post(r9k_message))
            .route("/api/apc", post(dilax_message))
            .route("/jobs/detector", get(detector))
            .route("/info/{vehicle_id}", get(vehicle_info))
            .route("/god-mode/set-trip/{vehicle_id}/{trip_id}", get(set_trip))
            .route("/god-mode/reset/{vehicle_id}", get(reset));
        wasi_http::serve(router, request).await
    }
}

#[axum::debug_handler]
async fn r9k_message(body: Bytes) -> HttpResult<Reply<R9kReply>> {
    R9kRequest::handler(body.to_vec())?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map_err(Into::into)
}

#[axum::debug_handler]
async fn dilax_message(body: Bytes) -> HttpResult<Reply<DilaxReply>> {
    DilaxRequest::handler(body.to_vec())?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map_err(Into::into)
}

#[axum::debug_handler]
async fn detector() -> HttpResult<Reply<DetectionReply>> {
    DetectionRequest::handler(())?.provider(&Provider::new()).owner("at").await.map_err(Into::into)
}

#[axum::debug_handler]
async fn vehicle_info(Path(vehicle_id): Path<String>) -> HttpResult<Reply<VehicleInfoReply>> {
    VehicleInfoRequest::handler(vehicle_id)?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map_err(Into::into)
}

#[axum::debug_handler]
async fn set_trip(
    Path((vehicle_id, trip_id)): Path<(String, String)>,
) -> HttpResult<Reply<SetTripReply>> {
    SetTripRequest::handler((vehicle_id, trip_id))?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map_err(Into::into)
}

#[axum::debug_handler]
async fn reset(Path(vehicle_id): Path<String>) -> HttpResult<Reply<ResetReply>> {
    ResetRequest::handler(vehicle_id)?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map_err(Into::into)
}
