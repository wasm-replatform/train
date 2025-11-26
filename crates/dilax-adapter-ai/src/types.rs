use std::fmt;
use std::ops::Deref;

use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Pacific::Auckland;
use serde::{Deserialize, Serialize};

macro_rules! string_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                self.as_str()
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }
    };
}

string_newtype!(VehicleId);
string_newtype!(VehicleLabel);
string_newtype!(TripId);
string_newtype!(StopId);
string_newtype!(StopCode);
string_newtype!(ServiceDate);
string_newtype!(ServiceTime);
string_newtype!(RedisKey);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageToken(u64);

impl MessageToken {
    pub const ZERO: Self = Self(0);

    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn value(self) -> u64 {
        self.0
    }
}

impl From<u64> for MessageToken {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PassengerCount(u32);

impl PassengerCount {
    pub const ZERO: Self = Self(0);

    pub fn new(value: u32) -> Self {
        Self(value)
    }

    pub fn saturating_sub(self, rhs: u32) -> Self {
        Self(self.0.saturating_sub(rhs))
    }

    pub fn saturating_add(self, rhs: u32) -> Self {
        Self(self.0.saturating_add(rhs))
    }

    pub fn value(self) -> u32 {
        self.0
    }
}

impl From<u32> for PassengerCount {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

impl From<PassengerCount> for u32 {
    fn from(value: PassengerCount) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OccupancyStatus {
    Empty,
    ManySeatsAvailable,
    FewSeatsAvailable,
    StandingRoomOnly,
    CrushedStandingRoomOnly,
    Full,
    NotAcceptingPassengers,
}

impl OccupancyStatus {
    pub fn code(self) -> OccupancyStatusCode {
        let numeric = match self {
            OccupancyStatus::Empty => 0,
            OccupancyStatus::ManySeatsAvailable => 1,
            OccupancyStatus::FewSeatsAvailable => 2,
            OccupancyStatus::StandingRoomOnly => 3,
            OccupancyStatus::CrushedStandingRoomOnly => 4,
            OccupancyStatus::Full => 5,
            OccupancyStatus::NotAcceptingPassengers => 6,
        };
        OccupancyStatusCode::new(numeric)
    }

