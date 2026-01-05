//! # Passenger Count
//!
//! This module stores occupancy status for a given vehicle and trip.

use anyhow::Context as _;
use serde::{Deserialize, Serialize};
use warp_sdk::api::{Context, Handler, Reply};
use warp_sdk::{Config, Error, HttpRequest, Identity, Publisher, Result, StateStore};

/// R9K empty response.
#[derive(Debug, Clone)]
pub struct PassengerCountReply;

const OCCUPANY_STATUS_TTL: u64 = 3 * 60 * 60; // 3 hours

async fn handle<P>(
    _owner: &str, request: PassengerCountMessage, provider: &P,
) -> Result<Reply<PassengerCountReply>>
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    // create state key
    let vehicle_id = &request.vehicle.id;
    let Trip { trip_id, start_date, start_time } = &request.trip;
    let key =
        format!("smartrakGtfs:occupancyStatus:{vehicle_id}:{trip_id}:{start_date}:{start_time}",);

    // save occupancy status to state if set, otherwise remove
    if let Some(occupancy_status) = request.occupancy_status {
        let bytes = serde_json::to_vec(&occupancy_status)?;
        StateStore::set(provider, &key, &bytes, Some(OCCUPANY_STATUS_TTL)).await?;
    } else {
        StateStore::delete(provider, &key).await?;
    }

    Ok(PassengerCountReply.into())
}

impl<P> Handler<P> for PassengerCountMessage
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    type Input = Vec<u8>;
    type Error = Error;
    type Output = PassengerCountReply;

    // TODO: implement "owner"
    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<PassengerCountReply>> {
        handle(ctx.owner, self, ctx.provider).await
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PassengerCountMessage {
    pub occupancy_status: Option<String>,
    pub vehicle: Vehicle,
    pub trip: Trip,
    pub timestamp: i64,
}

impl TryFrom<Vec<u8>> for PassengerCountMessage {
    type Error = Error;

    fn try_from(value: Vec<u8>) -> Result<Self> {
        serde_json::from_slice(&value)
            .context("deserializing PassengerCountMessage")
            .map_err(Into::into)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Vehicle {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Trip {
    pub trip_id: String,
    pub start_date: String,
    pub start_time: String,
}
