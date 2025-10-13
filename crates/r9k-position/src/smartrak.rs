//! SmarTrak event types for handling SmarTrak data.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_repr::Serialize_repr;

use crate::gtfs::StopInfo;

/// SmarTrak event.
//  N.B. that @JsonProperty descriptors are used for deserialisation only,
//while the property name will be used when the data is serialised before being
// published.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "lowercase"), rename_all(serialize = "camelCase"))]
pub struct SmarTrakEvent {
    /// The time the event was received.
    pub received_at: DateTime<Utc>,

    /// The type of the event.
    #[serde(rename(deserialize = "event"))]
    pub event_type: EventType,

    /// The identifier of the company associated with the event.
    pub company_id: u64,

    /// Message data for the event.
    pub message_data: MessageData,

    /// Remote data associated with the event.
    pub remote_data: RemoteData,

    /// Event data containing specific details about the event.
    pub event_data: EventData,

    /// Location data for the event.
    pub location_data: LocationData,

    /// Serial data associated with the event.
    pub serial_data: SerialData,
}

/// Smartrak event type.
#[derive(Debug, Clone, Default, Serialize_repr, Deserialize, PartialEq, Eq)]
#[serde(rename_all(deserialize = "lowercase"))]
#[repr(u8)]
pub enum EventType {
    /// Location event.
    #[default]
    Location = 0,

    /// Serial data event.
    SerialData = 1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "lowercase"), rename_all(serialize = "camelCase"))]
pub struct MessageData {
    /// Message identifier.
    pub message_id: u64,

    /// Message timestamp.
    pub timestamp: DateTime<Utc>,
}

impl Default for MessageData {
    fn default() -> Self {
        Self { message_id: 0, timestamp: Utc::now() }
    }
}

/// Remote data associated with the event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "lowercase"), rename_all(serialize = "camelCase"))]
pub struct RemoteData {
    /// Remote identifier.
    pub remote_id: u64,

    /// Remote name.
    pub remote_name: String,

    /// External identifier.
    pub external_id: String,
}

/// Event data with specific details about the event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "lowercase"), rename_all(serialize = "camelCase"))]
pub struct EventData {
    /// Event code.
    pub event_code: u64,

    /// Odometer reading at the time of the event.
    pub odometer: u64,

    /// Nearest address to the event location.
    pub nearest_address: String,

    /// Additional information about the event.
    pub extra_info: String,
}

/// Location data for the event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "lowercase"), rename_all(serialize = "camelCase"))]
pub struct LocationData {
    /// Latitude of the event location.
    pub latitude: f64,

    /// Longitude of the event location.
    pub longitude: f64,

    /// Heading of the event location.
    pub heading: f64,

    /// Speed of the event location.
    pub speed: f64,

    /// GPS accuracy of the event location.
    pub gps_accuracy: f64,

    /// Kilometric point of the event location, if available.
    pub kilometric_point: Option<f64>,
}

impl From<StopInfo> for LocationData {
    fn from(stop: StopInfo) -> Self {
        Self { latitude: stop.stop_lat, longitude: stop.stop_lon, ..Self::default() }
    }
}

impl From<&StopInfo> for LocationData {
    fn from(stop: &StopInfo) -> Self {
        stop.clone().into()
    }
}

/// Serial data associated with the event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "lowercase"), rename_all(serialize = "camelCase"))]
pub struct SerialData {
    /// Source of the serial data.
    pub source: u64,

    /// Raw serial bytes.
    pub serial_bytes: String,

    // Decoded serial data.
    pub decoded: Option<DecodedSerialData>,
}

// Decodes bdc Serial Data, supports base64 format encoded and just string
// ex: MjQ1MDU0NDgzMTJjMzEyYzMxMzUzYTMwMzgyYzMwMmMzMjMwMzIzMTM5MzgzNTMzMmMyYzJjMzQzMzMxMzUzMDJjMzEyYzMxMzUzYTMyMzAyYzMxMmMzNDMzMzIzMzJjMzMzMzM2MzkyYzMxMzUyYzM2MmMzMjJhMzYzNg==
// ex: $PTH1,1,00:02,0,22101670,,7380,124046,2,23:45,1,2035,2037,0,0,0*6b
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "lowercase"), rename_all(serialize = "camelCase"))]
pub struct DecodedSerialData {
    pub line_id: String,
    pub trip_number: String,
    pub start_at: String,
    pub passengers_number: u32,
    pub driver_id: String,
    pub trip_active: bool,
    pub trip_ended: bool,
    pub has_trip_ended_flag: bool,
    pub tag_ons: u32,
    pub tag_offs: u32,
    pub cash_fares: u32,
}
