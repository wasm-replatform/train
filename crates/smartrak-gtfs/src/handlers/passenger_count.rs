//! # Passenger Count
//!
//! This module stores occupancy status for a given vehicle and trip.

use fabric::api::{Context, Handler, Headers, Response};
use fabric::{Config, Error, HttpRequest, Identity, Publisher, Result, StateStore};
use serde::{Deserialize, Serialize};

/// R9K empty response.
#[derive(Debug, Clone)]
pub struct PassengerCountResponse;

const OCCUPANY_STATUS_TTL: u64 = 3 * 60 * 60; // 3 hours

async fn handle<P>(
    _owner: &str, request: PassengerCountMessage, provider: &P,
) -> Result<Response<PassengerCountResponse>>
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

    Ok(PassengerCountResponse.into())
}

impl<P> Handler<P> for PassengerCountMessage
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    type Error = Error;
    type Output = PassengerCountResponse;

    // TODO: implement "owner"
    async fn handle<H>(self, ctx: Context<'_, P, H>) -> Result<Response<PassengerCountResponse>>
    where
        H: Headers,
    {
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

impl TryFrom<&[u8]> for PassengerCountMessage {
    type Error = serde_json::Error;

    fn try_from(value: &[u8]) -> anyhow::Result<Self, Self::Error> {
        serde_json::from_slice(value)
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
