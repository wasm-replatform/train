use anyhow::Result;
use sdk_http::axum::http::header::AUTHORIZATION;
use sdk_http::{Client, Decode};
use serde::Deserialize;

use crate::config;

#[derive(Debug, Clone, Default)]
pub struct BlockMgtApi;

impl BlockMgtApi {
    pub fn get_vehicles_by_external_ref_id(&self, external_ref_id: &str) -> Result<Vec<String>> {
        // TODO: Where do we get token?
        let bearer_token = "";
        let response = Client::new()
            .get(format!(
                "{}/allocations/trips?externalRefId={}",
                config::get_block_mgt_url(),
                external_ref_id
            ))
            .header(AUTHORIZATION, bearer_token)
            .send()?
            .json::<Response>()?;

        Ok(response.all.into_iter().map(|a| a.vehicle_label).collect())
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Response {
    all: Vec<Allocation>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Allocation {
    vehicle_label: String,
}
