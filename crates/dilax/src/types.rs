use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Device {
    pub operator: String,
    pub site: String,
    pub model: String,
    pub serial: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Waypoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sat: Option<String>,
    pub lat: String,
    pub lon: String,
    pub speed: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Door {
    pub name: String,
    #[serde(rename = "in")]
    pub passengers_in: u32,
    #[serde(rename = "out")]
    pub passengers_out: u32,
    pub art: u32,
    pub st: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Clock {
    pub utc: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DilaxEvent {
    pub device: Device,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wpt: Option<Waypoint>,
    pub clock: Clock,
    pub doors: Vec<Door>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DilaxEnrichedEvent {
    #[serde(flatten)]
    pub event: DilaxEvent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trip_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct VehicleInfo {
    pub label: Option<String>,
    #[serde(rename = "vehicleId")]
    pub vehicle_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct VehicleCapacity {
    pub seating: i64,
    pub standing: Option<i64>,
    pub total: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct FleetVehicle {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub capacity: Option<VehicleCapacity>,
}

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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct StopInfo {
    #[serde(rename = "stopId")]
    pub stop_id: String,
    #[serde(rename = "stopCode")]
    pub stop_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct StopTypeEntry {
    #[serde(rename = "parent_stop_code")]
    pub parent_stop_code: Option<String>,
    #[serde(rename = "route_type")]
    pub route_type: u32,
    #[serde(rename = "stop_code")]
    pub stop_code: Option<String>,
}
