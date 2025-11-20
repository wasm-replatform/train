use std::any::Any;
use std::env;
use std::error::Error;

use anyhow::{Context, Result};
use bytes::Bytes;
use fromenv::FromEnv;
use http::{Request, Response};
use wasi_identity::credentials::get_identity;
use wasi_keyvalue::cache;
use wasi_messaging::producer;
use wasi_messaging::types::{Client, Message};
use wit_bindgen::block_on;

// const SERVICE: &str = "train";

#[derive(Clone, Default)]
pub struct Provider {
    pub config: ConfigSettings,
}

#[derive(Debug, Clone, FromEnv)]
pub struct ConfigSettings {
    #[env(from = "ENVIRONMENT", default = "dev")]
    pub environment: String,

    #[env(from = "BLOCK_MGT_URL")]
    pub block_mgt_url: String,

    #[env(from = "CC_STATIC_URL")]
    pub cc_static_url: String,

    #[env(from = "FLEET_URL")]
    pub fleet_url: String,

    #[env(from = "GTFS_STATIC_URL")]
    pub gtfs_static_url: String,

    #[env(from = "AZURE_IDENTITY")]
    pub azure_identity: String,
}

impl Default for ConfigSettings {
    fn default() -> Self {
        // we panic here to ensure configuration is always loaded
        // i.e. guest should not start without proper configuration
        Self::from_env().finalize().expect("should load configuration")
    }
}

impl Provider {
    pub fn new() -> Self {
        Self::default()
    }
}

impl realtime::Config for Provider {
    async fn get(&self, key: &str) -> Result<String> {
        match key {
            "ENVIRONMENT" => Ok(self.config.environment.clone()),
            "BLOCK_MGT_URL" => Ok(self.config.block_mgt_url.clone()),
            "CC_STATIC_URL" => Ok(self.config.cc_static_url.clone()),
            "FLEET_URL" => Ok(self.config.fleet_url.clone()),
            "GTFS_STATIC_URL" => Ok(self.config.gtfs_static_url.clone()),
            "AZURE_IDENTITY" => Ok(self.config.azure_identity.clone()),
            _ => Err(anyhow::anyhow!("unknown config key: {key}")),
        }
    }
}

impl realtime::Publisher for Provider {
    async fn send(&self, topic: &str, message: &realtime::Message) -> Result<()> {
        tracing::debug!("sending to topic: {topic}");

        let client = Client::connect("").context("connecting to broker")?;
        let msg = Message::new(&message.payload);
        let topic = format!("{}-{topic}", self.config.environment);

        wit_bindgen::block_on(async move {
            let _ = producer::send(&client, topic, msg).await.context("sending message");
            // if let Err(e) = producer::send(&client, topic, message).await {
            //     error!(
            //         monotonic_counter.processing_errors = 1, error = %e, service = %SERVICE
            //     );
            // }
        });

        // tracing::info!(
        //     monotonic_counter.messages_sent = 1, service = %SERVICE
        // );

        Ok(())
    }
}

impl realtime::HttpRequest for Provider {
    async fn fetch<T>(&self, request: Request<T>) -> Result<Response<Bytes>>
    where
        T: http_body::Body + Any + Send,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn Error + Send + Sync + 'static>>,
    {
        tracing::debug!("request: {:?}", request.uri());
        wasi_http::handle(request).await
    }
}

impl realtime::Identity for Provider {
    async fn access_token(&self) -> Result<String> {
        let identity = self.config.azure_identity.clone();
        let identity = block_on(get_identity(identity))?;
        let access_token = block_on(async move { identity.get_token(vec![]).await })?;
        Ok(access_token.token)
    }
}

impl realtime::StateStore for Provider {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let bucket = cache::open("train_cache").context("opening cache")?;
        bucket.get(key).context("reading state from cache")
    }

    async fn set(&self, key: &str, value: &[u8], ttl_secs: Option<u64>) -> Result<Option<Vec<u8>>> {
        let bucket = cache::open("train_cache").context("opening cache")?;
        bucket.set(key, value, ttl_secs).context("reading state from cache")
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let bucket = cache::open("train_cache").context("opening cache")?;
        bucket.delete(key).context("deleting state from cache")
    }
}
