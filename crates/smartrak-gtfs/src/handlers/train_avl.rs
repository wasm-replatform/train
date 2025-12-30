use common::fleet;
use fabric::api::{Context, Handler, Headers, NoHeaders, Response};
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
pub struct TrainAvlResponse;

async fn handle<P>(
    owner: &str, request: TrainAvlMessage, provider: &P,
) -> Result<Response<TrainAvlResponse>>
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    let request = request.0;

    // verify vehicle tag is 'train'
    let Some(vehicle_id) = request.vehicle_id() else {
        tracing::debug!("no vehicle identifier found");
        return Ok(TrainAvlResponse.into());
    };
    let Some(vehicle) = fleet::vehicle(vehicle_id, provider).await? else {
        tracing::debug!("vehicle info not found for {vehicle_id}");
        return Ok(TrainAvlResponse.into());
    };
    if let Some(tag) = vehicle.tag.as_deref().map(str::to_lowercase)
        && tag != "smartrak"
    {
        tracing::debug!("vehicle tag {tag} did not match rules");
        return Ok(TrainAvlResponse.into());
    }

    let headers = NoHeaders;
    SmarTrakMessage::handle(request, Context { owner, provider, headers: &headers }).await?;

    Ok(TrainAvlResponse.into())
}

impl<P> Handler<P> for TrainAvlMessage
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    type Error = fabric::Error;
    type Output = TrainAvlResponse;

    async fn handle<H>(self, ctx: Context<'_, P, H>) -> Result<Response<TrainAvlResponse>>
    where
        H: Headers,
    {
        handle(ctx.owner, self, ctx.provider).await
    }
}
