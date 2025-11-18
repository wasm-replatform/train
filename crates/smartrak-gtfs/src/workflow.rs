use serde::Serialize;
use tracing::{debug, info, warn};

use crate::Provider;
use crate::error::Result;
use crate::god_mode::god_mode;
use crate::models::{EventType, PassengerCountEvent, SmartrakEvent, VehicleInfo};
use crate::processor::location::{LocationOutcome, process_location, resolve_vehicle};
use crate::processor::passenger_count::process_passenger_count;
use crate::processor::serial_data::process_serial_data;

/// Processes a Smartrak Kafka payload and emits outbound messages when applicable.
///
/// # Errors
///
/// Returns an error when the incoming payload cannot be parsed or when domain logic
/// encounters an unrecoverable condition.
pub async fn process(
    provider: &impl Provider, topic: &str, payload: &[u8],
) -> Result<WorkflowOutcome> {
    if topic.contains("realtime-passenger-count") {
        let event: PassengerCountEvent = serde_json::from_slice(payload)?;
        process_passenger_count(provider, &event).await?;
        debug!(topic, "processed passenger count event");
        return Ok(WorkflowOutcome::NoOp);
    }

    let mut event: SmartrakEvent = serde_json::from_slice(payload)?;

    if event.event_type == EventType::SerialData {
        if let Some(god_mode) = god_mode() {
            god_mode.preprocess(&mut event);
        }

        process_serial_data(provider, &event).await?;
        return Ok(WorkflowOutcome::NoOp);
    }

    if event.event_type != EventType::Location {
        debug!(event_type = ?event.event_type, "unsupported smartrak event type");
        return Ok(WorkflowOutcome::NoOp);
    }

    let Some(vehicle_id) = event.vehicle_identifier() else {
        warn!(topic, "skip smartrak event without vehicle identifier");
        return Ok(WorkflowOutcome::NoOp);
    };

    let vehicle_info = resolve_vehicle(provider, vehicle_id).await?;
    let Some(vehicle) = vehicle_info else {
        warn!(vehicle_id, topic, "vehicle info not found, skipping event");
        return Ok(WorkflowOutcome::NoOp);
    };

    if !should_process_topic(topic, &vehicle) {
        debug!(topic, tag = ?vehicle.tag, "vehicle tag did not match topic rules");
        return Ok(WorkflowOutcome::NoOp);
    }

    let outcome = process_location(provider, &event, &vehicle).await?;
    let Some(result) = outcome else {
        return Ok(WorkflowOutcome::NoOp);
    };

    let mut messages = Vec::new();
    match result {
        LocationOutcome::VehiclePosition(feed) => {
            let topic = "realtime-gtfs-vp".to_string();
            messages.push(SerializedMessage::new(topic, feed.id.clone(), feed)?);
        }
        LocationOutcome::DeadReckoning(dr) => {
            let topic = "realtime-dead-reckoning".to_string();
            messages.push(SerializedMessage::new(topic, dr.vehicle.id.clone(), dr)?);
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
    let tag = vehicle.tag.as_deref().map(str::to_ascii_lowercase);

    if topic.contains("realtime-passenger-count") {
        return true;
    }

    if topic.contains("realtime-caf-avl") {
        return matches!(tag.as_deref(), Some("caf"));
    }

    if "realtime-smartrak-bus-avl,realtime-smartrak-train-avl,realtime-r9k-to-smartrak"
        .contains(topic)
    {
        return true;
    }

    if "realtime-smartrak-bus-avl,realtime-smartrak-train-avl,realtime-r9k-to-smartrak"
        .contains(topic)
    {
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
