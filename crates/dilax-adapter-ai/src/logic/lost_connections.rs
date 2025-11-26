use anyhow::{Context, Result};
use bytes::Bytes;
use http::header::AUTHORIZATION;
use http::{Method, Request, Uri};
use http_body_util::Empty;
use serde_json::Value;
use tracing::{debug, info, warn};

use crate::config::Config;
use crate::provider::{Provider, ProviderWrapper};
use crate::types::{
    LostConnectionCandidate, UnixTimestamp, VehicleAllocation, VehicleInfo, VehicleTripInfo,
    service_date_today,
};

/// Fetch allocations for the current service date, filtering out diesel trains and missing vehicles.
///
/// # Errors
/// Returns an error when HTTP calls or JSON deserialization for block management allocations fail.
pub async fn fetch_allocations_for_today<P>(
    wrapper: &ProviderWrapper<'_, P>,
) -> Result<Vec<VehicleAllocation>>
where
    P: Provider + ?Sized,
{
    let config = wrapper.config();
    let token = wrapper.access_token().await.context("retrieving access token")?;
    let url = format!("{}/allocations", config.block_mgt_api_url);

    let request = Request::builder()
        .method(Method::GET)
        .uri(url.parse::<Uri>().context("parsing block management allocations URI")?)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .body(Empty::<Bytes>::new())
        .context("building block management allocations request")?;

    let response =
        wrapper.send_http(request).await.context("fetching block management allocations")?;

    if !response.status().is_success() {
        warn!("Error fetching allocations: status {}", response.status());
        return Ok(Vec::new());
    }

    let body = response.into_body();
    let payload: Value =
        serde_json::from_slice(&body).context("deserializing block management allocations")?;
    let allocations_value = payload.get("all").cloned().unwrap_or(Value::Array(vec![]));
    let allocations: Vec<VehicleAllocation> =
        serde_json::from_value(allocations_value).context("deserializing allocations list")?;

    info!("Loaded {} allocations", allocations.len());

    let service_date = service_date_today();
    let todays_allocations: Vec<VehicleAllocation> = allocations
        .into_iter()
        .filter(|allocation| {
            allocation.service_date.as_deref() == Some(service_date.as_str())
                && !allocation.vehicle_id.is_empty()
                && !allocation.vehicle_label.starts_with(config.diesel_train_prefix)
        })
        .collect();

    info!("Caching {} allocations for today", todays_allocations.len());
    Ok(todays_allocations)
}

/// Detect Dilax lost connections for running trips at the provided detection time.
///
/// # Errors
/// Returns an error when reading cached vehicle trip information fails or when JSON decoding fails.
pub async fn detect_lost_connections<P>(
    wrapper: &ProviderWrapper<'_, P>, config: &Config, allocations: &[VehicleAllocation],
    detection_time: UnixTimestamp,
) -> Result<Vec<LostConnectionCandidate>>
where
    P: Provider + ?Sized,
{
    let detection_epoch = detection_time.value();
    info!("Start detecting lost dilax-adapter connection with time {}", detection_epoch);

    let running_trips: Vec<VehicleAllocation> = allocations
        .iter()
        .filter(|allocation| {
            allocation.start_datetime <= detection_epoch
                && allocation.end_datetime >= detection_epoch
        })
        .cloned()
        .collect();

    debug!(
        "Following services are currently running: {}",
        serde_json::to_string(&running_trips).unwrap_or_default()
    );

    let mut candidates = Vec::new();
    for allocation in running_trips {
        if let Some(candidate) =
            detect_candidate_for_allocation(wrapper, config, detection_time, allocation).await?
        {
            candidates.push(candidate);
        }
    }

    Ok(candidates)
}

async fn detect_candidate_for_allocation<P>(
    wrapper: &ProviderWrapper<'_, P>, config: &Config, detection_time: UnixTimestamp,
    allocation: VehicleAllocation,
) -> Result<Option<LostConnectionCandidate>>
where
    P: Provider + ?Sized,
{
    let key = format!("{}:{}", config.redis.key_vehicle_trip_info, allocation.vehicle_id);
    let payload = wrapper.state_get(&key).await.context("reading vehicle trip info from state")?;

    if let Some(bytes) = payload {
        let vehicle_trip_info: VehicleTripInfo =
            serde_json::from_slice(&bytes).context("deserializing vehicle trip info")?;
        if allocation.trip_id.as_deref() == vehicle_trip_info.trip_id.as_deref() {
            if vehicle_trip_info
                .last_received_timestamp
                .as_deref()
                .and_then(parse_timestamp)
                .is_some_and(|timestamp| {
                    is_dilax_connection_lost(config, detection_time, timestamp)
                })
            {
                return Ok(Some(build_candidate(detection_time, allocation, vehicle_trip_info)));
            }
            Ok(None)
        } else {
            Ok(detect_for_allocation(config, detection_time, allocation, Some(vehicle_trip_info)))
        }
    } else {
        Ok(detect_for_allocation(config, detection_time, allocation, None))
    }
}

fn detect_for_allocation(
    config: &Config, detection_time: UnixTimestamp, allocation: VehicleAllocation,
    previous: Option<VehicleTripInfo>,
) -> Option<LostConnectionCandidate> {
    let reference_timestamp = previous
        .as_ref()
        .and_then(|info| info.last_received_timestamp.as_deref().and_then(parse_timestamp))
        .unwrap_or(allocation.start_datetime);

    if !is_dilax_connection_lost(config, detection_time, reference_timestamp) {
        return None;
    }

    debug!(
        "{} - {} lost tracking. TripStartTime: {}, Detection: {}",
        allocation.vehicle_label,
        allocation.vehicle_id,
        allocation.start_datetime,
        detection_time.value()
    );

    let vehicle_trip_info = previous.unwrap_or_else(|| VehicleTripInfo {
        vehicle_info: VehicleInfo {
            label: Some(allocation.vehicle_label.clone()),
            vehicle_id: allocation.vehicle_id.clone(),
        },
        trip_id: allocation.trip_id.clone(),
        stop_id: None,
        dilax_message: None,
        last_received_timestamp: None,
    });

    Some(build_candidate(detection_time, allocation, vehicle_trip_info))
}

fn build_candidate(
    detection_time: UnixTimestamp, allocation: VehicleAllocation,
    vehicle_trip_info: VehicleTripInfo,
) -> LostConnectionCandidate {
    LostConnectionCandidate {
        detection_time: detection_time.value(),
        allocation,
        vehicle_trip_info,
    }
}

fn parse_timestamp(value: &str) -> Option<i64> {
    value.parse::<i64>().ok()
}

fn is_dilax_connection_lost(
    config: &Config, detection_time: UnixTimestamp, timestamp: i64,
) -> bool {
    let threshold_seconds_u64 = config.connection_lost_threshold_mins.saturating_mul(60);
    let threshold_seconds =
        i64::try_from(threshold_seconds_u64.min(i64::MAX as u64)).unwrap_or(i64::MAX);
    timestamp + threshold_seconds <= detection_time.value()
}

// TODO: [MISSING_LOGIC] Redis set operations (`smembers`, `sadd`) required for deduplicating lost connection alerts are not
// available through the current `StateStore` interface. Integrate once key/value set semantics are exposed by the provider.
