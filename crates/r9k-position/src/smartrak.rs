//! SmarTrak event types for handling SmarTrak data.

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize, Serializer};

use crate::stops::StopInfo;

/// SmarTrak event.
/// N.B. that `@JsonProperty` descriptors are used for deserialisation only,
/// while the property name will be used when the data is serialised before
/// being published.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmarTrakEvent {
    /// The time the event was received.
    #[serde(serialize_with = "with_nanos")]
    pub received_at: DateTime<Utc>,

    /// The type of the event.
    // #[serde(rename(deserialize = "event"))]
    pub event_type: EventType,

    /// Event data containing specific details about the event.
    pub event_data: EventData,

    /// Message data for the event.
    pub message_data: MessageData,

    /// Remote data associated with the event.
    pub remote_data: RemoteData,

    /// Location data for the event.
    pub location_data: LocationData,

    /// The identifier of the company associated with the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_id: Option<u64>,

    /// Serial data associated with the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial_data: Option<SerialData>,
}

fn with_nanos<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let trunc = dt.to_rfc3339_opts(SecondsFormat::Millis, true);
    serializer.serialize_str(&trunc)
}

/// Smartrak event type.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventType {
    /// Location event.
    #[default]
    Location,

    /// Serial data event.
    SerialData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageData {
    /// Message identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<u64>,

    /// Message timestamp.
    pub timestamp: DateTime<Utc>,
}

impl Default for MessageData {
    fn default() -> Self {
        Self { message_id: None, timestamp: Utc::now() }
    }
}

/// Remote data associated with the event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteData {
    /// Remote identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_id: Option<u64>,

    /// Remote name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_name: Option<String>,

    /// External identifier.
    pub external_id: String,
}

/// Event data with specific details about the event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventData {
    /// Event code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_code: Option<u64>,

    /// Odometer reading at the time of the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub odometer: Option<u64>,

    /// Nearest address to the event location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nearest_address: Option<String>,

    /// Additional information about the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_info: Option<String>,
}

/// Location data for the event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

    /// Heading of the event location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading: Option<f64>,

    /// Kilometric point of the event location, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
