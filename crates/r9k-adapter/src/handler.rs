//! R9K Position Adapter
//!
//! Transform an R9K XML message into a SmarTrak[`TrainUpdate`].

use anyhow::Context as _;
use bytes::Bytes;
use chrono::Utc;
use http::header::AUTHORIZATION;
use http_body_util::Empty;
use serde::Deserialize;
use warp_sdk::api::{Context, Handler, Reply};
use warp_sdk::{Config, Error, HttpRequest, Identity, Message, Publisher, Result};

use crate::r9k::TrainUpdate;
use crate::smartrak::{EventType, MessageData, RemoteData, SmarTrakEvent};
use crate::stops;

const SMARTRAK_TOPIC: &str = "realtime-r9k-to-smartrak.v1";

/// R9K train update message as deserialized from the XML received from
/// KiwiRail.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct R9kMessage {
    /// The train update.
    #[serde(rename(deserialize = "ActualizarDatosTren"))]
    pub train_update: TrainUpdate,
}

async fn handle<P>(owner: &str, request: R9kMessage, provider: &P) -> Result<Reply<()>>
where
    P: Config + HttpRequest + Identity + Publisher,
{
    // validate message
    let update = request.train_update;
    update.validate()?;

    // convert to SmarTrak events
    let events = update.into_events(owner, provider).await?;

    // publish events to SmarTrak topic
    // publish 2x in order to properly signal departure from the station
    // (for schedule adherence)
    let env = Config::get(provider, "ENV").await.unwrap_or_else(|_| "dev".to_string());
    let topic = format!("{env}-{SMARTRAK_TOPIC}");

    for _ in 0..2 {
        #[cfg(not(debug_assertions))]
        std::thread::sleep(std::time::Duration::from_secs(5));

        for event in &events {
            tracing::info!(monotonic_counter.smartrak_events_published = 1);

            let payload = serde_json::to_vec(&event).context("serializing event")?;
            let external_id = &event.remote_data.external_id;

            let mut message = Message::new(&payload);
            message.headers.insert("key".to_string(), external_id.clone());

            Publisher::send(provider, &topic, &message).await?;
        }
    }

    Ok(Reply::ok(()))
}

impl<P> Handler<P> for R9kMessage
where
    P: Config + HttpRequest + Identity + Publisher,
{
    type Error = Error;
    type Input = Vec<u8>;
    type Output = ();

    fn from_input(input: Vec<u8>) -> Result<Self> {
        quick_xml::de::from_reader(input.as_ref())
            .context("deserializing R9kMessage")
            .map_err(Into::into)
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<()>> {
        handle(ctx.owner, self, ctx.provider).await
    }
}

impl TrainUpdate {
    /// Transform the R9K message to SmarTrak events
    async fn into_events<P>(self, owner: &str, provider: &P) -> Result<Vec<SmarTrakEvent>>
    where
        P: Config + HttpRequest + Identity + Publisher,
    {
        let changes = &self.changes;
        let change_type = changes[0].r#type;

        // filter out irrelevant updates (not related to trip progress)
        if !change_type.is_relevant() {
            // TODO: do we need this metric?
            tracing::info!(monotonic_counter.irrelevant_change_type = 1, type = %change_type);
            return Ok(vec![]);
        }

        // is station is relevant?
        let station = changes[0].station;
        let Some(stop_info) =
            stops::stop_info(owner, provider, station, change_type.is_arrival()).await?
        else {
            tracing::info!(monotonic_counter.irrelevant_station = 1, station = %station);
            return Ok(vec![]);
        };

        // get train allocations for this trip
        let url = Config::get(provider, "BLOCK_MGT_URL").await?;
        let identity = Config::get(provider, "AZURE_IDENTITY").await?;

        let token = Identity::access_token(provider, identity).await?;

        let request = http::Request::builder()
            .uri(format!("{url}/allocations/trips?externalRefId={}", self.train_id()))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Empty::<Bytes>::new())
            .context("building block management request")?;
        let response =
            HttpRequest::fetch(provider, request).await.context("fetching train allocations")?;

        let bytes = response.into_body();
        let allocated: Vec<String> =
            serde_json::from_slice(&bytes).context("deserializing block management response")?;

        // publish `SmarTrak` events
        let mut events = Vec::new();
        for train in allocated {
            events.push(SmarTrakEvent {
                received_at: Utc::now(),
                event_type: EventType::Location,
                message_data: MessageData::default(),
                remote_data: RemoteData {
                    external_id: train.replace(' ', ""),
                    ..RemoteData::default()
                },
                location_data: stop_info.clone().into(),
                ..SmarTrakEvent::default()
            });
        }

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::R9kMessage;

    #[test]
    fn deserialization() {
        let xml = include_str!("../data/sample.xml");
        let message: R9kMessage = quick_xml::de::from_str(xml).expect("should deserialize");

        let update = message.train_update;
        assert_eq!(update.even_train_id, Some("1234".to_string()));
        assert!(!update.changes.is_empty(), "should have changes");
    }
}
