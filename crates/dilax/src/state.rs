use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DilaxState {
    pub count: i64,
    pub token: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_trip_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occupancy_status: Option<String>,
}

impl Default for DilaxState {
    fn default() -> Self {
        Self { count: 0, token: 0, last_trip_id: None, occupancy_status: None }
    }
}
