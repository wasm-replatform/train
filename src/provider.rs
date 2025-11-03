use std::any::Any;
use std::error::Error;
use std::future::Future;

use anyhow::Result;
use bytes::Bytes;
use dilax::provider::HttpRequest as DilaxHttpRequest;
use http::{Request, Response};
use http_body::Body;
use http_body_util::Full;
use std::pin::Pin;
use r9k_position::{HttpRequest as R9kHttpRequest, Provider as R9kProvider};

#[derive(Clone, Copy, Default)]
pub struct Provider;

impl R9kProvider for Provider {}

impl R9kHttpRequest for Provider {
    fn fetch<T>(
        &self, request: Request<T>,
    ) -> impl Future<Output = Result<Response<Bytes>>> + Send
    where
        T: Body + Any + Send,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn Error + Send + Sync + 'static>>,
    {
        async move { wasi_http::handle(request).await }
    }
}

#[derive(Clone, Default)]
pub struct WasiHttpClient;

impl DilaxHttpRequest for WasiHttpClient {
    fn fetch<'life0, 'async_trait>(
        &'life0 self, request: Request<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<Response<Vec<u8>>>> + Send + 'async_trait>>
    where
        Self: 'async_trait,
        'life0: 'async_trait,
    {
        Box::pin(async move {
            let (parts, body) = request.into_parts();
            let wasm_request = Request::from_parts(parts, Full::new(Bytes::from(body)));
            let response = wasi_http::handle(wasm_request).await?;
            let (parts, body) = response.into_parts();
            Ok(Response::from_parts(parts, body.to_vec()))
        })
    }
}
