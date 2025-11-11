use serde::{Deserialize, Serialize};

// TODO: document structs and fields

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DilaxMessage {
    pub dlx_vers: String,
    pub dlx_type: String,
    pub driving: bool,
    pub atstop: bool,
    pub operational: bool,
    pub distance_start: i64,
    pub trigger: String,
    pub device: Device,
    pub clock: Clock,
    pub pis: Pis,
    pub doors: Vec<Door>,
    #[serde(default)]
    pub arrival_utc: Option<String>,
    #[serde(default)]
    pub departure_utc: Option<String>,
    #[serde(default)]
    pub distance_laststop: Option<i64>,
    #[serde(default)]
    pub speed: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wpt: Option<Waypoint>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DilaxEnrichedEvent {
    #[serde(flatten)]
    pub event: DilaxMessage,

    // TODO: why are these optional?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_id: Option<String>,
    pub trip_id: Option<String>,
    pub start_date: Option<String>,
    pub start_time: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Device {
    pub operator: String,
    pub site: String,
    pub model: String,
    pub serial: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Clock {
    pub utc: String,
    pub tz: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pis {
    pub line: String,
    pub stop: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Door {
    pub name: String,
    #[serde(rename = "in")]
    pub passengers_in: u32,
    #[serde(rename = "out")]
    pub passengers_out: u32,
    pub st: String,
    pub art: u32,
    #[serde(default)]
    pub err: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Waypoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sat: Option<String>,
    pub lat: String,
    pub lon: String,
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
