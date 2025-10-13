#![cfg(target_arch = "wasm32")]

//! # R9K  Transformer
//!
//! Transforms R9K messages into SmarTrak events.

mod provider;

use anyhow::{Context, Result};
use credibil_api::Client as ApiClient;
use r9k_position::R9kMessage;
use tracing::{error, warn};
use wit_bindings::messaging;
use wit_bindings::messaging::incoming_handler::Configuration;
use wit_bindings::messaging::types::{Client as MsgClient, Message};
use wit_bindings::messaging::{producer, types};

// use crate::{Error, R9kMessage};

#[allow(dead_code)]
const GTFS_API_ADDR: &str = "https://www-dev-cc-static-api-01.azurewebsites.net";
#[allow(dead_code)]
const BLOCK_MGT_ADDR: &str = "https://www-dev-block-mgt-client-api-01.azurewebsites.net";

const SERVICE: &str = "r9k-position-adapter";

pub struct Messaging;

messaging::export!(Messaging with_types_in wit_bindings::messaging);

impl messaging::incoming_handler::Guest for Messaging {
    #[sdk_otel::instrument(name = "messaging_guest_handle")]
    async fn handle(message: Message) -> Result<(), types::Error> {
        let topic = message.topic().unwrap_or_default();
        if topic != "r9k.request" {
            warn!(monotonic_counter.unhandled_topics = 1, topic = %topic, service = %SERVICE);
        }

        if let Err(e) = process(&message.data()).await {
            error!(monotonic_counter.processing_errors = 1, error = %e, service = %SERVICE);
        }
        Ok(())
    }

    async fn configure() -> Result<Configuration, types::Error> {
        Ok(Configuration { topics: vec!["r9k.request".to_string()] })
    }
}

// Process incoming R9k messages, consolidating error handling.
#[sdk_otel::instrument]
async fn process(message: &[u8]) -> Result<()> {
    let api = ApiClient::new(provider::AppContext::new());
    let request = R9kMessage::try_from(message)?;
    let response = api.request(request).owner("owner").await?;
    let Some(events) = response.body.smartrak_events else { return Ok(()) };

    // forward transformed events
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
