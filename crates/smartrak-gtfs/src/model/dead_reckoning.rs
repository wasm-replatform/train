use serde::{Deserialize, Serialize};

use super::trip::TripDescriptor;

// Mirrors message payload from legacy/at_smartrak_gtfs_adapter/src/model/dead-reckoning.ts.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeadReckoningMessage {
    pub id: String,
    pub received_at: i64,
    pub position: PositionDr,
    pub trip: TripDescriptor,
    pub vehicle: VehicleDr,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PositionDr {
    pub odometer: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VehicleDr {
    pub id: String,
}
