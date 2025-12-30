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
pub struct CafAvlResponse;

async fn handle<P>(
    owner: &str, request: CafAvlMessage, provider: &P,
) -> Result<Reply<CafAvlResponse>>
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    let request = request.0;

    // verify vehicle tag is 'caf'
    let Some(vehicle_id) = request.vehicle_id() else {
        tracing::debug!("no vehicle identifier found");
        return Ok(CafAvlResponse.into());
    };
    let Some(vehicle) = fleet::vehicle(vehicle_id, provider).await? else {
        tracing::debug!("vehicle info not found for {vehicle_id}");
        return Ok(CafAvlResponse.into());
    };
    if let Some(tag) = vehicle.tag.as_deref().map(str::to_lowercase)
        && tag != "caf"
    {
        tracing::debug!("vehicle tag {tag} did not match rules");
        return Ok(CafAvlResponse.into());
    }

    let headers = NoHeaders;
    SmarTrakMessage::handle(request, Context { owner, provider, headers: &headers }).await?;
    Ok(CafAvlResponse.into())
}

impl<P> Handler<P> for CafAvlMessage
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    type Error = Error;
    type Output = CafAvlResponse;

    async fn handle<H>(self, ctx: Context<'_, P, H>) -> Result<Reply<CafAvlResponse>>
    where
        H: Headers,
    {
        handle(ctx.owner, self, ctx.provider).await
    }
}
