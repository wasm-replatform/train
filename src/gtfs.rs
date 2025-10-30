use anyhow::Result;
use r9k_position::gtfs::StopInfo;
use sdk_http::{Client, Decode};

use crate::config;

#[derive(Debug, Clone, Default)]
pub struct GtfsApi;

impl GtfsApi {
    pub fn get_stop_info(&self, stop_code: &str) -> Result<Option<StopInfo>> {
        let stops = Client::new()
            .get(format!(
                "{}/gtfs/stops?fields=stop_code,stop_lon,stop_lat",
                config::get_gtfs_cc_static_url()
            ))
            .send()?
            .json::<Vec<StopInfo>>()?;
        Ok(stops.into_iter().find(|stop| stop.stop_code == stop_code))
    }
}
