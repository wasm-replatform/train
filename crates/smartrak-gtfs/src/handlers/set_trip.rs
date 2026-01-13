use anyhow::Context as _;
use qwasr_sdk::api::{Context, Handler, Reply};
use qwasr_sdk::{
    Config, Error, HttpRequest, Identity, IntoBody, Publisher, Result, StateStore, bad_request,
};
use serde::{Deserialize, Serialize};

use crate::god_mode;

#[derive(Debug, Clone, Deserialize)]
pub struct SetTripRequest(String, String);

#[derive(Debug, Clone, Serialize)]
pub struct SetTripReply {
    pub message: String,
    pub process: u32,
}

async fn handle<P>(
    _owner: &str, request: SetTripRequest, provider: &P,
) -> Result<Reply<SetTripReply>>
where
    P: HttpRequest + Publisher + StateStore + Identity + Config,
{
    let vehicle_id = request.0;
    let trip_id = request.1;

    if !god_mode::is_enabled(provider).await? {
        return Err(bad_request!("God mode not enabled"));
    }

    god_mode::set_vehicle_to_trip(provider, vehicle_id, trip_id)
        .await
        .context("setting vehicle to trip")?;
    Ok(SetTripReply { message: "Ok".to_string(), process: 0 }.into())
}

impl<P> Handler<P> for SetTripRequest
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    type Error = Error;
    type Input = (String, String);
    type Output = SetTripReply;

    fn from_input(input: (String, String)) -> Result<Self> {
        Ok(Self(input.0, input.1))
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<SetTripReply>> {
        handle(ctx.owner, self, ctx.provider).await
    }
}

impl IntoBody for SetTripReply {
    fn into_body(self) -> anyhow::Result<Vec<u8>> {
        serde_json::to_vec(&self).context("serializing reply")
    }
}
