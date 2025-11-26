use std::any::Any;
use std::error::Error as StdError;

use anyhow::Result;
use bytes::Bytes;
use http::{Request, Response};
use http_body::Body;

use crate::config::Config;
use realtime::{HttpRequest, Identity, Publisher, StateStore};

/// Provider capability contract for the Dilax adapter.
pub trait Provider: HttpRequest + Identity + Publisher + StateStore {}

impl<T> Provider for T where T: HttpRequest + Identity + Publisher + StateStore {}

/// Wrapper that constrains all I/O access paths and exposes configuration.
pub struct ProviderWrapper<'a, P>
where
    P: Provider + ?Sized,
{
    provider: &'a P,
    config: &'a Config,
}

impl<'a, P> ProviderWrapper<'a, P>
where
    P: Provider + ?Sized,
{
    pub fn new(provider: &'a P, config: &'a Config) -> Self {
        Self { provider, config }
    }

    pub fn config(&self) -> &'a Config {
        self.config 
    }

    pub fn provider(&self) -> &'a P {
        self.provider
    }

    pub async fn send_http<T>(&self, request: Request<T>) -> Result<Response<Bytes>>
    where
        T: Body + Any + Send,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn StdError + Send + Sync + 'static>>,
    {
        HttpRequest::fetch(self.provider, request).await
    }

    pub async fn state_get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        StateStore::get(self.provider, key).await
    }

    pub async fn state_set(
        &self, key: &str, value: &[u8], ttl_secs: Option<u64>,
    ) -> Result<Option<Vec<u8>>> {
        StateStore::set(self.provider, key, value, ttl_secs).await
    }

    pub async fn state_delete(&self, key: &str) -> Result<()> {
        StateStore::delete(self.provider, key).await
    }

    pub async fn access_token(&self) -> Result<String> {
        Identity::access_token(self.provider).await
    }
}
