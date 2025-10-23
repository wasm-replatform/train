#![cfg(target_arch = "wasm32")]

//! R9K adapter guest kafka messages and SmarTrak GTFS adapter guest entry point.

mod provider;

use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use axum::extract::Path;
use axum::http::Request;
use axum::routing::get;
use axum::{Json, Router};
use credibil_api::Client as ApiClient;
use r9k_position::R9kMessage;
use smartrak_gtfs::{KafkaWorkflow, RestError, RestService, SerializedMessage, HttpError, Error};
use tracing::{error, info};
use wasi::exports::http::incoming_handler::Guest;
use wasi::http::types::{ IncomingRequest, ResponseOutparam };
use wit_bindings::messaging;
use wit_bindings::messaging::incoming_handler::Configuration;
use wit_bindings::messaging::types::{Client as MsgClient, Message};
use wit_bindings::messaging::{producer, types};

use sdk_http::Result as HttpResult;

#[allow(dead_code)]
const SERVICE: &str = "r9k-position-adapter";

struct HttpGuest;
wasi::http::proxy::export!(HttpGuest);

#[allow(clippy::renamed_function_params)]
impl Guest for HttpGuest {
    fn handle(request: IncomingRequest, response: ResponseOutparam) {
        println!("HTTP request received: {} ", request.path_with_query().unwrap_or_default());
        let router = Router::new()
            .route("/", get(handle_index))
            .route("/info/{vehicle_id}", get(handle_vehicle_info))
            .route("/god-mode/set-trip/:vehicle_id/:trip_id", get(handle_set_vehicle_to_trip))
            .route("/god-mode/reset/{vehicle_id}", get(handle_reset_vehicle));

        let out = sdk_http::serve(router, request);
        ResponseOutparam::set(response, out);
    }
}

#[axum::debug_handler]
#[sdk_otel::instrument]
async fn handle_index(request: Request<axum::body::Body>) -> HttpResult<Json<serde_json::Value>, HttpError> {
    // Log user-agent if present.
    let user_agent = request.headers().get("user-agent").and_then(|v| v.to_str().ok());
    if let Some(agent) = user_agent {
        info!(user_agent = %agent, "root endpoint called");
    } else {
        info!("root endpoint called");
    }
    Ok(Json(serde_json::json!({ "message": "OK" })))
}

#[axum::debug_handler]
#[sdk_otel::instrument]
async fn handle_vehicle_info(Path(vehicle_id): Path<String>) -> HttpResult<Json<serde_json::Value> , HttpError> {
    match RestService::vehicle_info(&vehicle_id) {
        Ok(payload) => Ok(Json(serde_json::json!({ 
            "pid": payload.pid,
            "vehicleId": vehicle_id,
            "signOnTime": payload.sign_on_time,
            "tripInfo": payload.trip_info,
            "fleetInfo": payload.fleet_info
        }))),
        Err(RestError::NotFound) => Err(anyhow!(Error::NotFound("Vehicle not found".to_string())))?,
        Err(err) => {
            error!(vehicle_id = vehicle_id, error = %err, "fetching vehicle info failed");
            Err(anyhow!(Error::Internal("Could not process request".to_string())))?
        }
    }
}

#[axum::debug_handler]
#[sdk_otel::instrument]
async fn handle_set_vehicle_to_trip(Path((vehicle_id, trip_id)): Path<(String, String)>) -> HttpResult<Json<serde_json::Value> , HttpError> {
    match RestService::set_vehicle_to_trip(&vehicle_id, &trip_id) {
        Ok(_payload) =>  Ok(Json(serde_json::json!({ 
            "message": "Ok",
            "process": 0
        }))),
        Err(RestError::NotFound) => Err(anyhow!(Error::NotFound("Vehicle or trip not found".to_string())))?,
        Err(err) => {
            error!(
                vehicle_id = vehicle_id,
                trip_id = trip_id,
                error = %err,
                "setting vehicle to trip failed"
            );
            Err(anyhow!(Error::Internal("Could not process request".to_string())))?
        }
    }
}