    pub fn from_code(code: OccupancyStatusCode) -> Option<Self> {
        match code.value() {
            0 => Some(Self::Empty),
            1 => Some(Self::ManySeatsAvailable),
            2 => Some(Self::FewSeatsAvailable),
            3 => Some(Self::StandingRoomOnly),
            4 => Some(Self::CrushedStandingRoomOnly),
            5 => Some(Self::Full),
            6 => Some(Self::NotAcceptingPassengers),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct OccupancyStatusCode(u8);

impl OccupancyStatusCode {
    pub fn new(value: u8) -> Self {
        Self(value)
    }

    pub fn value(self) -> u8 {
        self.0
    }
}

impl fmt::Debug for OccupancyStatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for OccupancyStatusCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for OccupancyStatusCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let numeric = value.parse::<u8>().map_err(serde::de::Error::custom)?;
        Ok(Self(numeric))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub operator: String,
    pub site: String,
    pub model: String,
    pub serial: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeoWaypoint {
    pub sat: String,
    pub lat: String,
    pub lon: String,
    pub speed: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clock {
    pub utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoorEvent {
    pub name: String,
    #[serde(default)]
    pub r#in: u32,
    #[serde(default)]
    pub out: u32,
    pub art: Option<i64>,
    pub st: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DilaxEvent {
    pub device: Device,
    pub clock: Clock,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wpt: Option<GeoWaypoint>,
    #[serde(default)]
    pub doors: Vec<DoorEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DilaxEventEnriched {
    #[serde(flatten)]
    pub event: DilaxEvent,
    #[serde(rename = "stop_id", default, skip_serializing_if = "Option::is_none")]
    pub stop_id: Option<StopId>,
    #[serde(rename = "trip_id", default, skip_serializing_if = "Option::is_none")]
    pub trip_id: Option<TripId>,
    #[serde(rename = "start_date", default, skip_serializing_if = "Option::is_none")]
    pub start_date: Option<ServiceDate>,
    #[serde(rename = "start_time", default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<ServiceTime>,
}

impl DilaxEventEnriched {
    pub fn new(event: DilaxEvent) -> Self {
        Self { event, stop_id: None, trip_id: None, start_date: None, start_time: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(rename = "vehicleId")]
    pub vehicle_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleTripInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_received_timestamp: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dilax_message: Option<DilaxEvent>,
    pub trip_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_id: Option<String>,
    pub vehicle_info: VehicleInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DilaxStateRecord {
    pub count: PassengerCount,
    pub token: MessageToken,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_trip_id: Option<TripId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occupancy_status: Option<OccupancyStatusCode>,
}

impl Default for DilaxStateRecord {
    fn default() -> Self {
        Self {
            count: PassengerCount::ZERO,
            token: MessageToken::ZERO,
            last_trip_id: None,
            occupancy_status: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DilaxState {
    pub count: PassengerCount,
    pub token: MessageToken,
    pub last_trip_id: Option<TripId>,
    pub occupancy_status: Option<OccupancyStatus>,
}

impl From<DilaxStateRecord> for DilaxState {
    fn from(record: DilaxStateRecord) -> Self {
        let occupancy_status = record.occupancy_status.and_then(OccupancyStatus::from_code);

        Self {
            count: record.count,
            token: record.token,
            last_trip_id: record.last_trip_id,
            occupancy_status,
        }
    }
}

impl From<&DilaxState> for DilaxStateRecord {
    fn from(state: &DilaxState) -> Self {
        Self {
            count: state.count,
            token: state.token,
            last_trip_id: state.last_trip_id.clone(),
            occupancy_status: state.occupancy_status.map(OccupancyStatus::code),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FleetVehicleCapacity {
    pub seating: Option<u32>,
    pub standing: Option<u32>,
    pub total: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FleetVehicleResponse {
    pub id: String,
    #[serde(default)]
    pub capacity: Option<FleetVehicleCapacity>,
    #[serde(default)]
    pub r#type: Option<FleetVehicleType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FleetVehicleType {
    pub r#type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockAllocationResponse {
    #[serde(default)]
    pub current: Vec<VehicleAllocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleAllocation {
    pub trip_id: Option<String>,
    pub service_date: Option<String>,
    pub start_time: Option<String>,
    pub start_datetime: i64,
    pub end_datetime: i64,
    pub vehicle_label: String,
    pub vehicle_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopInfoRecord {
    pub stop_id: String,
    pub stop_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainStopTypeRecord {
    pub parent_stop_code: Option<String>,
    pub route_type: Option<i32>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LostConnectionCandidate {
    pub detection_time: i64,
    pub allocation: VehicleAllocation,
    pub vehicle_trip_info: VehicleTripInfo,
}

impl LostConnectionCandidate {
    pub fn detection_date_time(&self) -> DateTime<Utc> {
        Utc.timestamp_opt(self.detection_time, 0).single().unwrap_or_else(Utc::now)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnixTimestamp(pub i64);

impl UnixTimestamp {
    pub fn now() -> Self {
        Self(Utc::now().timestamp())
    }

    pub fn value(self) -> i64 {
        self.0
    }

    pub fn add_minutes(self, minutes: i64) -> Self {
        Self(self.0 + minutes * 60)
    }

    pub fn is_before(self, other: UnixTimestamp) -> bool {
        self.0 < other.0
    }
}

impl From<i64> for UnixTimestamp {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

pub fn service_date_today() -> ServiceDate {
    let today = Auckland.from_utc_datetime(&Utc::now().naive_utc());
    ServiceDate::new(today.format("%Y%m%d").to_string())
}
