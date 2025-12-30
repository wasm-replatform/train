use anyhow::{Context, Result};
use dilax_adapter::DilaxMessage;
use fabric::api::Client;
use r9k_adapter::R9kMessage;
use smartrak_gtfs::{CafAvlMessage, PassengerCountMessage, SmarTrakMessage, TrainAvlMessage};
use wasi_messaging::types::{Error, Message};

use crate::provider::Provider;

pub struct Messaging;

wasi_messaging::export!(Messaging with_types_in wasi_messaging);
#[allow(clippy::future_not_send)]
impl wasi_messaging::incoming_handler::Guest for Messaging {
    #[wasi_otel::instrument(name = "messaging_guest_handle")]
    async fn handle(message: Message) -> Result<(), Error> {
        let topic = message.topic().unwrap_or_default();

        // check we're processing topics for the correct environment
        let env = &Provider::new().config.environment;
        let Some(topic) = topic.strip_prefix(&format!("{env}-")) else {
            return Err(Error::Other("Incorrect environment".to_string()));
        };

        // process message based on topic
        if let Err(e) = match &topic {
            t if t.contains("realtime-r9k.v1") => r9k(&message.data()).await,
            t if t.contains("realtime-dilax-apc.v2") => dilax(&message.data()).await,
            t if t.contains("realtime-r9k-to-smartrak.v1") => smartrak(&message.data()).await,
            t if t.contains("realtime-caf-avl.v1") => caf_avl(&message.data()).await,
            t if t.contains("realtime-train-avl.v1") => train_avl(&message.data()).await,
            t if t.contains("realtime-passenger-count.v1") => {
                passenger_count(&message.data()).await
            }
            _ => {
                return Err(Error::Other("Unhandled topic".to_string()));
            }
        } {
            return Err(Error::Other(e.to_string()));
        }

        Ok(())
    }
}

#[wasi_otel::instrument]
async fn r9k(message: &[u8]) -> Result<()> {
    let api_client = Client::new("at").provider(Provider::new());
    let request = R9kMessage::try_from(message).context("parsing message")?;
    api_client.request(request).await?;
    Ok(())
}

#[wasi_otel::instrument]
async fn dilax(payload: &[u8]) -> Result<()> {
    let api_client = Client::new("at").provider(Provider::new());
    let request = DilaxMessage::try_from(payload).context("deserializing event")?;
    api_client.request(request).await?;
    Ok(())
}

#[wasi_otel::instrument]
async fn passenger_count(payload: &[u8]) -> Result<()> {
    let api_client = Client::new("at").provider(Provider::new());
    let request = PassengerCountMessage::try_from(payload).context("deserializing event")?;
    api_client.request(request).await?;
    Ok(())
}

#[wasi_otel::instrument]
async fn smartrak(payload: &[u8]) -> Result<()> {
    let api_client = Client::new("at").provider(Provider::new());
    let request = SmarTrakMessage::try_from(payload).context("deserializing event")?;
    api_client.request(request).await?;
    Ok(())
}

#[wasi_otel::instrument]
async fn caf_avl(payload: &[u8]) -> Result<()> {
    let api_client = Client::new("at").provider(Provider::new());
    let request = CafAvlMessage::try_from(payload).context("deserializing event")?;
    api_client.request(request).await?;
    Ok(())
}

#[wasi_otel::instrument]
async fn train_avl(payload: &[u8]) -> Result<()> {
    let api_client = Client::new("at").provider(Provider::new());
    let request = TrainAvlMessage::try_from(payload).context("deserializing event")?;
    api_client.request(request).await?;
    Ok(())
}
