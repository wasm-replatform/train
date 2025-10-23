#![cfg(target_arch = "wasm32")]

//! # R9K  Transformer
//!
//! Transforms R9K messages into SmarTrak events.

mod provider;

use std::sync::LazyLock;
use std::time::Duration;
use std::{env, thread};

use anyhow::{Context, Result};
use credibil_api::Client;
use r9k_position::R9kMessage;
use tracing::{error, info, warn};
use wit_bindings::messaging;
use wit_bindings::messaging::incoming_handler::Configuration;
use wit_bindings::messaging::types::{Client as MsgClient, Message};
use wit_bindings::messaging::{producer, types};

// use crate::{Error, R9kMessage};

const SERVICE: &str = "r9k-position-adapter";
const SMARTRAK_TOPIC: &str = "dev-realtime-r9k-to-smartrak.v1";
static R9K_TOPIC: LazyLock<String> =
    LazyLock::new(|| env::var("R9K_TOPIC").unwrap_or_else(|_| "dev-realtime-r9k.v1".to_string()));

pub struct Messaging;
messaging::export!(Messaging with_types_in wit_bindings::messaging);

impl messaging::incoming_handler::Guest for Messaging {
    #[sdk_otel::instrument(name = "messaging_guest_handle")]
    async fn handle(message: Message) -> Result<(), types::Error> {
        let topic = message.topic().unwrap_or_default();
        if topic != *R9K_TOPIC {
            warn!(monotonic_counter.unhandled_topics = 1, topic = %topic, service = %SERVICE);
        }

        if let Err(e) = r9k_message(&message.data()).await {
            error!(monotonic_counter.processing_errors = 1, error = %e, service = %SERVICE);
        }
        Ok(())
    }

    async fn configure() -> Result<Configuration, types::Error> {
        Ok(Configuration { topics: vec![R9K_TOPIC.clone()] })
    }
}

// Process incoming R9k messages, consolidating error handling.
#[sdk_otel::instrument]
async fn r9k_message(message: &[u8]) -> Result<()> {
    let dest_topic = env::var("SMARTRAK_TOPIC").unwrap_or_else(|_| SMARTRAK_TOPIC.to_string());

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
                if let Err(e) = producer::send(client, topic, message).await {
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
