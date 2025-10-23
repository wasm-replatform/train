use serde::{Deserialize, Serialize};

// Mirrors TripInstance model from legacy/at_smartrak_gtfs_adapter/src/apis/trip-mgt.ts.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TripInstance {
    pub trip_id: String,
    #[serde(default)]
    pub route_id: Option<String>,
    #[serde(default)]
    pub service_date: String,
    #[serde(default)]
    pub start_time: String,
    #[serde(default)]
    pub end_time: Option<String>,
    #[serde(default)]
    pub direction_id: Option<i32>,
    #[serde(default)]
    pub is_added_trip: Option<bool>,
    #[serde(default)]
    pub error: bool,
}

impl TripInstance {
    pub fn error_marker() -> Self {
        Self { error: true, ..Self::default() }
    }

    pub fn has_error(&self) -> bool {
        self.error
    }

    pub fn service_date(&self) -> Option<&str> {
        (!self.service_date.is_empty()).then(|| self.service_date.as_str())
    }

    pub fn start_time(&self) -> Option<&str> {
        (!self.start_time.is_empty()).then(|| self.start_time.as_str())
    }

    pub fn end_time(&self) -> Option<&str> {
        self.end_time.as_deref().filter(|value| !value.is_empty())
    }

    pub fn remap(&self, trip_id: &str, route_id: &str) -> Self {
        Self { trip_id: trip_id.to_string(), route_id: Some(route_id.to_string()), ..self.clone() }
    }

    pub fn to_trip_descriptor(&self) -> TripDescriptor {
        TripDescriptor {
            trip_id: self.trip_id.clone(),
            route_id: self.route_id.clone(),
            start_date: Some(self.service_date.clone()),
            start_time: Some(self.start_time.clone()),
            direction_id: self.direction_id,
            schedule_relationship: if self.is_added_trip.unwrap_or(false) {
                Some(ScheduleRelationship::Added)
            } else {
                Some(ScheduleRelationship::Scheduled)
            },
        }
    }
}

// Mirrors BlockInstance model from legacy/at_smartrak_gtfs_adapter/src/apis/block-mgt.ts.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BlockInstance {
    #[serde(default)]
    pub trip_id: String,
    #[serde(default)]
    pub start_time: String,
    #[serde(default)]
    pub service_date: String,
    #[serde(default)]
    pub vehicle_ids: Vec<String>,
    #[serde(default)]
    pub error: bool,
}

impl BlockInstance {
    pub fn has_error(&self) -> bool {
        self.error
    }
}

// Aligns with TripDescriptor usage in legacy/at_smartrak_gtfs_adapter/src/processors/location.ts.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TripDescriptor {
    pub trip_id: String,
    #[serde(default)]
    pub route_id: Option<String>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub direction_id: Option<i32>,
    #[serde(default)]
    pub schedule_relationship: Option<ScheduleRelationship>,
}

impl TripDescriptor {
    pub fn trip_id(&self) -> &str {
        &self.trip_id
    }

    pub fn route_id(&self) -> Option<&str> {
        self.route_id.as_deref()
    }

    pub fn start_date(&self) -> &str {
        self.start_date.as_deref().unwrap_or("")
    }

    pub fn start_time(&self) -> &str {
        self.start_time.as_deref().unwrap_or("")
    }

    pub fn direction_id(&self) -> Option<i32> {
        self.direction_id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScheduleRelationship {
    #[default]
    Scheduled,
    Added,
}
