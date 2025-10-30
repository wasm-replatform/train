//! R9K Position Adapter
//!
//! Transform an R9K XML message into a SmarTrak[`TrainUpdate`].

use anyhow::{Context, anyhow};
use credibil_api::{Body, Handler, Request, Response};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::Error;
use crate::provider::{Key, Provider, Source, SourceData, Time};
use crate::r9k::{R9kMessage, TrainUpdate};
use crate::smartrak::{EventType, MessageData, RemoteData, SmarTrakEvent};
use crate::{EventData, Result, stops};

const MAX_DELAY_SECS: i64 = 60;
const MIN_DELAY_SECS: i64 = -30;

/// R9K response for SmarTrak consumption.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct R9kResponse {
    /// Train update, converted to SmarTrak events.
    pub smartrak_events: Vec<SmarTrakEvent>,
}

async fn handle(
    owner: &str, request: R9kMessage, provider: &impl Provider,
) -> Result<Response<R9kResponse>> {
    let train_update = request.train_update;
    train_update.validate(provider)?;
    let smartrak_events = train_update.into_events(owner, provider).await?;
    Ok(R9kResponse { smartrak_events }.into())
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
    /// Validate the message.
    fn validate(&self, provider: &impl Time) -> Result<()> {
        if self.changes.is_empty() {
            return Err(Error::NoUpdate);
        }

        // An *actual* update will have a +ve arrival or departure time
        let change = &self.changes[0];

        let event_seconds = if change.has_departed {
            change.actual_departure_time
        } else if change.has_arrived {
            change.actual_arrival_time
        } else {
            0
        };

        if event_seconds <= 0 {
            return Err(Error::NoActualUpdate);
        }

        // Validate message delay
        let event_dt = self.created_date.to_timestamp_secs(i64::from(event_seconds));
        let delay_secs = provider.now().timestamp().as_second() - event_dt;

        // TODO: do we need this metric?;
        info!(gauge.r9k_delay = delay_secs);

        if delay_secs > MAX_DELAY_SECS {
            return Err(Error::Outdated(delay_secs));
        }
        if delay_secs < MIN_DELAY_SECS {
            return Err(Error::WrongTime(-delay_secs));
        }

        Ok(())
    }

    /// Transform the R9K message to SmarTrak events
    async fn into_events(
        self, owner: &str, provider: &impl Provider,
    ) -> Result<Vec<SmarTrakEvent>> {
        let changes = &self.changes;
        let change_type = changes[0].r#type;

        // filter out irrelevant updates (not related to trip progress)
        if !change_type.is_relevant() {
            // TODO: do we need this metric?
            info!(monotonic_counter.irrelevant_change_type = 1 ,type = %change_type);
            return Ok(vec![]);
        }

        // is station is relevant?
        let station = changes[0].station;
        let Some(stop_info) =
            stops::stop_info(owner, provider, station, change_type.is_arrival()).await?
        else {
            info!(monotonic_counter.irrelevant_station = 1, station = %station);
            return Ok(vec![]);
        };

        // fetch allocated trains
        let key = Key::BlockMgt(self.train_id());
        let SourceData::BlockMgt(allocated) =
            Source::fetch(provider, owner, &key).await.context("fetching allocated vehicles")?
        else {
            return Err(anyhow!("no vehicles allocated for {key:?}").into());
        };

        // convert to SmarTrak events
        let mut events = vec![];
        for train in allocated {
            events.push(SmarTrakEvent {
                received_at: self.created_date.to_zoned().timestamp(),
                event_type: EventType::Location,
                message_data: MessageData { timestamp: provider.now().timestamp() },
                remote_data: RemoteData { external_id: train.replace(' ', "") },
                location_data: stop_info.clone().into(),
                event_data: EventData {},
            });
        }

        Ok(events)
    }
}
