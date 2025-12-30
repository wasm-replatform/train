use common::fleet;
use fabric::api::{Context, Handler, Headers, NoHeaders, Reply};
use fabric::{Config, HttpRequest, Identity, Publisher, Result, StateStore};
use serde::Deserialize;

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

    let headers = NoHeaders;
    SmarTrakMessage::handle(request, Context { owner, provider, headers: &headers }).await?;

    Ok(TrainAvlReply.into())
}

impl<P> Handler<P> for TrainAvlMessage
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    type Error = fabric::Error;
    type Output = TrainAvlReply;

    async fn handle<H>(self, ctx: Context<'_, P, H>) -> Result<Reply<TrainAvlReply>>
    where
        H: Headers,
    {
        handle(ctx.owner, self, ctx.provider).await
    }
}
