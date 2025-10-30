#![cfg(target_arch = "wasm32")]

//! # R9K  Transformer
//!
//! Transforms R9K messages into SmarTrak events.

mod block_mgt;
mod config;
mod gtfs;
mod provider;


use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use credibil_api::Client as ApiClient;
use dilax::api::{
    BlockMgtClient, BlockMgtProvider, CcStaticProvider, CcStaticProviderImpl, FleetApiProvider,
    FleetProvider, GtfsStaticProvider, GtfsStaticProviderImpl,
};
use dilax::processor::DilaxProcessor;
use dilax::provider::HttpRequest as DilaxHttpRequest;
use dilax::store::KvStore;
use dilax::types::{DilaxEnrichedEvent, DilaxEvent};
use r9k_position::{R9kMessage, SmarTrakEvent};
use tracing::{error, warn};
use wit_bindings::messaging;
use wit_bindings::messaging::incoming_handler::Configuration;
use wit_bindings::messaging::types::{Client as MsgClient, Message};
use wit_bindings::messaging::{producer, types};


use crate::provider::{ AppContext, WasiHttpClient };

const SERVICE: &str = "r9k-position-adapter";

pub struct Messaging;

messaging::export!(Messaging with_types_in wit_bindings::messaging);

impl messaging::incoming_handler::Guest for Messaging {
    #[sdk_otel::instrument(name = "messaging_guest_handle")]
    async fn handle(message: Message) -> Result<(), types::Error> {
        let topic = message.topic().unwrap_or_default();
        let r9k_topic = config::get_r9k_source_topic();
        let dilax_topic = config::get_dilax_source_topic();

        if topic != r9k_topic && topic != dilax_topic {
            warn!(monotonic_counter.unhandled_topics = 1, topic = %topic, service = %SERVICE);
        }

        match topic.as_str() {
            current if current == r9k_topic.as_str() => {
                if let Err(e) = process_r9k(&message.data()).await {
                    error!(monotonic_counter.processing_errors = 1, error = %e, service = %SERVICE);
                }
            }
            current if current == dilax_topic.as_str() => {
                if let Err(e) = process_dilax(&message.data()).await {
                    error!(monotonic_counter.processing_errors = 1, error = %e, service = %SERVICE);
                }
            }
            _ => {}
        }

        if let Err(e) = process_r9k(&message.data()).await {
            error!(monotonic_counter.processing_errors = 1, error = %e, service = %SERVICE);
        }
        Ok(())
    }

    async fn configure() -> Result<Configuration, types::Error> {
        Ok(Configuration {
            topics: vec![config::get_r9k_source_topic(), config::get_dilax_source_topic()],
        })
    }
}

// Process incoming R9k messages, consolidating error handling.
#[sdk_otel::instrument]
async fn process_r9k(message: &[u8]) -> Result<()> {
    let context = AppContext::default();
    let api = ApiClient::new(context);
    let request =
        R9kMessage::try_from(message).context(String::from_utf8_lossy(message).to_string())?;
    let response = api.request(request).owner("owner").await?;

    // This twoTap is used for schedule adherence to depart vehicle from the station properly
    thread::sleep(Duration::from_secs(5));
    publish_r9k(&response.body.smartrak_events)?;

    thread::sleep(Duration::from_secs(5));
    publish_r9k(&response.body.smartrak_events)?;

    Ok(())
}

fn publish_r9k(events: &[SmarTrakEvent]) -> Result<()> {
    let now = jiff::Timestamp::now();
    for evt in events {
        let client = MsgClient::connect("<not used>").context("connecting to message broker")?;
        let key = evt.remote_data.external_id.clone();
        let value = evt.clone_with_new_message_timestamp(now);
        let msg = serde_json::to_vec(&value).unwrap();

        let message = Message::new(&msg);
        message.add_metadata("key", &key);

        wit_bindgen::spawn(async move {
            if let Err(e) = producer::send(client, "r9k.response".to_string(), message).await {
                error!(monotonic_counter.processing_errors = 1, error = %e, service = %SERVICE);
            }
        });
    }
    Ok(())
}

#[sdk_otel::instrument]
async fn process_dilax(message: &[u8]) -> Result<()> {
    let event: DilaxEvent =
        serde_json::from_slice(message).context(String::from_utf8_lossy(message).to_string())?;
    let config = dilax::config::Config::default();
    let kv_store = KvStore::open("dilax").context("opening dilax store")?;

    let vehicle_label_key = config.redis.vehicle_label_key.clone().into_owned();
    let fleet_api_url = config::get_fleet_api_url();
    let block_mgt_url = config::get_block_mgt_url();
    let gtfs_static_url = config::get_gtfs_static_url();
    let cc_static_url = config::get_gtfs_cc_static_url();
    let block_mgt_bearer = config::get_block_mgt_bearer_token();

    let http_client: Arc<dyn DilaxHttpRequest> = Arc::new(WasiHttpClient);

    let fleet: Arc<dyn FleetProvider> = Arc::new(FleetApiProvider::new(
        kv_store.clone(),
        fleet_api_url,
        vehicle_label_key,
        Arc::clone(&http_client),
    ));
    let cc_static: Arc<dyn CcStaticProvider> =
        Arc::new(CcStaticProviderImpl::new(cc_static_url, Arc::clone(&http_client)));
    let gtfs: Arc<dyn GtfsStaticProvider> = Arc::new(GtfsStaticProviderImpl::new(
        kv_store.clone(),
        gtfs_static_url,
        Arc::clone(&http_client),
    ));
    let block: Arc<dyn BlockMgtProvider> =
        Arc::new(BlockMgtClient::new(block_mgt_url, block_mgt_bearer, http_client));

    let processor = DilaxProcessor::with_providers(
        config,
        kv_store,
        fleet,
        cc_static,
        gtfs,
        block,
    );

    let enriched =
        processor.process(event).await.context(String::from_utf8_lossy(message).to_string())?;

    publish_dilax(&enriched)?;

    Ok(())
}

fn publish_dilax(event: &DilaxEnrichedEvent) -> Result<()> {
    let client = MsgClient::connect("<not used>").context("connecting to message broker")?;    
    let key = event.trip_id.clone().unwrap_or_default();
    let msg = serde_json::to_vec(&event).unwrap();
    let message = Message::new(&msg);
    message.add_metadata("key", &key);
    wit_bindgen::spawn(async move {
        if let Err(e) = producer::send(client, config::get_dilax_outbound_topic(), message).await {
            error!(monotonic_counter.processing_errors = 1, error = %e, service = %SERVICE);
        }
    });
    Ok(())
}
