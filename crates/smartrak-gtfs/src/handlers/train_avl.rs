use credibil_api::{Handler, Request, Response};
use serde::Deserialize;
use tracing::debug;

use crate::god_mode;
use crate::location;
use crate::location::Location;
use crate::models::{EventType, SmarTrakMessage};
use crate::serial_data;
use crate::{Error, Message, Provider, Publisher, Result};

#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct TrainAvlMessage(SmarTrakMessage);

/// R9K empty response.
#[derive(Debug, Clone)]
pub struct TrainAvlResponse;

async fn handle(
    _owner: &str, request: TrainAvlMessage, provider: &impl Provider,
) -> Result<Response<TrainAvlResponse>> {
    let request = request.0;

    // process serial data
    if request.event_type == EventType::SerialData {
        let mut request = request.clone();
        if let Some(god_mode) = god_mode::god_mode() {
            god_mode.preprocess(&mut request);
        }
        serial_data::process(provider, &request).await?;
        return Ok(TrainAvlResponse.into());
    }

    // verifications
    if request.event_type != EventType::Location {
        debug!("unsupported request type: {:?}", request.event_type);
        return Ok(TrainAvlResponse.into());
    }
    let Some(vehicle_id) = request.vehicle_identifier() else {
        debug!("no vehicle identifier found");
        return Ok(TrainAvlResponse.into());
    };

    // resolve vehicle info
    let Some(vehicle) = location::resolve_vehicle(provider, vehicle_id).await? else {
        debug!("vehicle info not found for {vehicle_id}");
        return Ok(TrainAvlResponse.into());
    };

    if let Some(tag) = vehicle.tag.as_deref().map(str::to_lowercase)
        && tag != "smartrak"
    {
        debug!("vehicle tag {tag} did not match rules");
        return Ok(TrainAvlResponse.into());
    }

    let Some(result) = location::process(provider, &request, &vehicle).await? else {
        return Ok(TrainAvlResponse.into());
    };

    let (payload, key, topic) = match result {
        Location::VehiclePosition(feed) => {
            (serde_json::to_vec(&feed)?, feed.id, "realtime-gtfs-vp.v1")
        }
        Location::DeadReckoning(dr) => {
            (serde_json::to_vec(&dr)?, dr.id, "realtime-dead-reckoning.v1")
        }
    };

    // publish
    let mut message = Message::new(&payload);
    message.headers.insert("key".to_string(), key.clone());
    Publisher::send(provider, topic, &message).await?;

    Ok(TrainAvlResponse.into())
}

impl<P: Provider> Handler<TrainAvlResponse, P> for Request<TrainAvlMessage> {
    type Error = Error;

    // TODO: implement "owner"
    async fn handle(self, owner: &str, provider: &P) -> Result<Response<TrainAvlResponse>> {
        handle(owner, self.body, provider).await
    }
}
