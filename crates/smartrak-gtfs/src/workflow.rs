use serde::Serialize;
use tracing::{debug, info, warn};

use crate::god_mode::god_mode;
use crate::models::{EventType, PassengerCountEvent, SmartrakEvent, VehicleInfo};
use crate::processor::location::{LocationOutcome, process_location, resolve_vehicle};
use crate::processor::passenger_count::process_passenger_count;
use crate::processor::serial_data::process_serial_data;
use crate::{Provider, Result};

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
        debug!(vehicle_id, topic, "vehicle info not found, skipping event");
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
            let topic = "realtime-gtfs-vp.v1".to_string();
            messages.push(SerializedMessage::new(topic, feed.id.clone(), feed)?);
        }
        LocationOutcome::DeadReckoning(dr) => {
            let topic = "realtime-dead-reckoning.v1".to_string();
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
    let topic = topic.to_ascii_lowercase();
    let tag = vehicle.tag.as_deref().map(str::to_ascii_lowercase);

    if topic.contains("realtime-passenger-count") {
        return true;
    }

    if topic.contains("realtime-caf-avl") {
        return matches!(tag.as_deref(), Some("caf"));
    }

    if topic.contains("realtime-r9k-to-smartrak") {
        return true;
    }

    if topic.contains("realtime-smartrak-bus-avl") || topic.contains("realtime-smartrak-train-avl")
    {
        return matches!(tag.as_deref(), Some("smartrak"));
    }

    false
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{VehicleCapacity, VehicleType};

    fn vehicle_with_tag(tag: Option<&str>) -> VehicleInfo {
        VehicleInfo {
            id: "veh".to_string(),
            label: None,
            registration: None,
            capacity: VehicleCapacity::default(),
            vehicle_type: VehicleType::default(),
            tag: tag.map(std::string::ToString::to_string),
        }
    }

    #[test]
    fn allows_passenger_count_topic() {
        let vehicle = vehicle_with_tag(None);
        assert!(should_process_topic("realtime-passenger-count.v1", &vehicle));
    }

    #[test]
    fn allows_r9k_topic_without_tag() {
        let vehicle = vehicle_with_tag(None);
        assert!(should_process_topic("realtime-r9k-to-smartrak.v1", &vehicle));
    }

    #[test]
    fn requires_smartrak_tag_for_bus_and_train_topics() {
        let no_tag = vehicle_with_tag(None);
        assert!(!should_process_topic("realtime-smartrak-bus-avl.v1", &no_tag));

        let smartrak_tag = vehicle_with_tag(Some("SmArTrAk"));
        assert!(should_process_topic("realtime-smartrak-train-avl.v1", &smartrak_tag));
    }

    #[test]
    fn requires_caf_tag_for_caf_topic() {
        let non_caf = vehicle_with_tag(Some("smartrak"));
        assert!(!should_process_topic("realtime-caf-avl.v1", &non_caf));

        let caf_tag = vehicle_with_tag(Some("CAF"));
        assert!(should_process_topic("realtime-caf-avl.v1", &caf_tag));
    }

    #[test]
    fn rejects_unknown_topics() {
        let vehicle = vehicle_with_tag(Some("smartrak"));
        assert!(!should_process_topic("realtime-unknown-topic", &vehicle));
    }
}
