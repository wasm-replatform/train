use std::fmt;

use chrono::{DateTime, TimeZone, Utc};
use serde::de::{Error as DeError, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SmartrakEvent {
    #[serde(rename = "eventType")]
    pub event_type: EventType,
    #[serde(default)]
    pub message_data: MessageData,
    #[serde(default)]
    pub remote_data: RemoteData,
    #[serde(default)]
    pub event_data: EventData,
    #[serde(default)]
    pub location_data: LocationData,
    #[serde(default)]
    pub serial_data: SerialData,
}

impl SmartrakEvent {
    /// Return the best effort vehicle identifier for the event.
    pub fn vehicle_id_or_label(&self) -> Option<&str> {
        if let Some(id) = self.remote_data.external_id.as_deref() {
            if !id.is_empty() {
                return Some(id);
            }
        }
        self.remote_data.remote_name.as_deref()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "PascalCase")]
pub enum EventType {
    #[default]
    Location,
    SerialData,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MessageData {
    #[serde(default, serialize_with = "serialize_ts", deserialize_with = "deserialize_ts")]
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(default)]
    pub message_id: Option<u64>,
}

fn deserialize_ts<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptTsVisitor;

    impl<'de> serde::de::Visitor<'de> for OptTsVisitor {
        type Value = Option<DateTime<Utc>>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a timestamp as RFC3339 string or unix seconds")
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(None)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserialize_ts(deserializer)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(Utc.timestamp_opt(value, 0).single())
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            let secs = value as i64;
            Ok(Utc.timestamp_opt(secs, 0).single())
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            let secs = value as i64;
            Ok(Utc.timestamp_opt(secs, 0).single())
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            if value.trim().is_empty() {
                return Ok(None);
            }

            if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
                return Ok(Some(dt.with_timezone(&Utc)));
            }

            if let Ok(secs) = value.parse::<i64>() {
                if let Some(dt) = Utc.timestamp_opt(secs, 0).single() {
                    return Ok(Some(dt));
                }
            }

            Err(DeError::invalid_value(Unexpected::Str(value), &self))
        }
    }

    deserializer.deserialize_option(OptTsVisitor)
}

fn serialize_ts<S>(value: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(dt) => serializer.serialize_str(&dt.to_rfc3339()),
        None => serializer.serialize_none(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoteData {
    #[serde(default)]
    pub remote_id: Option<u64>,
    #[serde(default)]
    pub remote_name: Option<String>,
    #[serde(default)]
    pub external_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EventData {
    #[serde(default)]
    pub odometer: Option<f64>,
    #[serde(default)]
    pub extra_info: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocationData {
    #[serde(default)]
    pub latitude: Option<f64>,
    #[serde(default)]
    pub longitude: Option<f64>,
    #[serde(default)]
    pub heading: Option<f64>,
    #[serde(default)]
    pub speed: Option<f64>,
    #[serde(default)]
    pub gps_accuracy: Option<f64>,
    #[serde(default)]
    pub odometer: Option<f64>,
}

impl LocationData {
    pub fn has_coordinates(&self) -> bool {
        self.latitude.is_some() && self.longitude.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SerialData {
    #[serde(default)]
    pub source: Option<u64>,
    #[serde(default)]
    pub serial_bytes: Option<String>,
    #[serde(default)]
    pub decoded_serial_data: Option<DecodedSerialData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DecodedSerialData {
    #[serde(default)]
    pub line_id: Option<String>,
    #[serde(default)]
    pub trip_number: Option<String>,
    #[serde(default)]
    pub trip_id: Option<String>,
    #[serde(default)]
    pub start_at: Option<String>,
    #[serde(default)]
    pub passengers_number: Option<u32>,
    #[serde(default)]
    pub driver_id: Option<String>,
    #[serde(default)]
    pub trip_active: Option<bool>,
    #[serde(default)]
    pub trip_ended: Option<bool>,
    #[serde(default)]
    pub has_trip_ended_flag: Option<bool>,
    #[serde(default)]
    pub tag_ons: Option<u32>,
    #[serde(default)]
    pub tag_offs: Option<u32>,
    #[serde(default)]
    pub cash_fares: Option<u32>,
}

impl DecodedSerialData {
    pub fn trip_identifier(&self) -> Option<&str> {
        self.trip_id
            .as_deref()
            .filter(|value| !value.is_empty())
            .or_else(|| self.trip_number.as_deref().filter(|value| !value.is_empty()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PassengerCountEvent {
    #[serde(default)]
    pub occupancy_status: Option<String>,
    pub vehicle: PassengerVehicle,
    pub trip: PassengerTrip,
    #[serde(default)]
    pub timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PassengerVehicle {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PassengerTrip {
    pub trip_id: String,
    pub route_id: String,
    pub start_date: String,
    pub start_time: String,
}
