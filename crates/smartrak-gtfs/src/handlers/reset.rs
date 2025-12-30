use std::convert::Infallible;

use fabric::api::{Handler, Response};
use fabric::{Config, Error, HttpRequest, Identity, Publisher, Result, StateStore, bad_request};
use serde::{Deserialize, Serialize};

use crate::god_mode::god_mode;

#[derive(Debug, Clone, Deserialize)]
pub struct ResetRequest(String);

#[allow(clippy::infallible_try_from)]
impl TryFrom<String> for ResetRequest {
    type Error = Infallible;

    fn try_from(value: String) -> anyhow::Result<Self, Self::Error> {
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResetResponse {
    pub message: String,
    pub process: u32,
}

#[allow(clippy::unused_async)]
async fn handle<P>(
    _owner: &str, request: ResetRequest, _provider: &P,
) -> Result<Response<ResetResponse>>
where
    P: HttpRequest + Publisher + StateStore + Identity + Config,
{
    let vehicle_id = request.0;

    let Some(god_mode) = god_mode() else {
        return Err(bad_request!("God mode not enabled"));
    };

    if vehicle_id == "all" {
        god_mode.reset_all();
    } else {
        god_mode.reset_vehicle(&vehicle_id);
    }

    Ok(ResetResponse { message: "Ok".to_string(), process: 0 }.into())
}

impl<P> Handler<P> for ResetRequest
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    type Output = ResetResponse;
    type Error = Error;

    async fn handle(self, owner: &str, provider: &P) -> Result<Response<ResetResponse>> {
        handle(owner, self, provider).await
    }
}
