use credibil_api::{Handler, Request, Response};
use serde::Serialize;
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

    let mut messages = Vec::new();
    match result {
        Location::VehiclePosition(feed) => {
            let topic = "realtime-gtfs-vp.v1".to_string();
            messages.push(Serialized::new(topic, feed.id.clone(), feed)?);
        }
        Location::DeadReckoning(dr) => {
            let topic = "realtime-dead-reckoning.v1".to_string();
            messages.push(Serialized::new(topic, dr.vehicle.id.clone(), dr)?);
        }
    }

    // publish messages to topics
    for msg in messages {
        let mut message = Message::new(&msg.payload);
        message.headers.insert("key".to_string(), msg.key.clone());
        Publisher::send(provider, &msg.topic, &message).await?;
    }

    Ok(SmarTrakResponse.into())
}

impl<P: Provider> Handler<SmarTrakResponse, P> for Request<SmarTrakMessage> {
    type Error = Error;

    // TODO: implement "owner"
    async fn handle(self, owner: &str, provider: &P) -> Result<Response<SmarTrakResponse>> {
        handle(owner, self.body, provider).await
    }
}

pub struct Serialized {
    pub topic: String,
    pub key: String,
    pub payload: Vec<u8>,
}

impl Serialized {
    /// Creates a serialized message ready for publication.
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be serialized to JSON.
    pub fn new<T>(topic: String, key: String, value: T) -> Result<Self>
    where
        T: Serialize,
    {
        let payload = serde_json::to_vec(&value)?;
        Ok(Self { topic, key, payload })
    }
}
