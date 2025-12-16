use serde::{Deserialize, Serialize};

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
    pub fn remap(&self, trip_id: &str, route_id: &str) -> Self {
        let mut clone = self.clone();
        clone.trip_id = trip_id.to_string();
        clone.route_id = route_id.to_string();
        clone
    }
}

impl From<&TripInstance> for TripDescriptor {
    fn from(inst: &TripInstance) -> Self {
        Self {
            trip_id: inst.trip_id.clone(),
            route_id: inst.route_id.clone(),
            start_time: Some(inst.start_time.clone()),
            start_date: Some(inst.service_date.clone()),
            direction_id: inst.direction_id,
            schedule_relationship: Some(if inst.is_added_trip {
                Self::ADDED.to_string()
            } else {
                Self::SCHEDULED.to_string()
            }),
        }
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
