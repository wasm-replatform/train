use std::convert::Infallible;

use anyhow::Context as _;
use fabric::api::{Context, Handler, Headers, Reply};
use fabric::{
    Config, Error, HttpRequest, Identity, IntoBody, Publisher, Result, StateStore, bad_request,
};
use serde::{Deserialize, Serialize};

use crate::god_mode::god_mode;

#[derive(Debug, Clone, Deserialize)]
pub struct SetTripRequest(String, String);

#[allow(clippy::infallible_try_from)]
impl TryFrom<(String, String)> for SetTripRequest {
    type Error = Infallible;

    fn try_from(value: (String, String)) -> anyhow::Result<Self, Self::Error> {
        Ok(Self(value.0, value.1))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SetTripReply {
    pub message: String,
    pub process: u32,
}

#[allow(clippy::unused_async)]
async fn handle<P>(
    _owner: &str, request: SetTripRequest, _provider: &P,
) -> Result<Reply<SetTripReply>>
where
    P: HttpRequest + Publisher + StateStore + Identity + Config,
{
    let vehicle_id = request.0;
    let trip_id = request.1;

    let Some(god_mode) = god_mode() else {
        return Err(bad_request!("God mode not enabled"));
    };
    god_mode.set_vehicle_to_trip(vehicle_id, trip_id);
    Ok(SetTripReply { message: "Ok".to_string(), process: 0 }.into())
}

impl<P> Handler<P> for SetTripRequest
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    type Error = Error;
    type Output = SetTripReply;

    async fn handle<H>(self, ctx: Context<'_, P, H>) -> Result<Reply<SetTripReply>>
    where
        H: Headers,
    {
        handle(ctx.owner, self, ctx.provider).await
    }
}

impl IntoBody for SetTripReply {
    fn into_body(self) -> anyhow::Result<Vec<u8>> {
        serde_json::to_vec(&self).context("serializing reply")
    }
}
