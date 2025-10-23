use serde::{Deserialize, Serialize};

use super::trip::{ScheduleRelationship, TripDescriptor};

// Matches GTFS output constructed in legacy/at_smartrak_gtfs_adapter/src/processors/location.ts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedEntity {
    pub id: String,
    pub vehicle: VehiclePosition,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VehiclePosition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trip: Option<TripDescriptorPayload>,
    pub vehicle: VehicleDescriptor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occupancy_status: Option<OccupancyStatus>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearing: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub odometer: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VehicleDescriptor {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_plate: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TripDescriptorPayload {
    pub trip_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule_relationship: Option<GtfsScheduleRelationship>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GtfsScheduleRelationship {
    #[default]
    Scheduled,
    Added,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OccupancyStatus {
    Empty,
    ManySeatsAvailable,
    FewSeatsAvailable,
    StandingRoomOnly,
    CrushedStandingRoomOnly,
    Full,
    NotAcceptingPassengers,
}

impl From<&TripDescriptor> for TripDescriptorPayload {
    fn from(value: &TripDescriptor) -> Self {
        Self {
            trip_id: value.trip_id().to_string(),
            route_id: value.route_id().map(ToString::to_string),
            start_date: (!value.start_date().is_empty()).then(|| value.start_date().to_string()),
            start_time: (!value.start_time().is_empty()).then(|| value.start_time().to_string()),
            direction_id: value.direction_id(),
            schedule_relationship: value.schedule_relationship.as_ref().map(|relationship| {
                match relationship {
                    ScheduleRelationship::Scheduled => GtfsScheduleRelationship::Scheduled,
                    ScheduleRelationship::Added => GtfsScheduleRelationship::Added,
                }
            }),
        }
    }
}
