use std::convert::Infallible;

use common::fleet::{self, Vehicle};
use fabric::api::{Handler, Request, Response};
use fabric::{Config, Error, HttpRequest, Identity, Publisher, Result, StateStore};
use serde::{Deserialize, Serialize};

use crate::trip::TripInstance;

#[derive(Debug, Clone, Deserialize)]
pub struct VehicleInfoRequest(String);

#[allow(clippy::infallible_try_from)]
impl TryFrom<String> for VehicleInfoRequest {
    type Error = Infallible;

    fn try_from(value: String) -> anyhow::Result<Self, Self::Error> {
        Ok(Self(value))
    }
}

const PROCESS_ID: u32 = 0;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleInfoResponse {
    pub pid: u32,
    pub vehicle_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sign_on_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trip_info: Option<TripInstance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fleet_info: Option<Vehicle>,
}

async fn handle<P>(
    _owner: &str, request: VehicleInfoRequest, provider: &P,
) -> Result<Response<VehicleInfoResponse>>
where
    P: HttpRequest + Publisher + StateStore + Identity + Config,
{
    let vehicle_id = request.0;

    let trip_key = format!("smartrakGtfs:trip:vehicle:{vehicle_id}");
    let trip_info = if let Some(bytes) = StateStore::get(provider, &trip_key).await? {
        Some(serde_json::from_slice::<TripInstance>(&bytes)?)
    } else {
        None
    };

    let sign_on_key = format!("smartrakGtfs:vehicle:signOn:{vehicle_id}");
    let sign_on_time = StateStore::get(provider, &sign_on_key)
        .await?
        .map(|bytes| String::from_utf8_lossy(&bytes).to_string());

    let fleet_info = fleet::vehicle(&vehicle_id, provider).await?;

    Ok(VehicleInfoResponse { pid: PROCESS_ID, vehicle_id, sign_on_time, trip_info, fleet_info }
        .into())
}

impl<P> Handler<VehicleInfoResponse, P> for Request<VehicleInfoRequest>
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    type Error = Error;

    async fn handle(self, owner: &str, provider: &P) -> Result<Response<VehicleInfoResponse>> {
        handle(owner, self.body, provider).await
    }
}
