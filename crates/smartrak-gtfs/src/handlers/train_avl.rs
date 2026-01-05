use anyhow::Context as _;
use common::fleet;
use http::HeaderMap;
use serde::Deserialize;
use warp_sdk::api::{Context, Handler, Reply};
use warp_sdk::{Config, Decode, Error, HttpRequest, Identity, Publisher, Result, StateStore};

use crate::SmarTrakMessage;

#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct TrainAvlMessage(SmarTrakMessage);

impl TryFrom<&[u8]> for TrainAvlMessage {
    type Error = serde_json::Error;

    fn try_from(value: &[u8]) -> anyhow::Result<Self, Self::Error> {
        serde_json::from_slice(value)
    }
}

impl Decode for TrainAvlMessage {
    type DecodeError = Error;

    fn decode(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes)
            .context("deserializing TrainAvlMessage")
            .map_err(Into::into)
    }
}

/// Train AVL response.
#[derive(Debug, Clone)]
pub struct TrainAvlReply;

async fn handle<P>(
    owner: &str, request: TrainAvlMessage, provider: &P,
) -> Result<Reply<TrainAvlReply>>
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    let request = request.0;

    // verify vehicle tag is 'train'
    let Some(vehicle_id) = request.vehicle_id() else {
        tracing::debug!("no vehicle identifier found");
        return Ok(TrainAvlReply.into());
    };
    let Some(vehicle) = fleet::vehicle(vehicle_id, provider).await? else {
        tracing::debug!("vehicle info not found for {vehicle_id}");
        return Ok(TrainAvlReply.into());
    };
    if let Some(tag) = vehicle.tag.as_deref().map(str::to_lowercase)
        && tag != "smartrak"
    {
        tracing::debug!("vehicle tag {tag} did not match rules");
        return Ok(TrainAvlReply.into());
    }

    let headers = HeaderMap::default();
    SmarTrakMessage::handle(request, Context { owner, provider, headers: &headers }).await?;

    Ok(TrainAvlReply.into())
}

impl<P> Handler<P> for TrainAvlMessage
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    type Error = warp_sdk::Error;
    type Output = TrainAvlReply;

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<TrainAvlReply>> {
        handle(ctx.owner, self, ctx.provider).await
    }
}
