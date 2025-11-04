use std::any::Any;
use std::error::Error;
use std::future::Future;

use anyhow::Result;
use bytes::Bytes;
use dilax::provider::HttpRequest as DilaxHttpRequest;
use http::{Request, Response};
use r9k_position::HttpRequest;

#[derive(Clone, Copy, Default)]
pub struct Provider;

impl r9k_position::Provider for Provider {}

impl HttpRequest for Provider {
    fn fetch<T>(&self, request: Request<T>) -> impl Future<Output = Result<Response<Bytes>>> + Send
    where
        T: http_body::Body + Any + Send,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn Error + Send + Sync + 'static>>,
    {
        async move {
            tracing::debug!("request: {:?}", request.uri());
            wasi_http::handle(request).await
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct WasiHttpClient;

impl DilaxHttpRequest for WasiHttpClient {
    fn fetch<T>(&self, request: Request<T>) -> impl Future<Output = Result<Response<Bytes>>> + Send
    where
        T: http_body::Body + Any + Send,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn Error + Send + Sync + 'static>>,
    {
        async move {
            tracing::debug!("request: {:?}", request.uri());
            wasi_http::handle(request).await
        }
    }
}
