use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use chrono_tz::Pacific;
use credibil_api::{Body, Handler, Request, Response};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::Result;
use crate::block_mgt::{self, VehicleAllocation};
use crate::error::Error;
use crate::provider::{HttpRequest, Provider, StateStore};
use crate::trip_state::{self, VehicleInfo, VehicleTripInfo};

const DIESEL_TRAIN_PREFIX: &str = "ADL";
const LOST_THRESHOLD: Duration = Duration::hours(1);
const KEY_LOST_CONNECTION: &str = "apc:lostConnections";

const TTL_RETENTION: Duration = Duration::days(7);

#[derive(Debug, Clone)]
pub struct DetectionRequest;

#[derive(Debug, Clone)]
pub struct DetectionResponse {
    pub detections: Vec<Detection>,
}

async fn handle(
    _owner: &str, _: DetectionRequest, provider: &impl Provider,
) -> Result<Response<DetectionResponse>> {
    let detections = lost_connections(provider)
        .await
        .map_err(|e| Error::Internal(format!("detecting lost connections: {e}")))?;

    Ok(DetectionResponse { detections }.into())
}

impl<P: Provider> Handler<DetectionResponse, P> for Request<DetectionRequest> {
    type Error = Error;

    // TODO: implement "owner"
    async fn handle(self, owner: &str, provider: &P) -> Result<Response<DetectionResponse>> {
        handle(owner, self.body, provider).await
    }
}

impl Body for DetectionRequest {}

async fn lost_connections(provider: &impl Provider) -> anyhow::Result<Vec<Detection>> {
    info!("Starting Dilax lost connection job");

    let allocs = allocations(provider).await.context("refreshing Dilax allocations")?;
    let detections = detect(allocs, provider)
        .await
        .map_err(|e| Error::Internal(format!("detecting lost connections: {e}")))?;

    info!(count = detections.len(), "Completed Dilax lost connection job");

    Ok(detections)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    pub detection_time: i64,
    pub allocation: VehicleAllocation,
    pub vehicle_trip_info: VehicleTripInfo,
}

/// Refreshes cached allocations for the current service day.
///
/// # Errors
///
/// Returns an error if the block management provider or backing store cannot be queried.
async fn allocations(http: &impl HttpRequest) -> Result<Vec<VehicleAllocation>> {
    let allocations =
        block_mgt::allocations(http).await.map_err(|e| Error::Internal(e.to_string()))?;

    let now_tz = Utc::now().with_timezone(&Pacific::Auckland);
    let service_date = now_tz.format("%Y%m%d").to_string();

    let filtered: Vec<VehicleAllocation> = allocations
        .into_iter()
        .filter(|alloc| {
            alloc.service_date == service_date
                && !alloc.vehicle_id.is_empty()
                && !alloc.vehicle_label.starts_with(DIESEL_TRAIN_PREFIX)
        })
        .collect();

    info!("Caching {} allocations", filtered.len());

    Ok(filtered)
}

/// Runs the lost-connection detection workflow.
///
/// # Errors
///
/// Returns an error when Redis access or candidate deserialization fails.
async fn detect(
    allocs: Vec<VehicleAllocation>, provider: &impl Provider,
) -> anyhow::Result<Vec<Detection>> {
    info!("Starting Dilax lost connection detection pass");
    let candidates = detect_candidates(allocs, provider).await?;

    debug!(candidate_count = candidates.len(), "Dilax detection candidates evaluated");
    if candidates.is_empty() {
        info!("No Dilax lost connection candidates found");
        return Ok(Vec::new());
    }

    // fetch existing vehicle/trip mappings
    let now = Utc::now().with_timezone(&Pacific::Auckland);
    let set_key = format!("{KEY_LOST_CONNECTION}{}", now.format("%Y%m%d"));

    let mut mapping_set = (StateStore::get(provider, &set_key).await?)
        .map_or_else(SetEnvelope::default, |raw| {
            serde_json::from_slice::<SetEnvelope>(&raw).unwrap_or_default()
        });

    let now_ts = now.timestamp();

    // check whether expired
    if mapping_set.expires_at.is_some_and(|expires_at| expires_at <= now_ts) {
        StateStore::delete(provider, &set_key).await?;
        mapping_set = SetEnvelope::default();
    }

    let mut trip_vehicles = mapping_set.members;

    let mut new_detections = Vec::new();
    for c in candidates {
        let vehicle_trip =
            format!("{}|{}", c.vehicle_trip_info.vehicle_info.vehicle_id, c.allocation.trip_id);
        if trip_vehicles.contains(&vehicle_trip) {
            continue;
        }

        log_detection(&c);

        let member_key = format!("{set_key}:{vehicle_trip}");
        let bytes = serde_json::to_vec(&c)?;
        StateStore::set(provider, &member_key, &bytes, Some(TTL_RETENTION)).await?;

        trip_vehicles.push(vehicle_trip);
        new_detections.push(c);
    }

    // save vehicle/trip mappings
    let mapping_set = SetEnvelope {
        expires_at: Some(now_ts + TTL_RETENTION.num_seconds()),
        members: trip_vehicles,
    };
    let bytes = serde_json::to_vec(&mapping_set)?;
    StateStore::set(provider, &set_key, &bytes, Some(TTL_RETENTION)).await?;

    info!("{} Dilax lost connection detections recorded", new_detections.len());
    Ok(new_detections)
}

