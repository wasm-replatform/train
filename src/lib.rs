#![cfg(target_arch = "wasm32")]

mod config;
mod provider;

use std::env;
use std::sync::{Arc, LazyLock};

use anyhow::{anyhow, Context, Result};
use credibil_api::Client;
use dilax::api::{
    BlockMgtClient, BlockMgtProvider, CcStaticProvider, CcStaticProviderImpl, FleetApiProvider,
    FleetProvider, GtfsStaticProvider, GtfsStaticProviderImpl,
};
use dilax::processor::DilaxProcessor;
use dilax::provider::HttpRequest as DilaxHttpRequest;
use dilax::store::KvStore;
use dilax::types::{DilaxEnrichedEvent, DilaxEvent};
use r9k_position::{R9kMessage, SmarTrakEvent};
use tracing::{error, info, warn};
use wasi_messaging::incoming_handler;
use wasi_messaging::types::{Client as MsgClient, Message};
use wasi_messaging::{producer, types};

use crate::provider::{Provider as R9kProvider, WasiHttpClient};

const SERVICE: &str = "r9k-position-adapter";
const SMARTRAK_TOPIC: &str = "realtime-r9k-to-smartrak.v1";
const DEFAULT_OWNER: &str = "owner";

static ENVIRONMENT: LazyLock<String> =
    LazyLock::new(|| env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string()));
static OWNER: LazyLock<String> =
    LazyLock::new(|| env::var("R9K_OWNER").unwrap_or_else(|_| DEFAULT_OWNER.to_string()));

pub struct Messaging;

wasi_messaging::export!(Messaging with_types_in wasi_messaging);

impl incoming_handler::Guest for Messaging {
    #[allow(clippy::future_not_send)]
    async fn handle(message: Message) -> Result<(), types::Error> {
        let topic = message.topic().unwrap_or_default();
        let payload = message.data();

        let r9k_topic = config::get_r9k_source_topic();
        let dilax_topic = config::get_dilax_source_topic();

        if topic == r9k_topic {
            if let Err(err) = process_r9k(&payload).await {
                error!(
                    monotonic_counter.processing_errors = 1,
                    error = %err,
                    topic = %topic,
                    service = %SERVICE
                );
            }
            return Ok(());
        }

        if topic == dilax_topic {
            if let Err(err) = process_dilax(&payload).await {
                error!(
                    monotonic_counter.processing_errors = 1,
                    error = %err,
                    topic = %topic,
                    service = %SERVICE
                );
            }
            return Ok(());
        }

        warn!(monotonic_counter.unhandled_topics = 1, topic = %topic, service = %SERVICE);
        Ok(())
    }
}
#[allow(clippy::future_not_send)]
async fn process_r9k(payload: &[u8]) -> Result<()> {
    let message = R9kMessage::try_from(payload).context("parsing R9K payload")?;
    let client = Client::new(R9kProvider);
    let response = client
        .request(message)
        .owner(OWNER.as_str())
        .await
        .context("processing R9K request")?;

    if let Some(events) = response.body.smartrak_events.as_ref() {
        publish_r9k(events).await?;
    }

    Ok(())
}
#[allow(clippy::future_not_send)]
async fn publish_r9k(events: &[SmarTrakEvent]) -> Result<()> {
    if events.is_empty() {
        return Ok(());
    }

    let client = MsgClient::connect("<not used>").context("connecting to message broker")?;
    let topic = format!("{}-{SMARTRAK_TOPIC}", ENVIRONMENT.as_str());

    for event in events {
        let payload = serde_json::to_vec(event).context("serializing SmarTrak event")?;
        let message = Message::new(&payload);
        if !event.remote_data.external_id.is_empty() {
            message.add_metadata("key", &event.remote_data.external_id);
        }

        producer::send(&client, topic.clone(), message)
            .await
            .map_err(|err| anyhow!("failed to publish R9K event: {err}"))?;

        info!(
            monotonic_counter.messages_sent = 1,
            external_id = %event.remote_data.external_id,
            service = %SERVICE
        );
    }

    Ok(())
}
#[allow(clippy::future_not_send)]
async fn process_dilax(payload: &[u8]) -> Result<()> {
    let event: DilaxEvent = serde_json::from_slice(payload).context("deserializing Dilax event")?;

    let config = dilax::config::Config::default();
    let vehicle_label_key = config.redis.vehicle_label_key.clone().into_owned();
    let kv_store = KvStore::open("dilax").context("opening dilax store")?;
    let http_client: Arc<dyn DilaxHttpRequest> = Arc::new(WasiHttpClient);

    let fleet: Arc<dyn FleetProvider> = Arc::new(FleetApiProvider::new(
        kv_store.clone(),
        config::get_fleet_api_url(),
        vehicle_label_key,
        Arc::clone(&http_client),
    ));
    let cc_static: Arc<dyn CcStaticProvider> = Arc::new(CcStaticProviderImpl::new(
        config::get_gtfs_cc_static_url(),
        Arc::clone(&http_client),
    ));
    let gtfs: Arc<dyn GtfsStaticProvider> = Arc::new(GtfsStaticProviderImpl::new(
        kv_store.clone(),
        config::get_gtfs_static_url(),
        Arc::clone(&http_client),
    ));
    let block: Arc<dyn BlockMgtProvider> = Arc::new(BlockMgtClient::new(
        config::get_block_mgt_url(),
        config::get_block_mgt_bearer_token(),
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

    producer::send(&client, config::get_dilax_outbound_topic(), message)
        .await
        .map_err(|err| anyhow!("failed to publish Dilax event: {err}"))?;

    info!(monotonic_counter.messages_sent = 1, service = %SERVICE, event = "dilax");

    Ok(())
}
