use anyhow::{Context, Result, anyhow};
use http::{Request, Response};
use r9k_position::HttpRequest;
use serde::de::DeserializeOwned;

pub struct Provider;

impl r9k_position::Provider for Provider {}

impl HttpRequest for Provider {
    async fn fetch<B: Sync, U: DeserializeOwned>(
        &self, request: &Request<B>,
    ) -> Result<Response<U>> {
        tracing::debug!("request: {:?}", request.uri());

        let response = sdk_http::Client::new()
            .get(request.uri())
            .headers(request.headers())
            .send()
            .map_err(|e| anyhow!(e))?;

        let data = response.body();
        if data.is_empty() {
            return Err(anyhow!("empty response"));
        }

        let body = serde_json::from_slice::<U>(data).context("deserializing response body")?;
        Response::builder().status(200).body(body).map_err(|e| anyhow!(e))
    }
}
