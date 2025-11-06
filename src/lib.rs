#![cfg(target_arch = "wasm32")]
mod provider;

use std::sync::{Arc, LazyLock};
use std::time::Duration;
use std::{env, thread};

use anyhow::{anyhow, Context, Result};
use axum::routing::get;
use axum::{Json, Router};
use credibil_api::Client;
use dilax::api::{
    BlockMgtClient, BlockMgtProvider, CcStaticProvider, CcStaticProviderImpl,
    FleetApiProvider, FleetProvider, GtfsStaticProvider, GtfsStaticProviderImpl,
};
use dilax::detector::run_lost_connection_job;
use dilax::processor::DilaxProcessor;
use dilax::store::KvStore;
use dilax::types::{DilaxEnrichedEvent, DilaxEvent};
use r9k_position::R9kMessage ;
use serde_json::Value;
use tracing::{error, info, warn, Level};
use wasi_http::Result as HttpResult;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};
use wasi_messaging::types::{Client as MsgClient, Message};
use wasi_messaging::{producer, types};

use crate::provider::WasiHttpClient;

const SERVICE: &str = "r9k-position-adapter";
const SMARTRAK_TOPIC: &str = "realtime-r9k-to-smartrak.v1";
const R9K_TOPIC: &str = "realtime-r9k.v1";
const DILAX_TOPIC: &str = "realtime-dilax-apc.v1";
const DILAX_ENRICHED_TOPIC: &str = "realtime-dilax-apc-enriched.v1";

static ENV: LazyLock<String> =
    LazyLock::new(|| env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string()));


pub struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
   #[wasi_otel::instrument(name = "http_guest_handle",level = Level::INFO)]
    async fn handle(request: Request) -> HttpResult<Response, ErrorCode> {
        let router = Router::new().route("/jobs/detector", get(jobs_detector));
        wasi_http::serve(router, request).await
    }
}

#[axum::debug_handler]
async fn jobs_detector() -> HttpResult<Json<Value>> {
    let http_client = Arc::new(WasiHttpClient);
    let detections = run_lost_connection_job(http_client)
        .await
        .context("running Dilax lost connection job")?;

    Ok(Json(serde_json::json!({
        "status": "job detection triggered",
        "detections": detections.len()
    })))
}

pub struct Messaging;

wasi_messaging::export!(Messaging with_types_in wasi_messaging);

#[allow(clippy::future_not_send)]
impl wasi_messaging::incoming_handler::Guest for Messaging {
    #[wasi_otel::instrument(name = "messaging_guest_handle")]
    async fn handle(message: Message) -> Result<(), types::Error> {
        let topic = message.topic().unwrap_or_default();
        if topic == format!("{}-{R9K_TOPIC}", *ENV) {
            if let Err(e) = r9k_message(&message.data()).await {
                error!(
                    monotonic_counter.processing_errors = 1, 
                    error = %e,
                    topic = %topic,
                    service = %SERVICE
                );
            }
        } else if topic == format!("{}-{DILAX_TOPIC}", *ENV) {
            if let Err(e) = process_dilax(&message.data()).await {
                error!(
                    monotonic_counter.processing_errors = 1,
                    error = %e,
                    topic = %topic,
                    service = %SERVICE
                );
            }
        } else {
            warn!(monotonic_counter.unhandled_topics = 1, topic = %topic, service = %SERVICE);
        }

        
        Ok(())
    }
}

// Process incoming R9k messages, consolidating error handling.
#[wasi_otel::instrument]
async fn r9k_message(message: &[u8]) -> Result<()> {
    let dest_topic = format!("{}-{SMARTRAK_TOPIC}", *ENV);

    let api = Client::new(provider::Provider);
    let request = R9kMessage::try_from(message).context("parsing message")?;
    let response = api.request(request).owner("owner").await?;
    let Some(events) = response.body.smartrak_events.as_ref() else { return Ok(()) };

    // publish events 2x in order to properly signal departure from the station
    // (for schedule adherence)
    for _ in 0..2 {
        thread::sleep(Duration::from_secs(5));

        for evt in events {
            let external_id = &evt.remote_data.external_id;
            let msg = serde_json::to_vec(&evt).context("serializing event")?;
            let message = Message::new(&msg);
            message.add_metadata("key", external_id);

            let client = MsgClient::connect("").context("connecting to message broker")?;
            let topic = dest_topic.clone();

            wit_bindgen::spawn(async move {
                if let Err(e) = producer::send(&client, topic, message).await {
                    error!(
                        monotonic_counter.processing_errors = 1, error = %e, service = %SERVICE
                    );
                }
            });

            info!(
                monotonic_counter.messages_sent = 1, external_id = %external_id, service = %SERVICE
            );
        }
    }

    Ok(())
}

#[allow(clippy::future_not_send)]
async fn process_dilax(payload: &[u8]) -> Result<()> {
    let event: DilaxEvent = serde_json::from_slice(payload).context("deserializing Dilax event")?;

    let config = dilax::config::Config::default();
    let vehicle_label_key = config.redis.vehicle_label_key.clone().into_owned();
    let kv_store = KvStore::open("dilax").context("opening dilax store")?;
    let http_client = Arc::new(WasiHttpClient);

    let fleet: Arc<dyn FleetProvider> = Arc::new(FleetApiProvider::new(
        kv_store.clone(),
        vehicle_label_key,
        Arc::clone(&http_client),
    ));
    let cc_static: Arc<dyn CcStaticProvider> = Arc::new(CcStaticProviderImpl::new(
        Arc::clone(&http_client),
    ));
    let gtfs: Arc<dyn GtfsStaticProvider> = Arc::new(GtfsStaticProviderImpl::new(
        kv_store.clone(),
        Arc::clone(&http_client),
    ));
    let block: Arc<dyn BlockMgtProvider> = Arc::new(BlockMgtClient::new(        
        Arc::clone(&http_client),
    ));

    let processor = DilaxProcessor::with_providers(config, kv_store, fleet, cc_static, gtfs, block);

    let enriched = processor
        .process(event)
        .await
        .context("processing Dilax event")?;

    publish_dilax(&enriched).await?;

    Ok(())
}
#[allow(clippy::future_not_send)]
async fn publish_dilax(event: &DilaxEnrichedEvent) -> Result<()> {
    let client = MsgClient::connect("<not used>").context("connecting to message broker")?;
    let payload = serde_json::to_vec(event).context("serializing Dilax enriched event")?;
    let message = Message::new(&payload);
    if let Some(key) = event.trip_id.as_deref() {
        message.add_metadata("key", key);
    }

    producer::send(&client, format!("{}-{DILAX_ENRICHED_TOPIC}", *ENV), message)
        .await
        .map_err(|err| anyhow!("failed to publish Dilax event: {err}"))?;

    info!(monotonic_counter.messages_sent = 1, service = %SERVICE, event = "dilax");

    Ok(())
}
