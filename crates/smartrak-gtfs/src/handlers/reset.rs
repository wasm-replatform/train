use anyhow::Context as _;
use serde::{Deserialize, Serialize};
use warp_sdk::api::{Context, Handler, Reply};
use warp_sdk::{
    Config, Error, HttpRequest, Identity, IntoBody, Publisher, Result, StateStore, bad_request,
};

use crate::god_mode::god_mode;

#[derive(Debug, Clone, Deserialize)]
pub struct ResetRequest(String);


#[derive(Debug, Clone, Serialize)]
pub struct ResetReply {
    pub message: String,
    pub process: u32,
}

#[allow(clippy::unused_async)]
async fn handle<P>(_owner: &str, request: ResetRequest, _provider: &P) -> Result<Reply<ResetReply>>
where
    P: HttpRequest + Publisher + StateStore + Identity + Config,
{
    let vehicle_id = request.0;

    let Some(god_mode) = god_mode() else {
        return Err(bad_request!("God mode not enabled"));
    };

    if vehicle_id == "all" {
        god_mode.reset_all();
    } else {
        god_mode.reset_vehicle(&vehicle_id);
    }

    Ok(ResetReply { message: "Ok".to_string(), process: 0 }.into())
}

impl<P> Handler<P> for ResetRequest
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    type Error = Error;
    type Input = String;
    type Output = ResetReply;

    fn from_input(input: String) -> Result<Self> {
        Ok(Self(input))
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<ResetReply>> {
        handle(ctx.owner, self, ctx.provider).await
    }
}

impl IntoBody for ResetReply {
    fn into_body(self) -> anyhow::Result<Vec<u8>> {
        serde_json::to_vec(&self).context("serializing reply")
    }
}
