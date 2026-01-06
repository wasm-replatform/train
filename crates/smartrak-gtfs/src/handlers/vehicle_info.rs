use anyhow::Context as _;
use common::fleet::{self, Vehicle};
use serde::{Deserialize, Serialize};
use warp_sdk::api::{Context, Handler, Reply};
use warp_sdk::{Config, Error, HttpRequest, Identity, IntoBody, Publisher, Result, StateStore};

use crate::trip::TripInstance;

#[derive(Debug, Clone, Deserialize)]
pub struct VehicleInfoRequest(String);


const PROCESS_ID: u32 = 0;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleInfoReply {
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
) -> Result<Reply<VehicleInfoReply>>
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

    Ok(VehicleInfoReply { pid: PROCESS_ID, vehicle_id, sign_on_time, trip_info, fleet_info }.into())
}

impl<P> Handler<P> for VehicleInfoRequest
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    type Error = Error;
    type Input = String;
    type Output = VehicleInfoReply;

    fn from_input(input: String) -> Result<Self> {
        Ok(Self(input))
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<VehicleInfoReply>> {
        handle(ctx.owner, self, ctx.provider).await
    }
}

impl IntoBody for VehicleInfoReply {
    fn into_body(self) -> anyhow::Result<Vec<u8>> {
        serde_json::to_vec(&self).context("serializing reply")
    }
}
