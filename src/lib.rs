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
use wasi_messaging::types::{Client as MsgClient, Message};
use wasi_messaging::{producer, types};

use crate::provider::Provider;

const SERVICE: &str = "r9k-position-adapter";
const SMARTRAK_TOPIC: &str = "realtime-r9k-to-smartrak.v1";
const R9K_TOPIC: &str = "realtime-r9k.v1";

static ENV: LazyLock<String> =
    LazyLock::new(|| env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string()));

pub struct Messaging;
wasi_messaging::export!(Messaging with_types_in wasi_messaging);

impl wasi_messaging::incoming_handler::Guest for Messaging {
    #[wasi_otel::instrument(name = "messaging_guest_handle")]
    async fn handle(message: Message) -> Result<(), types::Error> {
        let topic = message.topic().unwrap_or_default();

        if topic != format!("{}-{R9K_TOPIC}", *ENV) {
            warn!(monotonic_counter.unhandled_topics = 1, topic = %topic, service = %SERVICE);
        }

        if let Err(e) = r9k_message(&message.data()).await {
            error!(monotonic_counter.processing_errors = 1, error = %e, service = %SERVICE);
        }

        Ok(())
    }
}

// Process incoming R9k messages, consolidating error handling.
#[wasi_otel::instrument]
async fn r9k_message(message: &[u8]) -> Result<()> {
    let api_client = Client::new(Provider);
    let request = R9kMessage::try_from(message).context("parsing message")?;
    let response = api_client.request(request).owner("owner").await?;

    let Some(events) = response.body.smartrak_events.as_ref() else { return Ok(()) };

    // publish events 2x in order to properly signal departure from the station
    // (for schedule adherence)
    let dest_topic = format!("{}-{SMARTRAK_TOPIC}", *ENV);

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
