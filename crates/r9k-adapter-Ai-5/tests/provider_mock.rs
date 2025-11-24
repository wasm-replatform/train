use std::sync::{Arc, Mutex};
use anyhow::{Result, anyhow};
use bytes::Bytes;
use http::{Request, Response};
use http_body::Body;
use realtime::{HttpRequest, Identity, Publisher, Message};
use serde_json::json;

#[derive(Clone, Default)]
pub struct PublishedEvent { pub topic: String, pub payload: Vec<u8>, pub headers: std::collections::HashMap<String,String> }

#[derive(Clone)]
pub struct MockProvider {
    pub published: Arc<Mutex<Vec<PublishedEvent>>>,
    pub gtfs_payload: Arc<Vec<u8>>, // JSON stops array
    pub vehicles: Arc<Vec<String>>, // vehicle labels
}

impl MockProvider {
    pub fn new() -> Self {
        let stops = json!([
          {"stop_code":"133","stop_lat":-36.84448,"stop_lon":174.76915},
          {"stop_code":"134","stop_lat":-37.20299,"stop_lon":174.90990},
          {"stop_code":"9218","stop_lat":-36.99412,"stop_lon":174.8770}
        ]);
        Self { published: Arc::new(Mutex::new(Vec::new())), gtfs_payload: Arc::new(serde_json::to_vec(&stops).unwrap()), vehicles: Arc::new(vec!["EMU 001".into(), "EMU 002".into()]) }
    }
    pub fn with_vehicles(labels: Vec<String>) -> Self {
        let stops = json!([
          {"stop_code":"133","stop_lat":-36.84448,"stop_lon":174.76915},
          {"stop_code":"134","stop_lat":-37.20299,"stop_lon":174.90990},
          {"stop_code":"9218","stop_lat":-36.99412,"stop_lon":174.8770}
        ]);
        Self { published: Arc::new(Mutex::new(Vec::new())), gtfs_payload: Arc::new(serde_json::to_vec(&stops).unwrap()), vehicles: Arc::new(labels) }
    }
    pub fn take_published(&self) -> Vec<PublishedEvent> { self.published.lock().unwrap().drain(..).collect() }
}

impl HttpRequest for MockProvider {
    fn fetch<T>(&self, request: Request<T>) -> impl std::future::Future<Output = Result<Response<Bytes>>> + Send
    where
        T: Body + std::any::Any + Send,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    {
        let uri = request.uri().to_string();
        let gtfs_payload = self.gtfs_payload.clone();
        let vehicles = self.vehicles.clone();
        async move {
            if uri.contains("/gtfs/stops") {
                Ok(Response::builder().status(200).body(Bytes::from(gtfs_payload.as_ref().clone()))?)
            } else if uri.contains("/allocations/trips") {
                let envelope = json!({"all": vehicles.iter().map(|v| json!({"vehicleLabel": v})).collect::<Vec<_>>()});
                Ok(Response::builder().status(200).body(Bytes::from(serde_json::to_vec(&envelope)?))?)
            } else {
                Err(anyhow!("unexpected uri {uri}"))
            }
        }
    }
}

impl Publisher for MockProvider {
    fn send(&self, topic: &str, message: &Message) -> impl std::future::Future<Output = Result<()>> + Send {
        let published = self.published.clone();
        let topic = topic.to_string();
        let payload = message.payload.clone();
        let headers = message.headers.clone();
        async move {
            published.lock().unwrap().push(PublishedEvent { topic, payload, headers });
            Ok(())
        }
    }
}

impl Identity for MockProvider {
    fn access_token(&self) -> impl std::future::Future<Output = Result<String>> + Send {
        async move { Ok("mock-token".to_string()) }
    }
}
