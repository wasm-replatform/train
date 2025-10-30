//! SmarTrak event types for handling SmarTrak data.

use jiff::Timestamp;
use serde::{Deserialize, Serialize, Serializer};

use crate::gtfs::StopInfo;

/// SmarTrak event.
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmarTrakEvent {
    /// The time the event was received.
    #[serde(serialize_with = "serialize_timestamp")]
    pub received_at: Timestamp,

    /// The type of the event.
    pub event_type: EventType,

    /// Message data for the event.
    pub message_data: MessageData,

    /// Remote data associated with the event.
    pub remote_data: RemoteData,

    /// Event data containing specific details about the event.
    pub event_data: EventData,

    /// Location data for the event.
    pub location_data: LocationData,
}

impl SmarTrakEvent {
    #[must_use]
    pub fn clone_with_new_message_timestamp(&self, timestamp: Timestamp) -> Self {
        Self { message_data: MessageData { timestamp }, ..self.clone() }
    }
}

/// Smartrak event type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventType {
    /// Location event.
    Location = 0,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageData {
    /// Message timestamp.
    #[serde(serialize_with = "serialize_timestamp")]
    pub timestamp: Timestamp,
}

/// Remote data associated with the event.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteData {
    /// External identifier.
    pub external_id: String,
}

/// Event data with specific details about the event.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
// Brackets are needed so that this gets serialized into an empty object.
#[allow(clippy::empty_structs_with_brackets)]
pub struct EventData {}

/// Location data for the event.
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationData {
    /// Latitude of the event location.
    pub latitude: f64,

    /// Longitude of the event location.
    pub longitude: f64,

    /// Speed of the event location.
    pub speed: i64,

    /// GPS accuracy of the event location.
    pub gps_accuracy: i64,
}

impl From<StopInfo> for LocationData {
    fn from(stop: StopInfo) -> Self {
        Self { latitude: stop.stop_lat, longitude: stop.stop_lon, speed: 0, gps_accuracy: 0 }
    }
}

impl From<&StopInfo> for LocationData {
    fn from(stop: &StopInfo) -> Self {
        stop.clone().into()
    }
}

// Serialization function for Timestamp with 3 decimal places to match typescript.
fn serialize_timestamp<S>(timestamp: &Timestamp, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let formatted = timestamp.strftime("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    serializer.serialize_str(&formatted)
}
