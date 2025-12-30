use common::fleet;
use fabric::api::{Context, Handler, Headers, NoHeaders, Reply};
use fabric::{Config, Error, HttpRequest, Identity, Publisher, Result, StateStore};
use serde::Deserialize;

use crate::SmarTrakMessage;

#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct CafAvlMessage(SmarTrakMessage);

impl TryFrom<&[u8]> for CafAvlMessage {
    type Error = serde_json::Error;

    fn try_from(value: &[u8]) -> anyhow::Result<Self, Self::Error> {
        serde_json::from_slice(value)
    }
}

/// CAF AVL response.
#[derive(Debug, Clone)]
pub struct CafAvlReply;

async fn handle<P>(owner: &str, request: CafAvlMessage, provider: &P) -> Result<Reply<CafAvlReply>>
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    let request = request.0;

    // verify vehicle tag is 'caf'
    let Some(vehicle_id) = request.vehicle_id() else {
        tracing::debug!("no vehicle identifier found");
        return Ok(CafAvlReply.into());
    };
    let Some(vehicle) = fleet::vehicle(vehicle_id, provider).await? else {
        tracing::debug!("vehicle info not found for {vehicle_id}");
        return Ok(CafAvlReply.into());
    };
    if let Some(tag) = vehicle.tag.as_deref().map(str::to_lowercase)
        && tag != "caf"
    {
        tracing::debug!("vehicle tag {tag} did not match rules");
        return Ok(CafAvlReply.into());
    }

    let headers = NoHeaders;
    SmarTrakMessage::handle(request, Context { owner, provider, headers: &headers }).await?;
    Ok(CafAvlReply.into())
}

impl<P> Handler<P> for CafAvlMessage
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    type Error = Error;
    type Output = CafAvlReply;

    async fn handle<H>(self, ctx: Context<'_, P, H>) -> Result<Reply<CafAvlReply>>
    where
        H: Headers,
    {
        handle(ctx.owner, self, ctx.provider).await
    }
}
