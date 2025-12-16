use credibil_api::{Handler, Request, Response};
use serde::Deserialize;

use crate::{Error, Provider, Result, SmarTrakMessage, fleet};

#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct TrainAvlMessage(SmarTrakMessage);

/// Train AVL response.
#[derive(Debug, Clone)]
pub struct TrainAvlResponse;

async fn handle(
    owner: &str, request: TrainAvlMessage, provider: &impl Provider,
) -> Result<Response<TrainAvlResponse>> {
    let request = request.0;

    // verify vehicle tag is 'train'
    let Some(vehicle_id) = request.vehicle_id() else {
        tracing::debug!("no vehicle identifier found");
        return Ok(TrainAvlResponse.into());
    };
    let Some(vehicle) = fleet::get_vehicle(vehicle_id, provider).await? else {
        tracing::debug!("vehicle info not found for {vehicle_id}");
        return Ok(TrainAvlResponse.into());
    };
    if let Some(tag) = vehicle.tag.as_deref().map(str::to_lowercase)
        && tag != "smartrak"
    {
        tracing::debug!("vehicle tag {tag} did not match rules");
        return Ok(TrainAvlResponse.into());
    }

    Request::<SmarTrakMessage>::handle(request.into(), owner, provider).await?;

    Ok(TrainAvlResponse.into())
}

impl<P: Provider> Handler<TrainAvlResponse, P> for Request<TrainAvlMessage> {
    type Error = Error;

    async fn handle(self, owner: &str, provider: &P) -> Result<Response<TrainAvlResponse>> {
        handle(owner, self.body, provider).await
    }
}
