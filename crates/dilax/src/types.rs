use std::fmt;

use serde::de::{self, Unexpected, Visitor};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalizedF64(pub f64);

impl Serialize for NormalizedF64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = self.0;
        if value.is_finite() {
            let integer = value.trunc();
            if (value - integer).abs() < f64::EPSILON
                && integer >= i64::MIN as f64
                && integer <= i64::MAX as f64
            {
                return serializer.serialize_i64(integer as i64);
            }
        }

        serializer.serialize_f64(value)
    }
}

impl<'de> Deserialize<'de> for NormalizedF64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct NormalizedF64Visitor;

        impl<'de> Visitor<'de> for NormalizedF64Visitor {
            type Value = NormalizedF64;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a numeric value")
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NormalizedF64(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NormalizedF64(value as f64))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NormalizedF64(value as f64))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value
                    .parse::<f64>()
                    .map(NormalizedF64)
                    .map_err(|_| E::invalid_value(Unexpected::Str(value), &self))
            }
        }

        deserializer.deserialize_any(NormalizedF64Visitor)
    }
}

impl From<NormalizedF64> for f64 {
    fn from(value: NormalizedF64) -> Self {
        value.0
    }
}

impl From<f64> for NormalizedF64 {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Device {
    pub operator: String,
    pub site: String,
    pub model: String,
    pub serial: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Pis {
    pub line: String,
    pub stop: String,
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Waypoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sat: Option<String>,
    pub lat: String,
    pub lon: String,
    pub speed: NormalizedF64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Clock {
    pub utc: String,
    pub tz: String,
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DilaxEvent {
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
    pub speed: Option<NormalizedF64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wpt: Option<Waypoint>,
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DilaxEnrichedEvent {
    #[serde(flatten)]
    pub event: DilaxEvent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_id: Option<String>,
    pub trip_id: Option<String>,
    pub start_date: Option<String>,
    pub start_time: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct VehicleInfo {
    pub label: Option<String>,
    #[serde(rename = "vehicleId")]
    pub vehicle_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct VehicleCapacity {
    pub seating: i64,
    pub standing: Option<i64>,
    pub total: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct FleetVehicle {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub capacity: Option<VehicleCapacity>,
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct VehicleTripInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_received_timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dilax_message: Option<DilaxEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trip_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_id: Option<String>,
    pub vehicle_info: VehicleInfo,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct VehicleAllocation {
    #[serde(rename = "operationalBlockId")]
    pub operational_block_id: String,
    #[serde(rename = "tripId")]
    pub trip_id: String,
    #[serde(rename = "serviceDate")]
    pub service_date: String,
    #[serde(rename = "startTime")]
    pub start_time: String,
    #[serde(rename = "vehicleId")]
    pub vehicle_id: String,
    #[serde(rename = "vehicleLabel")]
    pub vehicle_label: String,
    #[serde(rename = "routeId")]
    pub route_id: String,
    #[serde(rename = "directionId")]
    pub direction_id: Option<u32>,
    #[serde(rename = "referenceId")]
    pub reference_id: String,
    #[serde(rename = "endTime")]
    pub end_time: String,
    pub delay: i64,
    #[serde(rename = "startDatetime")]
    pub start_datetime: i64,
    #[serde(rename = "endDatetime")]
    pub end_datetime: i64,
    #[serde(rename = "isCanceled")]
    pub is_canceled: bool,
    #[serde(rename = "isCopied")]
    pub is_copied: bool,
    pub timezone: String,
    #[serde(rename = "creationDatetime")]
    pub creation_datetime: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum StopType {
    #[serde(rename = "2")]
    TrainStop = 2,
    #[serde(rename = "3")]
    BusStop = 3,
    #[serde(rename = "4")]
    FerryStop = 4,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct StopInfo {
    #[serde(rename = "stopId")]
    pub stop_id: String,
    #[serde(rename = "stopCode")]
    pub stop_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct StopTypeEntry {
    #[serde(rename = "parent_stop_code")]
    pub parent_stop_code: Option<String>,
    #[serde(rename = "route_type")]
    pub route_type: Option<u32>,
    #[serde(rename = "stop_code")]
    pub stop_code: Option<String>,
}
