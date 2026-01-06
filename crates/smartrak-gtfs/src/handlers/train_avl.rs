use anyhow::Context as _;
use common::fleet;
use http::HeaderMap;
use serde::Deserialize;
use warp_sdk::api::{Context, Handler, Reply};
use warp_sdk::{Config, HttpRequest, Identity, Publisher, Result, StateStore};

use crate::SmarTrakMessage;

#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct TrainAvlMessage(SmarTrakMessage);

async fn handle<P>(owner: &str, request: TrainAvlMessage, provider: &P) -> Result<Reply<()>>
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    let request = request.0;

    // verify vehicle tag is 'train'
    let Some(vehicle_id) = request.vehicle_id() else {
        tracing::debug!("no vehicle identifier found");
        return Ok(Reply::ok(()));
    };
    let Some(vehicle) = fleet::vehicle(vehicle_id, provider).await? else {
        tracing::debug!("vehicle info not found for {vehicle_id}");
        return Ok(Reply::ok(()));
    };
    if let Some(tag) = vehicle.tag.as_deref().map(str::to_lowercase)
        && tag != "smartrak"
    {
        tracing::debug!("vehicle tag {tag} did not match rules");
        return Ok(Reply::ok(()));
    }

    let headers = HeaderMap::default();
    SmarTrakMessage::handle(request, Context { owner, provider, headers: &headers }).await?;

    Ok(Reply::ok(()))
}

impl<P> Handler<P> for TrainAvlMessage
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    type Error = warp_sdk::Error;
    type Input = Vec<u8>;
    type Output = ();

    fn from_input(input: Vec<u8>) -> Result<Self> {
        serde_json::from_slice(&input).context("deserializing TrainAvlMessage").map_err(Into::into)
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<()>> {
        handle(ctx.owner, self, ctx.provider).await
    }
}
