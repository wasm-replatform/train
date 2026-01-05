use anyhow::Context as _;
use common::fleet;
use http::HeaderMap;
use serde::Deserialize;
use warp_sdk::api::{Context, Handler, Reply};
use warp_sdk::{Config, Decode, Error, HttpRequest, Identity, Publisher, Result, StateStore};

use crate::SmarTrakMessage;

#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct CafAvlMessage(SmarTrakMessage);

impl Decode for CafAvlMessage {
    type DecodeError = Error;
    type Encoded = Vec<u8>;

    fn decode(encoded: Self::Encoded) -> Result<Self> {
        serde_json::from_slice(&encoded).context("deserializing CafAvlMessage").map_err(Into::into)
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

    let headers = HeaderMap::default();
    SmarTrakMessage::handle(request, Context { owner, provider, headers: &headers }).await?;
    Ok(CafAvlReply.into())
}

impl<P> Handler<P> for CafAvlMessage
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    type Error = Error;
    type Output = CafAvlReply;

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<CafAvlReply>> {
        handle(ctx.owner, self, ctx.provider).await
    }
}
