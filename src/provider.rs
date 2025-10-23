#![allow(missing_docs)]

use std::collections::HashMap;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use r9k_position::gtfs::StopInfo;
use r9k_position::provider::{Key, Provider, Source, SourceData};
use smartrak_gtfs::model::fleet::{VehicleCapacity, VehicleInfo, VehicleType};
use smartrak_gtfs::model::trip::{BlockInstance, TripInstance};
use smartrak_gtfs::provider::AdapterProvider;

#[allow(unused_imports)]
#[derive(Debug, Clone)]
pub struct AppContext {
    stops: HashMap<&'static str, StopInfo>,
    vehicles: HashMap<&'static str, String>,
    vehicle_info_by_id: HashMap<String, VehicleInfo>,
    vehicle_info_by_label: HashMap<String, VehicleInfo>,
}

impl AppContext {
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

        let vehicle_info = VehicleInfo {
            id: "5226".to_string(),
            label: Some("AMP        5226".to_string()),
            registration: Some("CAF5226".to_string()),
            capacity: VehicleCapacity { seating: Some(230), standing: Some(143), total: Some(373) },
            vehicle_type: VehicleType { category: Some("train".to_string()) },
            tag: Some("Smartrak".to_string()),
        };

        let mut vehicle_info_by_id = HashMap::new();
        vehicle_info_by_id.insert(vehicle_info.id.clone(), vehicle_info.clone());

        let mut vehicle_info_by_label = HashMap::new();
        if let Some(label) = vehicle_info.label.clone() {
            vehicle_info_by_label.insert(label, vehicle_info.clone());
        }

        Self {
            stops,
            vehicles,
            vehicle_info_by_id,
            vehicle_info_by_label,
        }
    }
}

impl Default for AppContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for AppContext {}

#[async_trait]
impl Source for AppContext {
    async fn fetch(&self, _owner: &str, key: &Key) -> Result<SourceData> {
        match key {
            // TODO: call GTFS API
            Key::StopInfo(stop_code) => {
                let stop_info = self
                    .stops
                    .get(stop_code.as_str())
                    .cloned()
                    .ok_or_else(|| anyhow!("stop info not found for stop code {stop_code}"))?;
                Ok(SourceData::StopInfo(stop_info))
            }
            // TODO: call Block Management API
            Key::BlockMgt(train_id) => {
                let Some(vehicle) = self.vehicles.get(train_id.as_str()) else {
                    return Ok(SourceData::BlockMgt(vec![]));
                };
                Ok(SourceData::BlockMgt(vec![vehicle.clone()]))
            }
        }
    }
}

#[async_trait]
impl AdapterProvider for AppContext {
  
    async fn fetch_vehicle_by_label(&self, label: &str) -> Result<Option<VehicleInfo>> {
        Ok(self.vehicle_info_by_label.get(label).cloned())
    }

    async fn fetch_vehicle_by_id(&self, id: &str) -> Result<Option<VehicleInfo>> {
        Ok(self.vehicle_info_by_id.get(id).cloned())
    }

    async fn fetch_vehicle_capacity(
        &self, vehicle_id: &str, _route_id: &str,
    ) -> Result<Option<VehicleCapacity>> {
        Ok(self.vehicle_info_by_id.get(vehicle_id).map(|info| info.capacity.clone()))
    }

    async fn fetch_trip_instances(
        &self, _trip_id: &str, _service_date: &str,
    ) -> Result<Vec<TripInstance>> {
        Ok(vec![])
    }

    async fn fetch_block_allocation(
        &self, vehicle_id: &str, _timestamp: i64,
    ) -> Result<Option<BlockInstance>> {
        let allocation = self
            .vehicles
            .get(vehicle_id)
            .map(|id| BlockInstance { vehicle_ids: vec![id.clone()], ..BlockInstance::default() });
        Ok(allocation)
    }
}
