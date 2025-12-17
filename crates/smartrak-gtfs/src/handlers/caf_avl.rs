use crate::{Error, Result, SmarTrakMessage};
use common::fleet;
use credibil_api::{Handler, Request, Response};
use realtime::{Config, HttpRequest, Identity, Publisher, StateStore};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct CafAvlMessage(SmarTrakMessage);

/// CAF AVL response.
#[derive(Debug, Clone)]
pub struct CafAvlResponse;

async fn handle<P>(
    owner: &str, request: CafAvlMessage, provider: &P,
) -> Result<Response<CafAvlResponse>>
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

    Request::<SmarTrakMessage>::handle(request.into(), owner, provider).await?;

    Ok(CafAvlResponse.into())
}

impl<P> Handler<CafAvlResponse, P> for Request<CafAvlMessage>
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    type Error = Error;

    async fn handle(self, owner: &str, provider: &P) -> Result<Response<CafAvlResponse>> {
        handle(owner, self.body, provider).await
    }
}
