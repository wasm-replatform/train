use anyhow::{Context, Result};
use axum::extract::Path;
use axum::routing::{get, post};
use axum::{Json, Router};
use bytes::Bytes;
use dilax_adapter::DetectionRequest;
use dilax_apc_connector::DilaxRequest;
use fabric::api::Client;
use r9k_connector::R9kRequest;
use smartrak_gtfs::{
    ResetReply, ResetRequest, SetTripReply, SetTripRequest, VehicleInfoReply, VehicleInfoRequest,
};
use tracing::Level;
use wasi_http::HttpError;
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
async fn r9k_message(body: Bytes) -> Result<Bytes, HttpError> {
    let client = Client::new("at").provider(Provider::new());
    let request = R9kRequest::try_from(body.as_ref()).context("parsing request")?;
    let response = client.request(request).await.context("processing request")?;
    Ok(response.body.try_into()?)
}

#[axum::debug_handler]
async fn dilax_message(body: Bytes) -> Result<Bytes, HttpError> {
    let client = Client::new("at").provider(Provider::new());
    let request = DilaxRequest::try_from(body.as_ref()).context("parsing request")?;
    let response = client.request(request).await.context("processing request")?;
    Ok(response.body.try_into()?)
}

#[axum::debug_handler]
async fn detector() -> Result<Bytes, HttpError> {
    let client = Client::new("at").provider(Provider::new());
    let response = client.request(DetectionRequest).await.context("processing request")?;
    Ok(response.body.try_into()?)
}

#[axum::debug_handler]
async fn vehicle_info(Path(vehicle_id): Path<String>) -> Result<Json<VehicleInfoReply>, HttpError> {
    let client = Client::new("at").provider(Provider::new());
    let request = VehicleInfoRequest::try_from(vehicle_id).context("parsing vehicle id")?;
    let response = client.request(request).await.context("processing request")?;
    Ok(Json(response.body))
}

#[axum::debug_handler]
async fn set_trip(
    Path((vehicle_id, trip_id)): Path<(String, String)>,
) -> Result<Json<SetTripReply>, HttpError> {
    let client = Client::new("at").provider(Provider::new());
    let request = SetTripRequest::try_from((vehicle_id, trip_id)).context("parsing vehicle id")?;
    let response = client.request(request).await.context("processing request")?;
    Ok(Json(response.body))
}

#[axum::debug_handler]
async fn reset(Path(vehicle_id): Path<String>) -> Result<Json<ResetReply>, HttpError> {
    let client = Client::new("at").provider(Provider::new());
    let request = ResetRequest::try_from(vehicle_id).context("parsing vehicle id")?;
    let response = client.request(request).await.context("processing request")?;
    Ok(Json(response.body))
}
