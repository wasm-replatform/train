#![allow(missing_docs)]

use std::collections::HashMap;

use anyhow::{Result, anyhow};
use r9k_position::gtfs::StopInfo;
use r9k_position::provider::{Key, Provider, Source, SourceData};

#[derive(Clone, Default)]
pub struct AppContext {
    stops: HashMap<&'static str, StopInfo>,
    vehicles: HashMap<&'static str, String>,
}

impl AppContext {
    #[must_use]
    pub fn new() -> Self {
        let stops = HashMap::from([
            (
                "133",
                StopInfo { stop_code: "133".to_string(), stop_lat: -36.12345, stop_lon: 174.12345 },
            ),
            (
                "134",
                StopInfo { stop_code: "134".to_string(), stop_lat: -36.20299, stop_lon: 174.76915 },
            ),
            (
                "9218",
                StopInfo { stop_code: "9218".to_string(), stop_lat: -36.567, stop_lon: 174.44444 },
            ),
        ]);
        let vehicles = HashMap::from([("5226", "vehicle 1".to_string())]);

        Self { stops, vehicles }
    }
}

impl Provider for AppContext {}

impl Source for AppContext {
    async fn fetch(&self, _owner: &str, key: &Key) -> Result<SourceData> {
        match key {
            Key::StopInfo(stop_code) => {
                let stop_info = self
                    .stops
                    .get(stop_code.as_str())
                    .cloned()
                    .ok_or_else(|| anyhow!("stop info not found for stop code {stop_code}"))?;
                Ok(SourceData::StopInfo(stop_info))
            }
            Key::BlockMgt(train_id) => {
                let Some(vehicle) = self.vehicles.get(train_id.as_str()) else {
                    return Ok(SourceData::BlockMgt(vec![]));
                };
                Ok(SourceData::BlockMgt(vec![vehicle.clone()]))
            }
        }
    }
}
