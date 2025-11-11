use std::any::Any;
use std::env;
use std::error::Error;

use anyhow::Result;
use bytes::Bytes;
use dilax::{HttpRequest as DilaxHttpRequest, StateStore};
use http::{Request, Response};
use http_body::Body;
use r9k_position::{HttpRequest as R9kHttpRequest, Identity};
use wasi_identity::credentials::get_identity;
use wit_bindgen::block_on;

#[derive(Clone, Default)]
pub struct Provider;

impl r9k_position::Provider for Provider {}

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

impl dilax::Provider for Provider {}

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

// TODO: implement state store using wasi-keyvalue

impl StateStore for Provider {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        // wasi_state_store::get(key)
        todo!()
    }

    async fn set(
        &self, key: &str, value: &[u8], expires: Option<chrono::Duration>,
    ) -> Result<Option<Vec<u8>>> {
        // wasi_state_store::set(key, value, expires)
        todo!()
    }

    async fn delete(&self, key: &str) -> Result<()> {
        // wasi_state_store::delete(key)
        todo!()
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
