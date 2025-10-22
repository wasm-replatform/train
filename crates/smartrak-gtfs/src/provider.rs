use anyhow::Result;
use async_trait::async_trait;

use crate::cache::CacheStore;
use crate::model::fleet::{VehicleCapacity, VehicleInfo};
use crate::model::trip::{BlockInstance, TripInstance};

#[async_trait]
pub trait AdapterProvider: Send + Sync + Clone + 'static {
    type Cache: CacheStore;

    fn cache_store(&self) -> Self::Cache;

    async fn fetch_vehicle_by_label(&self, label: &str) -> Result<Option<VehicleInfo>>;
    async fn fetch_vehicle_by_id(&self, id: &str) -> Result<Option<VehicleInfo>>;
    async fn fetch_vehicle_capacity(
        &self, vehicle_id: &str, route_id: &str,
    ) -> Result<Option<VehicleCapacity>>;
    async fn fetch_trip_instances(
        &self, trip_id: &str, service_date: &str,
    ) -> Result<Vec<TripInstance>>;
    async fn fetch_block_allocation(
        &self, vehicle_id: &str, timestamp: i64,
    ) -> Result<Option<BlockInstance>>;
}
