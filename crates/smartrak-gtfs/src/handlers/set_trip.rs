use std::convert::Infallible;

use credibil_api::{Handler, Request, Response};
use fabric::{Config, Error, HttpRequest, Identity, Publisher, Result, StateStore};
use http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::god_mode::god_mode;

#[derive(Debug, Clone, Deserialize)]
pub struct SetTripRequest(String, String);

#[allow(clippy::infallible_try_from)]
impl TryFrom<(String, String)> for SetTripRequest {
    type Error = Infallible;

    fn try_from(value: (String, String)) -> anyhow::Result<Self, Self::Error> {
        Ok(Self(value.0, value.1))
    }
}

// const PROCESS_ID: u32 = 0;

#[derive(Debug, Clone, Serialize)]
pub struct SetTripResponse {
    pub message: String,
    pub process: u32,
}

#[allow(clippy::unused_async)]
async fn handle<P>(
    _owner: &str, request: SetTripRequest, _provider: &P,
) -> Result<Response<SetTripResponse>>
where
    P: HttpRequest + Publisher + StateStore + Identity + Config,
{
    let vehicle_id = request.0;
    let trip_id = request.1;

    let Some(god_mode) = god_mode() else {
        let response = SetTripResponse { message: "God mode not enabled".to_string(), process: 0 };
        return Ok(Response { status: StatusCode::NOT_FOUND, body: response, headers: None });
    };

    god_mode.set_vehicle_to_trip(vehicle_id, trip_id);

    Ok(SetTripResponse { message: "Ok".to_string(), process: 0 }.into())
}

impl<P> Handler<SetTripResponse, P> for Request<SetTripRequest>
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    type Error = Error;

    async fn handle(self, owner: &str, provider: &P) -> Result<Response<SetTripResponse>> {
        handle(owner, self.body, provider).await
    }
}
