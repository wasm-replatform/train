use serde::Serialize;
use tracing::{debug, info, warn};

use crate::cache::CacheStore;
use crate::config::Config;
use crate::error::Result;
use crate::god_mode::GodMode;
use crate::models::{EventType, PassengerCountEvent, SmartrakEvent, VehicleInfo};
use crate::processor::{LocationOutcome, Processor};
use crate::provider::Provider;

pub async fn process(topic: &str, payload: &[u8]) -> Result<WorkflowOutcome> {
    let config = processor.config();

    if config.topics.matches_passenger_topic(topic) {
        let event: PassengerCountEvent = serde_json::from_slice(payload)?;
        processor.process_passenger_count(&event).await?;
        debug!(topic, "processed passenger count event");
        return Ok(WorkflowOutcome::NoOp);
    }

    let mut event: SmartrakEvent = serde_json::from_slice(payload)?;

    if event.event_type == EventType::SerialData && config.god_mode_enabled {
        if let Some(god_mode) = processor.god_mode.as_ref() {
            god_mode.preprocess(&mut event);
        }
    }

    let vehicle_identifier = event.vehicle_identifier();
    let Some(vehicle_id) = vehicle_identifier else {
        warn!(topic, "skip smartrak event without vehicle identifier");
        return Ok(WorkflowOutcome::NoOp);
    };

    let vehicle_info = processor.resolve_vehicle(vehicle_id).await?;
    let Some(vehicle) = vehicle_info else {
        warn!(vehicle_id, topic, "vehicle info not found, skipping event");
        return Ok(WorkflowOutcome::NoOp);
    };

    if !should_process_topic(topic, &vehicle) {
        debug!(topic, tag = ?vehicle.tag, "vehicle tag did not match topic rules");
        return Ok(WorkflowOutcome::NoOp);
    }

    if event.event_type == EventType::SerialData {
        processor.process_serial_data(&event).await?;
        return Ok(WorkflowOutcome::NoOp);
    }

    if event.event_type != EventType::Location {
        debug!(event_type = ?event.event_type, "unsupported smartrak event type");
        return Ok(WorkflowOutcome::NoOp);
    }

    let outcome = processor.process_location(&event, &vehicle).await?;
    let Some(result) = outcome else {
        return Ok(WorkflowOutcome::NoOp);
    };

    let mut messages = Vec::new();
    match result {
        LocationOutcome::VehiclePosition(feed) => {
            if let Some(topic) = config.topics.vehicle_position.clone() {
                messages.push(SerializedMessage::new(topic, feed.id.clone(), feed)?);
            }
        }
        LocationOutcome::DeadReckoning(dr) => {
            if let Some(topic) = config.topics.dead_reckoning.clone() {
                messages.push(SerializedMessage::new(topic, dr.vehicle.id.clone(), dr)?);
            }
        }
    }

    if messages.is_empty() {
        Ok(WorkflowOutcome::NoOp)
    } else {
        info!(topic, messages = messages.len(), "smartrak location processed");
        Ok(WorkflowOutcome::Messages(messages))
    }
}

fn should_process_topic(topic: &str, vehicle: &VehicleInfo) -> bool {
    let topics = &processor.config.topics;
    let tag = vehicle.tag.as_deref().map(|value| value.to_ascii_lowercase());

    if topics.matches_passenger_topic(topic) {
        return true;
    }

    if topics.matches_caf_topic(topic) {
        return matches!(tag.as_deref(), Some("caf"));
    }

    if topics.matches_passthrough_topic(topic) {
        return true;
    }

    if topics.matches_smartrak_topic(topic) {
        return matches!(tag.as_deref(), Some("smartrak"));
    }

    // default to processing when no rules match (legacy behaviour)
    true
}

pub enum WorkflowOutcome {
    NoOp,
    Messages(Vec<SerializedMessage>),
}

pub struct SerializedMessage {
    pub topic: String,
    pub key: String,
    pub payload: Vec<u8>,
}

impl SerializedMessage {
    pub fn new<T>(topic: String, key: String, value: T) -> Result<Self>
    where
        T: Serialize,
    {
        let payload = serde_json::to_vec(&value)?;
        Ok(Self { topic, key, payload })
    }
}
