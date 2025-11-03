use std::any::Any;
use std::error::Error;

use anyhow::Result;
use bytes::Bytes;
use http::{Request, Response};
use r9k_position::HttpRequest;

pub struct Provider;

impl r9k_position::Provider for Provider {}

impl HttpRequest for Provider {
    async fn fetch<T>(&self, request: Request<T>) -> Result<Response<Bytes>>
    where
        T: http_body::Body + Any,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn Error + Send + Sync + 'static>>,
    {
        tracing::debug!("request: {:?}", request.uri());
        wasi_http::handle(request).await
    }
}
