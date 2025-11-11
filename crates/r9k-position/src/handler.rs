//! R9K Position Adapter
//!
//! Transform an R9K XML message into a SmarTrak[`TrainUpdate`].

use std::env;

use anyhow::Context;
use bytes::Bytes;
use chrono::Utc;
use credibil_api::{Body, Handler, Request, Response};
use http::header::AUTHORIZATION;
use http_body_util::Empty;
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::provider::{HttpRequest, Identity, Provider};
use crate::r9k::{R9kMessage, TrainUpdate};
use crate::smartrak::{EventType, MessageData, RemoteData, SmarTrakEvent};
use crate::{Result, stops};

/// R9K response for SmarTrak consumption.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct R9kResponse {
    /// Train update, converted to SmarTrak events.
    pub smartrak_events: Option<Vec<SmarTrakEvent>>,
}

async fn handle(
    owner: &str, request: R9kMessage, provider: &impl Provider,
) -> Result<Response<R9kResponse>> {
    let train_update = request.train_update;
    train_update.validate()?;
    let events = train_update.into_events(owner, provider).await?;
    Ok(R9kResponse { smartrak_events: Some(events) }.into())
}

impl<P: Provider> Handler<R9kResponse, P> for Request<R9kMessage> {
    type Error = Error;

    // TODO: implement "owner"
    async fn handle(self, owner: &str, provider: &P) -> Result<Response<R9kResponse>> {
        handle(owner, self.body, provider).await
    }
}

impl Body for R9kMessage {}

impl TrainUpdate {
    /// Transform the R9K message to SmarTrak events
    async fn into_events(
        self, owner: &str, provider: &impl Provider,
    ) -> Result<Vec<SmarTrakEvent>> {
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
        let url = env::var("BLOCK_MGT_URL").context("getting `BLOCK_MGT_URL`")?;
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

        // convert to SmarTrak events
        let mut events = vec![];
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