async fn detect_candidates(
    allocs: Vec<VehicleAllocation>, provider: &impl Provider,
) -> anyhow::Result<Vec<Detection>> {
    let now_ts = Utc::now().with_timezone(&Pacific::Auckland).timestamp();

    let active: Vec<VehicleAllocation> = allocs
        .into_iter()
        .filter(|alloc| alloc.start_datetime <= now_ts && alloc.end_datetime >= now_ts)
        .collect();

    debug!("{} Dilax services currently running", active.len());

    let mut detections = Vec::new();
    for alloc in active {
        let Some(info) = trip_state::get_trip(&alloc.vehicle_id, provider).await? else {
            if let Some(detection) = detect_allocation(&alloc, None) {
                detections.push(detection);
            }
            continue;
        };

        // let info: VehicleTripInfo = serde_json::from_slice(&bytes)
        //     .map_err(|err| anyhow!("deserializing vehicle trip info: {err}"))?;

        if info.trip_id.as_deref() == Some(&alloc.trip_id) {
            let last_ts =
                info.last_received_timestamp.as_deref().and_then(|v| v.parse::<i64>().ok());

            if let Some(last) = last_ts
                && connection_lost(last)
            {
                detections.push(Detection {
                    detection_time: now_ts,
                    allocation: alloc.clone(),
                    vehicle_trip_info: info,
                });
            }
        } else if let Some(detection) = detect_allocation(&alloc, Some(info)) {
            detections.push(detection);
        }
    }

    Ok(detections)
}

fn detect_allocation(
    alloc: &VehicleAllocation, existing: Option<VehicleTripInfo>,
) -> Option<Detection> {
    if !connection_lost(alloc.start_datetime) {
        return None;
    }

    let vehicle_trip_info = existing.unwrap_or_else(|| VehicleTripInfo {
        vehicle_info: VehicleInfo {
            vehicle_id: alloc.vehicle_id.clone(),
            label: Some(alloc.vehicle_label.clone()),
        },
        trip_id: Some(alloc.trip_id.clone()),
        stop_id: None,
        last_received_timestamp: None,
        dilax_message: None,
    });

    Some(Detection {
        detection_time: Utc::now().with_timezone(&Pacific::Auckland).timestamp(),
        allocation: alloc.clone(),
        vehicle_trip_info,
    })
}

fn connection_lost(timestamp: i64) -> bool {
    let now_ts = Utc::now().with_timezone(&Pacific::Auckland).timestamp();
    (timestamp + LOST_THRESHOLD.num_seconds()) <= now_ts
}

fn log_detection(detection: &Detection) {
    let vehicle_info = &detection.vehicle_trip_info.vehicle_info;
    let mut vehicle_label = detection
        .vehicle_trip_info
        .dilax_message
        .as_ref()
        .map(|msg| msg.device.site.clone())
        .map_or_else(String::new, |site| {
            let mut value = site;
            value.push_str(" - ");
            value
        });

    if let Some(label) = &vehicle_info.label {
        vehicle_label.push_str(label);
    }

    let timestamp_str = detection
        .vehicle_trip_info
        .last_received_timestamp
        .as_deref()
        .and_then(|v| v.parse::<i64>().ok())
        .map_or_else(|| String::from("Never received a Dilax message"), format_timestamp);

    let coordinates = detection
        .vehicle_trip_info
        .dilax_message
        .as_ref()
        .and_then(|msg| msg.wpt.as_ref())
        .map_or_else(
            || String::from("No GPS Position available"),
            |message| {
                let mut parts = Vec::new();
                if !message.lat.is_empty() {
                    parts.push(format!("Latitude: {}", message.lat));
                }
                if !message.lon.is_empty() {
                    parts.push(format!("Longitude: {}", message.lon));
                }
                if parts.is_empty() {
                    String::from("No GPS Position available")
                } else {
                    format!("Last Coordinates: {}", parts.join("; "))
                }
            },
        );

    let vehicle_field = format!("{vehicle_label}{}", vehicle_info.vehicle_id);

    warn!(
        vehicle = %vehicle_field,
        trip_id = %detection.allocation.trip_id,
        timestamp = %timestamp_str,
        coordinates = %coordinates,
        "Dilax connection lost"
    );
}

fn format_timestamp(timestamp: i64) -> String {
    DateTime::<Utc>::from_timestamp(timestamp, 0)
        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap())
        .with_timezone(&Pacific::Auckland)
        .format("%Y-%m-%d %H:%M:%S %Z")
        .to_string()
}

#[derive(Default, Serialize, Deserialize)]
struct SetEnvelope {
    expires_at: Option<i64>,
    members: Vec<String>,
}
