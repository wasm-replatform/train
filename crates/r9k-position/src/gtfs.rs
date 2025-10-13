use serde::{Deserialize, Serialize};

/// Trait for GTFS API operations
pub trait GtfsApi {
    fn get_stop_info(&self, stop_code: &str) -> impl Future<Output = Option<StopInfo>> + Send;
}

/// Stop information from GTFS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopInfo {
    pub stop_code: String,
    pub stop_lat: f64,
    pub stop_lon: f64,
}
