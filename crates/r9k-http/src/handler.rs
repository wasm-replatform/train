//! R9K HTTP Connector
//!
//! Listen for incoming R9K SOAP requests and forward to the r9k-adapter topic
//! for validation and transformation to SmarTrak events.

use std::env;

use anyhow::Context;
use bytes::Bytes;
use chrono::Utc;
use credibil_api::{Handler, Request, Response};
use http::header::AUTHORIZATION;
use http_body_util::Empty;
use serde::{Deserialize, Serialize};

// use crate::error::Error;
use crate::provider::{HttpRequest, Identity, Provider};
use crate::Result;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct R9kRequest;

/// R9K response for SmarTrak consumption.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct R9kResponse;

async fn handle(
    owner: &str, request: R9kRequest, provider: &impl Provider,
) -> Result<Response<R9kResponse>> {
    Ok(R9kResponse.into())
}

impl<P: Provider> Handler<R9kResponse, P> for Request<R9kRequest> {
    type Error = anyhow::Error;

    // TODO: implement "owner"
    async fn handle(self, owner: &str, provider: &P) -> Result<Response<R9kResponse>> {
        handle(owner, self.body, provider).await
    }
}
