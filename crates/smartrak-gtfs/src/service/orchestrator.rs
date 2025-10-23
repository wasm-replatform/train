use anyhow::Result;
use tracing::{info, warn};

use crate::config::Config;
use crate::model::events::{PassengerCountEvent, SmartrakEvent};
use crate::provider::AdapterProvider;
use crate::service::{Processor, ProducedMessage};

/// Outcome of processing an incoming Kafka record.
#[derive(Debug, Default)]
pub struct ProcessingOutcome {
    pub produced: Vec<SerializedMessage>,
    pub passenger_updates: usize,
    pub skipped: bool,
}

/// Coordinates topic routing, event decoding, and message serialization.
#[derive(Debug, Clone)]
pub struct KafkaWorkflow<P: AdapterProvider> {
    processor: Processor<P>,
    routing: MessageRouting,
    builder: MessageBuilder,
}

impl<P: AdapterProvider> KafkaWorkflow<P> {
    pub fn new(config: &Config, processor: Processor<P>) -> Self {
        let routing = MessageRouting::from_config(config);
        let builder = MessageBuilder::new(config);
        Self { processor, routing, builder }
    }

    /// Execute the workflow for a raw Kafka topic + payload.
    ///
    /// # Errors
    /// Returns deserialization or domain errors bubbled up from downstream processors.
    pub async fn handle(&self, topic: &str, payload: &[u8]) -> Result<ProcessingOutcome> {
        if self.routing.is_passenger(topic) {
            return self.handle_passenger(payload).await;
        }

        let mut outcome = ProcessingOutcome::default();
        let mut event: SmartrakEvent = serde_json::from_slice(payload)?;
        let produced = self.processor.process(topic, &mut event).await?;

        if produced.is_empty() {
            let vehicle_id = event.vehicle_id_or_label().unwrap_or_default();
            warn!(
                monotonic_counter.smartrak_skipped = 1,
                topic = %topic,
                vehicle = %vehicle_id,
                "no outputs produced"
            );
            outcome.skipped = true;
            return Ok(outcome);
        }

        outcome.produced = self.builder.serialize(produced);
        let vehicle_id = event.vehicle_id_or_label().unwrap_or_default();
        log_processed(topic, vehicle_id, outcome.produced.len());
        Ok(outcome)
    }

    async fn handle_passenger(&self, payload: &[u8]) -> Result<ProcessingOutcome> {
        let mut outcome = ProcessingOutcome::default();
        let event: PassengerCountEvent = serde_json::from_slice(payload)?;
        self.processor.process_passenger_event(event).await?;
        outcome.passenger_updates = 1;
        Ok(outcome)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializedMessage {
    pub topic: String,
    pub key: String,
    pub payload: Vec<u8>,
}

impl SerializedMessage {
    #[must_use]
    pub const fn new(topic: String, key: String, payload: Vec<u8>) -> Self {
        Self { topic, key, payload }
    }
}

#[derive(Debug, Clone)]
struct MessageRouting {
    passenger_topic: String,
}

impl MessageRouting {
    fn from_config(config: &Config) -> Self {
        Self { passenger_topic: config.topics.passenger_count_topic.clone() }
    }

    fn is_passenger(&self, topic: &str) -> bool {
        self.passenger_topic == topic
    }
}

#[derive(Debug, Clone)]
struct MessageBuilder {
    vp_topic: String,
    dr_topic: String,
}

impl MessageBuilder {
    fn new(config: &Config) -> Self {
        Self { vp_topic: config.topics.vp_topic.clone(), dr_topic: config.topics.dr_topic.clone() }
    }

    fn serialize(&self, messages: Vec<ProducedMessage>) -> Vec<SerializedMessage> {
        let mut outgoing = Vec::with_capacity(messages.len());
        for message in messages {
            match message {
                ProducedMessage::VehiclePosition { topic, payload, key } => {
                    let resolved = if topic.is_empty() { self.vp_topic.clone() } else { topic };
                    outgoing.push(SerializedMessage::new(resolved, key, payload.into_bytes()));
                }
                ProducedMessage::DeadReckoning { topic, payload, key } => {
                    let resolved = if topic.is_empty() { self.dr_topic.clone() } else { topic };
                    outgoing.push(SerializedMessage::new(resolved, key, payload.into_bytes()));
                }
            }
        }
        outgoing
    }
}

fn log_processed(topic: &str, vehicle_id: &str, produced_count: usize) {
    info!(
        monotonic_counter.smartrak_processed = produced_count as u64,
        topic = %topic,
        vehicle = %vehicle_id,
        "processed smartrak kafka event"
    );
}
