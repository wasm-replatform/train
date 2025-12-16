use credibil_api::{Handler, Request, Response};
use serde::Deserialize;

use crate::{Error, Provider, Result, SmarTrakMessage, block_mgt};

#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct CafAvlMessage(SmarTrakMessage);

/// CAF AVL response.
#[derive(Debug, Clone)]
pub struct CafAvlResponse;

async fn handle(
    owner: &str, request: CafAvlMessage, provider: &impl Provider,
) -> Result<Response<CafAvlResponse>> {
    let request = request.0;

    // verify vehicle tag is 'caf'
    let Some(vehicle_id) = request.vehicle_id() else {
        tracing::debug!("no vehicle identifier found");
        return Ok(CafAvlResponse.into());
    };
    let Some(vehicle) = block_mgt::vehicle(&vehicle_id.parse()?, provider).await? else {
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

impl<P: Provider> Handler<CafAvlResponse, P> for Request<CafAvlMessage> {
    type Error = Error;

    async fn handle(self, owner: &str, provider: &P) -> Result<Response<CafAvlResponse>> {
        handle(owner, self.body, provider).await
    }
}
