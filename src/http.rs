use anyhow::{Context, Result};
use axum::Router;
use axum::extract::Path;
use axum::routing::{get, post};
use bytes::Bytes;
use dilax_adapter::{DetectionReply, DetectionRequest};
use dilax_apc_connector::{DilaxReply, DilaxRequest};
use fabric::api::{Client, HttpResult, Reply};
use r9k_connector::{R9kReply, R9kRequest};
use smartrak_gtfs::{
    ResetReply, ResetRequest, SetTripReply, SetTripRequest, VehicleInfoReply, VehicleInfoRequest,
};
use tracing::Level;
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
    let client = Client::new("at").provider(Provider::new());
    let request = R9kRequest::try_from(body.as_ref()).context("parsing request")?;
    let reply = client.request(request).await.context("processing request")?;
    Ok(reply)
}

#[axum::debug_handler]
async fn dilax_message(body: Bytes) -> HttpResult<Reply<DilaxReply>> {
    let client = Client::new("at").provider(Provider::new());
    let request = DilaxRequest::try_from(body.as_ref()).context("parsing request")?;
    let reply = client.request(request).await.context("processing request")?;
    Ok(reply)
}

#[axum::debug_handler]
async fn detector() -> HttpResult<Reply<DetectionReply>> {
    let client = Client::new("at").provider(Provider::new());
    let reply = client.request(DetectionRequest).await.context("processing request")?;
    Ok(reply)
}

#[axum::debug_handler]
async fn vehicle_info(Path(vehicle_id): Path<String>) -> HttpResult<Reply<VehicleInfoReply>> {
    let client = Client::new("at").provider(Provider::new());
    let request = VehicleInfoRequest::try_from(vehicle_id).context("parsing vehicle id")?;
    let reply = client.request(request).await.context("processing request")?;
    Ok(reply)
}

#[axum::debug_handler]
async fn set_trip(
    Path((vehicle_id, trip_id)): Path<(String, String)>,
) -> HttpResult<Reply<SetTripReply>> {
    let client = Client::new("at").provider(Provider::new());
    let request = SetTripRequest::try_from((vehicle_id, trip_id)).context("parsing vehicle id")?;
    let reply = client.request(request).await.context("processing request")?;
    Ok(reply)
}

#[axum::debug_handler]
async fn reset(Path(vehicle_id): Path<String>) -> HttpResult<Reply<ResetReply>> {
    let client = Client::new("at").provider(Provider::new());
    let request = ResetRequest::try_from(vehicle_id).context("parsing vehicle id")?;
    let reply = client.request(request).await.context("processing request")?;
    Ok(reply)
}
