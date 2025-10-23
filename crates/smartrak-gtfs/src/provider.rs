use anyhow::Result;
use async_trait::async_trait;

use crate::model::fleet::{VehicleCapacity, VehicleInfo};
use crate::model::trip::{BlockInstance, TripInstance};

use http::{Request, Response};

/// Provider entry point implemented by the host application.
pub trait Provider: HttpRequest {}

/// The `HttpRequest` trait defines the behavior for fetching data from a source.
pub trait HttpRequest: Send + Sync {
    /// Make outbound HTTP request.
    fn fetch(
        &self, request: &Request<Vec<u8>>,
    ) -> impl Future<Output = Result<Response<Vec<u8>>>> + Send;
}

#[async_trait]
pub trait AdapterProvider: Send + Sync + Clone + 'static {
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
