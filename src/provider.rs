use std::any::Any;
use std::env;
use std::error::Error;

use anyhow::Result;
use bytes::Bytes;
use http::{Request, Response};
use http_body::Body;
use r9k_position::{HttpRequest, Identity};
use wasi_identity::credentials::get_identity;
use wit_bindgen::block_on;

pub struct Provider;

impl r9k_position::Provider for Provider {}

impl HttpRequest for Provider {
    async fn fetch<T>(&self, request: Request<T>) -> Result<Response<Bytes>>
    where
        T: Body + Any,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn Error + Send + Sync + 'static>>,
    {
        tracing::debug!("request: {:?}", request.uri());
        wasi_http::handle(request).await
    }
}

impl Identity for Provider {
    async fn access_token(&self) -> Result<String> {
        let identity = env::var("AZURE_IDENTITY")?;
        let identity = block_on(get_identity(identity))?;
        let access_token = block_on(async move { identity.get_token(vec![]).await })?;
        Ok(access_token.token)
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
