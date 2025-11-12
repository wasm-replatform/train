use serde::{Deserialize, Serialize};

/// Raw Dilax payload emitted by the APC hardware on board a train.
/// The payload mirrors the legacy adapter schema so that parity can be
/// maintained against the historic Redis records.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DilaxMessage {
    /// Message format version supplied by Dilax firmware.
    pub dlx_vers: String,
    /// Message type that drives downstream routing.
    pub dlx_type: String,
    /// Whether the vehicle is currently moving.
    pub driving: bool,
    /// Whether the vehicle considers itself stopped at a platform.
    pub atstop: bool,
    /// Indicates if the APC device is operating in service mode.
    pub operational: bool,
    /// Distance travelled since the current trip began (in metres).
    pub distance_start: i64,
    /// Trigger source that caused the message to be emitted.
    pub trigger: String,
    /// Hardware metadata describing the emitting device.
    pub device: Device,
    /// Timestamp metadata in UTC plus timezone hint.
    pub clock: Clock,
    /// Passenger information system snapshot included with the message.
    pub pis: Pis,
    /// Door-level passenger counters captured for this interval.
    pub doors: Vec<Door>,
    /// Scheduled arrival timestamp if supplied by the APC device.
    #[serde(default)]
    pub arrival_utc: Option<String>,
    /// Scheduled departure timestamp if supplied by the APC device.
    #[serde(default)]
    pub departure_utc: Option<String>,
    /// Distance to the previous stop, when available.
    #[serde(default)]
    pub distance_laststop: Option<i64>,
    /// Vehicle speed reported by the hardware (km/h).
    #[serde(default)]
    pub speed: Option<u32>,
    /// Geo-spatial waypoint associated with the reading.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wpt: Option<Waypoint>,
}

/// Dilax message augmented with enrichment gathered from Auckland Transport
/// systems (vehicle stop, trip and timetable context).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DilaxEnrichedEvent {
    #[serde(flatten)]
    pub event: DilaxMessage,

    /// Optional stop identifier when a nearby train platform could be resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_id: Option<String>,
    /// Optional trip identifier when block allocation succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trip_id: Option<String>,
    /// Service date that the resolved trip belongs to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    /// Scheduled start time for the resolved trip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
}

/// Metadata describing the APC device that emitted the event.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Device {
    /// Operating company associated with the vehicle.
    pub operator: String,
    /// Site identifier reported by the hardware (used to derive vehicle label).
    pub site: String,
    /// Hardware model reference.
    pub model: String,
    /// Device serial number.
    pub serial: String,
}

/// Timestamp metadata accompanying a Dilax message.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Clock {
    /// UTC timestamp of the reading.
    pub utc: String,
    /// Timezone hint supplied by the device.
    pub tz: String,
}

/// Passenger information system context bundled with the Dilax payload.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pis {
    /// Line identifier displayed to passengers.
    pub line: String,
    /// Stop or station currently displayed.
    pub stop: String,
}

/// Door-level passenger counter values contained in a Dilax reading.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Door {
    /// Door name or label as reported by the hardware.
    pub name: String,
    #[serde(rename = "in")]
    /// Number of passengers boarding through the door within the interval.
    pub passengers_in: u32,
    #[serde(rename = "out")]
    /// Number of passengers alighting through the door within the interval.
    pub passengers_out: u32,
    /// Door status flag indicating open/closed state transitions.
    pub st: String,
    /// Automatic reset timer counter (as reported by the hardware).
    pub art: u32,
    #[serde(default)]
    /// Optional error code emitted by the door sensor.
    pub err: Option<String>,
}

/// Geo-spatial waypoint describing where the Dilax measurement occurred.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Waypoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Satellite lock quality reported by the GPS receiver.
    pub sat: Option<String>,
    /// Latitude of the waypoint.
    pub lat: String,
    /// Longitude of the waypoint.
    pub lon: String,
    /// Instantaneous speed reported (km/h).
    pub speed: u32,
}

// fn serialize_f64<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
// where
//     S: serde::Serializer,
// {
//     if value.is_nan() {
//         serializer.serialize_none()
//     } else {
//         serializer.serialize_f64(*value)
//     }
// }

// /// A wrapper for `f64` that normalizes serialization:
// /// - If the value is a whole number, it is serialized as an integer.
// /// - Otherwise, it is serialized as a float.
// ///
// /// This produces more compact and human-friendly output in formats like JSON.
// /// Deserialization accepts both integer and float representations.
// #[derive(Debug, Clone, Copy, PartialEq)]
// pub struct u32(pub f64);

// impl Eq for u32 {}

// impl Serialize for u32 {
//     #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let value = self.0;
//         if value.is_finite() {
//             let integer = value.trunc();
//             if (value - integer).abs() < f64::EPSILON
//                 && integer >= i64::MIN as f64
//                 && integer <= i64::MAX as f64
//             {
//                 return serializer.serialize_i64(integer as i64);
//             }
//         }

//         serializer.serialize_f64(value)
//     }
// }

// impl<'de> Deserialize<'de> for u32 {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         struct u32Visitor;

//         impl Visitor<'_> for u32Visitor {
//             type Value = u32;

//             fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
//                 formatter.write_str("a numeric value")
//             }

//             fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
//             where
//                 E: de::Error,
//             {
//                 Ok(u32(v))
//             }

//             #[allow(clippy::cast_precision_loss)]
//             fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
//             where
//                 E: de::Error,
//             {
//                 Ok(u32(v as f64))
//             }

//             #[allow(clippy::cast_precision_loss)]
//             fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
//             where
//                 E: de::Error,
//             {
//                 Ok(u32(v as f64))
//             }

//             fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
//             where
//                 E: de::Error,
//             {
//                 v.parse::<f64>()
//                     .map(u32)
//                     .map_err(|_parse_error| E::invalid_value(Unexpected::Str(v), &self))
//             }
//         }

//         deserializer.deserialize_any(u32Visitor)
//     }
// }

// impl From<u32> for f64 {
//     fn from(value: u32) -> Self {
//         value.0
//     }
// }

// impl From<f64> for u32 {
//     fn from(value: f64) -> Self {
//         Self(value)
//     }
// }
