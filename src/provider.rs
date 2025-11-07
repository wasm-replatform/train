use std::any::Any;
use std::env;
use std::error::Error;

use anyhow::Result;
use bytes::Bytes;
use http::HeaderValue;
use http::{Request, Response};
use http_body::Body;
use r9k_position::HttpRequest;
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

        // ------------------------------------------------
        // TODO: move this trait

        // get ManagedIdentity access token
        let identity = env::var("AZURE_IDENTITY")?;
        let identity = block_on(get_identity(identity))?;
        let access_token = block_on(async move { identity.get_token(vec![]).await })?;

        // inject authorization header
        let mut request = request;
        let auth_header = HeaderValue::from_str(&format!("Bearer {}", access_token.token)).unwrap();
        request.headers_mut().insert("Authorization", auth_header);
        // ------------------------------------------------

        wasi_http::handle(request).await
    }
}
