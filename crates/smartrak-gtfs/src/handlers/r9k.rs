use credibil_api::{Handler, Request, Response};
use tracing::debug;

use crate::god_mode;
use crate::location;
use crate::location::Location;
use crate::models::EventType;
pub use crate::models::SmarTrakMessage;
use crate::serial_data;
use crate::{Error, Message, Provider, Publisher, Result};

/// R9K empty response.
#[derive(Debug, Clone)]
pub struct SmarTrakResponse;

async fn handle(
    _owner: &str, request: SmarTrakMessage, provider: &impl Provider,
) -> Result<Response<SmarTrakResponse>> {
    if request.event_type == EventType::SerialData {
        let mut request = request.clone();
        if let Some(god_mode) = god_mode::god_mode() {
            god_mode.preprocess(&mut request);
        }
        serial_data::process(provider, &request).await?;
        return Ok(SmarTrakResponse.into());
    }

    if request.event_type != EventType::Location {
        debug!("unsupported request type: {:?}", request.event_type);
        return Ok(SmarTrakResponse.into());
    }

    let Some(vehicle_id) = request.vehicle_identifier() else {
        debug!("no vehicle identifier found");
        return Ok(SmarTrakResponse.into());
    };
    let vehicle_info = location::resolve_vehicle(provider, vehicle_id).await?;
    let Some(vehicle) = vehicle_info else {
        debug!("vehicle info not found for {vehicle_id}");
        return Ok(SmarTrakResponse.into());
    };

    let Some(result) = location::process(provider, &request, &vehicle).await? else {
        return Ok(SmarTrakResponse.into());
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

    Ok(SmarTrakResponse.into())
}

impl<P: Provider> Handler<SmarTrakResponse, P> for Request<SmarTrakMessage> {
    type Error = Error;

    // TODO: implement "owner"
    async fn handle(self, owner: &str, provider: &P) -> Result<Response<SmarTrakResponse>> {
        handle(owner, self.body, provider).await
    }
}
