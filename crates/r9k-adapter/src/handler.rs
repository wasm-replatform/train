//! R9K Position Adapter
//!
//! Transform an R9K XML message into a SmarTrak[`TrainUpdate`].

use anyhow::Context;
use bytes::Bytes;
use chrono::Utc;
use credibil_api::{Handler, Request, Response};
use http::header::AUTHORIZATION;
use http_body_util::Empty;
use realtime::{Config, Error, HttpRequest, Identity, Message, Publisher, Result};

use crate::r9k::{R9kMessage, TrainUpdate};
use crate::smartrak::{EventType, MessageData, RemoteData, SmarTrakEvent};
use crate::stops;

const SMARTRAK_TOPIC: &str = "realtime-r9k-to-smartrak.v1";

/// R9K empty response.
#[derive(Debug, Clone)]
pub struct R9kResponse;

async fn handle<P>(owner: &str, request: R9kMessage, provider: &P) -> Result<Response<R9kResponse>>
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
    for _ in 0..2 {
        #[cfg(not(debug_assertions))]
        std::thread::sleep(std::time::Duration::from_secs(5));

        for event in &events {
            tracing::info!(monotonic_counter.smartrak_events_published = 1);

            let payload = serde_json::to_vec(&event).context("serializing event")?;
            let external_id = &event.remote_data.external_id;

            let mut message = Message::new(&payload);
            message.headers.insert("key".to_string(), external_id.clone());

            Publisher::send(provider, SMARTRAK_TOPIC, &message).await?;
        }
    }

    Ok(R9kResponse.into())
}

impl<P> Handler<R9kResponse, P> for Request<R9kMessage>
where
    P: Config + HttpRequest + Identity + Publisher,
{
    type Error = Error;

    // TODO: implement "owner"
    async fn handle(self, owner: &str, provider: &P) -> Result<Response<R9kResponse>> {
        handle(owner, self.body, provider).await
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
        let url =
            Config::get(provider, "BLOCK_MGT_URL").await.context("getting `BLOCK_MGT_URL`")?;
        let token = Identity::access_token(provider).await?;

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
