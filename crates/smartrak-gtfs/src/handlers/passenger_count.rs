//! # Passenger Count
//!
//! This module stores occupancy status for a given vehicle and trip.

use credibil_api::{Handler, Request, Response};
use serde::{Deserialize, Serialize};

use crate::{Error, Provider, Result, StateStore};

/// R9K empty response.
#[derive(Debug, Clone)]
pub struct PassengerCountResponse;

const OCCUPANY_STATUS_TTL: u64 = 3 * 60 * 60; // 3 hours

async fn handle(
    _owner: &str, request: PassengerCountMessage, provider: &impl Provider,
) -> Result<Response<PassengerCountResponse>> {
    // create state key
    let vehicle_id = &request.vehicle.id;
    let Trip { trip_id, start_date, start_time } = &request.trip;
    let key = format!(
        "smartrakGtfs:occupancyStatus:{vehicle_id}:{trip_id}:{start_date}:{start_time}",
    );

    // save occupancy status to state if set, otherwise remove
    if let Some(occupancy_status) = request.occupancy_status {
        let bytes = serde_json::to_vec(&occupancy_status)?;
        StateStore::set(provider, &key, &bytes, Some(OCCUPANY_STATUS_TTL)).await?;
    } else {
        StateStore::delete(provider, &key).await?;
    }

    Ok(PassengerCountResponse.into())
}

impl<P: Provider> Handler<PassengerCountResponse, P> for Request<PassengerCountMessage> {
    type Error = Error;

    // TODO: implement "owner"
    async fn handle(self, owner: &str, provider: &P) -> Result<Response<PassengerCountResponse>> {
        handle(owner, self.body, provider).await
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
