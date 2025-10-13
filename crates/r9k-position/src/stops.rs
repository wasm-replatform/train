use std::collections::HashMap;
use std::sync::LazyLock;

use anyhow::{Context, Result, anyhow};

use crate::gtfs::StopInfo;
use crate::provider::{Key, Provider, Source, SourceData};

pub async fn stop_info(
    owner: &str, provider: &impl Provider, station: u32, is_arrival: bool,
) -> Result<Option<StopInfo>> {
    if !ACTIVE_STATIONS.contains(&station) {
        return Ok(None);
    }

    // FIXME: if station is in list above, we should always get location data
    // get station's stop code
    let Some(stop_code) = STATION_STOP.get(&station) else {
        return Ok(None);
    };

    // get stop info
    let key = Key::StopInfo((*stop_code).to_string());
    let SourceData::StopInfo(mut stop_info) =
        Source::fetch(provider, owner, &key).await.context("fetching stop info")?
    else {
        return Err(anyhow!("stop info not found for stop code {stop_code}"));
    };

    if !is_arrival {
        stop_info = DEPARTURES.get(&stop_info.stop_code).cloned().unwrap_or(stop_info);
    }

    Ok(Some(stop_info))
}

const ACTIVE_STATIONS: &[u32] = &[0, 19, 40];

static STATION_STOP: LazyLock<HashMap<u32, &str>> =
    LazyLock::new(|| HashMap::from([(0, "133"), (19, "9218"), (40, "134")]));

// Correct stops that have separate departure and arrival locations.
static DEPARTURES: LazyLock<HashMap<String, StopInfo>> = LazyLock::new(|| {
    HashMap::from([
        (
            "133".to_string(),
            StopInfo { stop_code: "133".to_string(), stop_lat: -36.84448, stop_lon: 174.76915 },
        ),
        (
            "134".to_string(),
            StopInfo { stop_code: "134".to_string(), stop_lat: -37.20299, stop_lon: 174.90990 },
        ),
        (
            "9218".to_string(),
            StopInfo { stop_code: "9218".to_string(), stop_lat: -36.99412, stop_lon: 174.8770 },
        ),
    ])
});
