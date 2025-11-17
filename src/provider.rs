use std::any::Any;
use std::env;
use std::error::Error;

use anyhow::{Context, Result};
use bytes::Bytes;
use dilax::{HttpRequest as DilaxHttpRequest, Identity as DilaxIdentity, StateStore};
use http::{Request, Response};
use r9k_adapter::{HttpRequest as R9kHttpRequest, Identity as R9kIdentity};
use r9k_connector::Publisher;
use wasi_identity::credentials::get_identity;
use wasi_keyvalue::cache;
use wasi_messaging::producer;
use wasi_messaging::types::{Client, Message};
use wit_bindgen::block_on;

use crate::ENV;


#[derive(Clone, Default)]
pub struct Provider;

impl R9kHttpRequest for Provider {
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

impl Publisher for Provider {
    async fn send(&self, topic: &str, payload: &[u8]) -> Result<()> {
        tracing::debug!("sending to topic: {}-{topic}",ENV.as_str());

        let client = Client::connect("").context("connecting to broker")?;
        let topic = format!("{}-{topic}", ENV.as_str());
        let message = Message::new(payload);

        wit_bindgen::block_on(async move {
            producer::send(&client, topic, message).await.context("sending message")
        })
    }
}

impl DilaxHttpRequest for Provider {
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

impl StateStore for Provider {
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

impl R9kIdentity for Provider {
    async fn access_token(&self) -> Result<String> {
        let identity = env::var("AZURE_IDENTITY")?;
        let identity = block_on(get_identity(identity))?;
        let access_token = block_on(async move { identity.get_token(vec![]).await })?;
        Ok(access_token.token)
    }
}

impl DilaxIdentity for Provider {
    async fn access_token(&self) -> Result<String> {
        R9kIdentity::access_token(self).await
    }
}