#[axum::debug_handler]
#[sdk_otel::instrument]
async fn handle_reset_vehicle(Path(vehicle_id): Path<String>) -> HttpResult<Json<serde_json::Value> , HttpError> {
    match RestService::reset_vehicle(&vehicle_id) {
        Ok(_payload) => Ok(Json(serde_json::json!({ 
            "message": "Ok",
            "process": 0
        }))),
        Err(RestError::NotFound) => Err(anyhow!(Error::NotFound("Vehicle not found".to_string())))?,

        Err(_) => Err(anyhow!(Error::Internal("Could not process request".to_string())))?,        
    }
}

pub struct Messaging;

messaging::export!(Messaging with_types_in wit_bindings::messaging);

impl messaging::incoming_handler::Guest for Messaging {
    #[tracing::instrument(name = "messaging_guest_handle", skip(message))]
    async fn handle(message: Message) -> Result<(), types::Error> {
        let topic = message.topic().unwrap_or_default();

        match topic.as_str() {
            "r9k.request" => {
                if let Err(e) = process_r9k(&message.data()).await {
                    error!(monotonic_counter.processing_errors = 1, error = %e, service = %SERVICE);
                }
            }
            other => {
                process_smartrak_gtfs(other, &message.data()).await;
            }
        }
        Ok(())
    }

    async fn configure() -> Result<Configuration, types::Error> {
        let config = smartrak_gtfs::Config::from_env()
            .map_err(|err| types::Error::Other(err.to_string()))?;
        Ok(Configuration { topics: config.topics.subscriptions.clone() })
    }
}

#[tracing::instrument(skip(message))]
async fn process_r9k(message: &[u8]) -> Result<()> {
    let api = ApiClient::new(provider::AppContext::new());
    let request = R9kMessage::try_from(message)?;
    let response = api.request(request).owner("owner").await?;
    let Some(events) = response.body.smartrak_events else { return Ok(()) };

    for evt in events {
        let client = MsgClient::connect("kafka").context("connecting to message broker")?;
        let msg = serde_json::to_vec(&evt).context("serializing event")?;
        let message = Message::new(&msg);

        wit_bindgen::spawn(async move {
            if let Err(e) = producer::send(client, "r9k.response".to_string(), message).await {
                error!(monotonic_counter.processing_errors = 1, error = %e, service = %SERVICE);
            }
        });
    }

    Ok(())
}

async fn process_smartrak_gtfs(topic: &str, payload: &[u8]) {
    let config = match smartrak_gtfs::Config::from_env() {
        Ok(config) => Arc::new(config),
        Err(err) => {
            error!(monotonic_counter.processing_errors = 1, error = %err, service = %SERVICE, "loading smartrak config failed");
            return;
        }
    };

    let provider = provider::AppContext::new();
    let processor = smartrak_gtfs::Processor::new(Arc::clone(&config), provider, None);
    let workflow = KafkaWorkflow::new(config.as_ref(), processor);

    match workflow.handle(topic, payload).await {
        Ok(outcome) => {
            for message in outcome.produced {
                if let Err(err) = publish_serialized(message).await {
                    error!(monotonic_counter.processing_errors = 1, error = %err, topic = %topic, service = %SERVICE, "failed to publish smartrak output");
                }
            }
            if outcome.passenger_updates > 0 {
                info!(
                    monotonic_counter.smartrak_passenger_updates = outcome.passenger_updates as u64,
                    topic = %topic,
                    service = %SERVICE,
                    "processed passenger count event"
                );
            }
        }
        Err(err) => {
            error!(monotonic_counter.processing_errors = 1, error = %err, topic = %topic, service = %SERVICE, "processing smartrak kafka message failed");
        }
    }
}

async fn publish_serialized(message: SerializedMessage) -> Result<()> {
    let SerializedMessage { topic, key: _key, payload } = message;
    let client = MsgClient::connect("kafka").context("connecting to message broker")?;
    let outgoing = Message::new(&payload);

    producer::send(client, topic, outgoing).await.map_err(|err| anyhow!(err.to_string()))
}
