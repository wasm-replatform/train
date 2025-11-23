use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub enum EventType {
    #[serde(alias = "serialData", alias = "SERIAL_DATA")]
    SerialData,
    #[serde(alias = "location", alias = "LOCATION")]
    Location,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoteData {
    pub external_id: Option<String>,
    pub remote_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageData {
    pub timestamp: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocationData {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub heading: Option<f64>,
    pub speed: Option<f64>,
    pub odometer: Option<f64>,
    #[serde(default)]
    pub gps_accuracy: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EventData {
    pub odometer: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodedSerialData {
    #[serde(alias = "tripNumber")]
    pub trip_number: Option<String>,
    #[serde(alias = "tripId")]
    pub trip_id: Option<String>,
    #[serde(alias = "lineId")]
    pub line_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialData {
    pub decoded_serial_data: Option<DecodedSerialData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartrakEvent {
    #[serde(rename = "eventType")]
    pub event_type: EventType,
    pub remote_data: Option<RemoteData>,
    pub message_data: MessageData,
    #[serde(default)]
    pub location_data: LocationData,
    #[serde(default)]
    pub event_data: EventData,
    pub serial_data: Option<SerialData>,
}

impl SmartrakEvent {
    #[must_use]
    pub fn timestamp_unix(&self) -> Option<i64> {
        DateTime::parse_from_rfc3339(&self.message_data.timestamp)
            .map(|dt| dt.with_timezone(&Utc).timestamp())
            .ok()
    }

    #[must_use]
    pub fn vehicle_identifier(&self) -> Option<&str> {
        self.remote_data
            .as_ref()
            .and_then(|remote| remote.external_id.as_deref().or(remote.remote_name.as_deref()))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PassengerCountEvent {
    pub occupancy_status: Option<String>,
    pub vehicle: PassengerVehicle,
    pub trip: PassengerTrip,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PassengerVehicle {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PassengerTrip {
    pub trip_id: String,
    pub route_id: String,
    pub start_date: String,
    pub start_time: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VehicleInfo {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub registration: Option<String>,
    #[serde(default)]
    pub capacity: VehicleCapacity,
    #[serde(default, rename = "type")]
    pub vehicle_type: VehicleType,
    #[serde(default)]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VehicleCapacity {
    pub seating: Option<i64>,
    pub standing: Option<i64>,
    pub total: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VehicleType {
    #[serde(default)]
    pub r#type: Option<String>,
}

impl VehicleType {
    #[must_use]
    pub fn is_train(&self) -> bool {
        matches!(self.r#type.as_deref(), Some("Train"))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TripInstance {
    pub trip_id: String,
    pub route_id: String,
    pub service_date: String,
    pub start_time: String,
    pub end_time: String,
    pub direction_id: Option<i32>,
    pub is_added_trip: bool,
    #[serde(default)]
    pub error: bool,
}

impl TripInstance {
    #[must_use]
    pub const fn has_error(&self) -> bool {
        self.error
    }

    #[must_use]
    pub fn to_trip_descriptor(&self) -> TripDescriptor {
        TripDescriptor {
            trip_id: self.trip_id.clone(),
            route_id: self.route_id.clone(),
            start_time: Some(self.start_time.clone()),
            start_date: Some(self.service_date.clone()),
            direction_id: self.direction_id,
            schedule_relationship: Some(if self.is_added_trip {
                TripDescriptor::ADDED.to_string()
            } else {
                TripDescriptor::SCHEDULED.to_string()
            }),
        }
    }

    #[must_use]
    pub fn remap(&self, trip_id: &str, route_id: &str) -> Self {
        let mut clone = self.clone();
        clone.trip_id = trip_id.to_string();
        clone.route_id = route_id.to_string();
        clone
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BlockInstance {
    pub trip_id: String,
    pub start_time: String,
    pub service_date: String,
    #[serde(default)]
    pub vehicle_ids: Vec<String>,
    #[serde(default)]
    pub error: bool,
}

impl BlockInstance {
    #[must_use]
    pub const fn has_error(&self) -> bool {
        self.error
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeadReckoningMessage {
    pub id: String,
    pub received_at: i64,
    pub position: PositionDr,
    pub trip: TripDescriptor,
    pub vehicle: VehicleDr,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionDr {
    pub odometer: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleDr {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FeedEntity {
    pub id: String,
    pub vehicle: Option<VehiclePosition>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VehiclePosition {
    pub position: Option<Position>,
    pub trip: Option<TripDescriptor>,
    pub vehicle: Option<VehicleDescriptor>,
    pub occupancy_status: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub bearing: Option<f64>,
    pub speed: Option<f64>,
    pub odometer: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VehicleDescriptor {
    pub id: String,
    pub label: Option<String>,
    pub license_plate: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TripDescriptor {
    pub trip_id: String,
    pub route_id: String,
    pub start_time: Option<String>,
    pub start_date: Option<String>,
    pub direction_id: Option<i32>,
    pub schedule_relationship: Option<String>,
}

impl TripDescriptor {
    pub const ADDED: &'static str = "ADDED";
    pub const SCHEDULED: &'static str = "SCHEDULED";
}
