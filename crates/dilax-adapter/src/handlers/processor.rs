use anyhow::Context;
use credibil_api::{Handler, Request, Response};
use tracing::{debug, info};

use crate::block_mgt::{self, FleetVehicle};
use crate::error::Error;
use crate::gtfs::{self, StopType, StopTypeEntry};
use crate::trip_state::{VehicleInfo, VehicleTripInfo};
use crate::types::{DilaxMessage, EnrichedEvent};
use crate::{HttpRequest, Message, Provider, Publisher, Result, trip_state};

const STOP_SEARCH_DISTANCE_METERS: u32 = 150;
const VEHICLE_LABEL_WIDTH: usize = 14;
const DILAX_ENRICHED_TOPIC: &str = "realtime-dilax-adapter-apc-enriched.v1";

/// Dilax empty response.
#[derive(Debug, Clone)]
pub struct DilaxResponse;

async fn handle(
    _owner: &str, request: DilaxMessage, provider: &impl Provider,
) -> Result<Response<DilaxResponse>> {
    process(request, provider).await?;
    Ok(DilaxResponse.into())
}

impl<P: Provider> Handler<DilaxResponse, P> for Request<DilaxMessage> {
    type Error = Error;

    // TODO: implement "owner"
    async fn handle(self, owner: &str, provider: &P) -> Result<Response<DilaxResponse>> {
        handle(owner, self.body, provider).await
    }
}

/// Enriches a Dilax event with vehicle, stop, trip, and occupancy information.
///
/// # Errors
///
/// Returns an error when one of the providers or the key-value store reports a failure
/// while augmenting the incoming Dilax event.
pub async fn process(event: DilaxMessage, provider: &impl Provider) -> Result<()> {
    let vehicle_label = vehicle_label(&event).ok_or_else(|| {
        Error::ProcessingError(format!("vehicle label missing for device {:?}", event.device))
    })?;

    let vehicle = block_mgt::vehicle(&vehicle_label, provider)
        .await
        .map_err(|err| {
            Error::ProcessingError(format!(
                "failed to resolve vehicle for label {vehicle_label}: {err}"
            ))
        })?
        .ok_or_else(|| {
            Error::ProcessingError(format!("vehicle not found for label {vehicle_label}"))
        })?;

    let (vehicle_seating, vehicle_total) = vehicle_capacity(&vehicle).ok_or_else(|| {
        Error::ProcessingError(format!("vehicle {} lacks capacity information", vehicle.id))
    })?;
    let vehicle_id = vehicle.id.clone();

    let allocation = block_mgt::vehicle_allocation(&vehicle_id, provider)
        .await
        .map_err(|err| {
            Error::ProcessingError(format!(
                "failed to fetch block allocation for vehicle {vehicle_id}: {err}"
            ))
        })?
        .ok_or_else(|| {
            Error::ProcessingError(format!("block allocation unavailable for vehicle {vehicle_id}"))
        })?;
    let trip_id_value = allocation.trip_id.clone();
    let start_date_value = allocation.service_date.clone();
    let start_time_value = allocation.start_time.clone();
    debug!(vehicle_id = %vehicle_id, allocation = ?allocation, trip_id = %trip_id_value);

    let stop_id_value = stop_id(&vehicle_id, &event, provider).await?;

    trip_state::update_vehicle(
        &vehicle_id,
        Some(trip_id_value.as_str()),
        vehicle_seating,
        vehicle_total,
        &event,
        provider,
    )
    .await
    .map_err(|err| {
        Error::ProcessingError(format!(
            "failed to update trip state for vehicle {vehicle_id}: {err}"
        ))
    })?;

    let vt = VehicleTripInfo {
        vehicle_info: VehicleInfo {
            vehicle_id: vehicle_id.clone(),
            label: Some(vehicle_label.clone()),
        },
        trip_id: Some(trip_id_value.clone()),
        stop_id: Some(stop_id_value.clone()),
        last_received_timestamp: Some(event.clock.utc.clone()),
        dilax_message: Some(event.clone()),
    };
    trip_state::set_trip(vt, provider).await.map_err(|err| {
        Error::ProcessingError(format!(
            "failed to persist trip info for vehicle {vehicle_id}: {err}"
        ))
    })?;

    // -------------------------------------
    let enriched = EnrichedEvent {
        event,
        stop_id: Some(stop_id_value),
        trip_id: Some(trip_id_value),
        start_date: Some(start_date_value),
        start_time: Some(start_time_value),
    };

    let payload = serde_json::to_vec(&enriched).context("serializing event")?;
    let mut message = Message::new(&payload);
    if let Some(trip_id) = &enriched.trip_id {
        message.headers.insert("key".to_string(), trip_id.clone());
    }

    Publisher::send(provider, DILAX_ENRICHED_TOPIC, &message).await?;
    // -------------------------------------

    Ok(())
}

fn vehicle_label(event: &DilaxMessage) -> Option<String> {
    let site = event.device.site.clone();
    if site.is_empty() {
        return None;
    }

    let mut segments = Vec::new();
    let mut current = String::new();
    let mut current_is_digit: Option<bool> = None;

    for ch in site.chars() {
        let is_digit = ch.is_ascii_digit();
        match current_is_digit {
            None => {
                current.push(ch);
                current_is_digit = Some(is_digit);
            }
            Some(previous) if previous == is_digit => current.push(ch),
            Some(_) => {
                segments.push(std::mem::take(&mut current));
                current.push(ch);
                current_is_digit = Some(is_digit);
            }
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    if segments.is_empty() {
        return None;
    }

    let mut iter = segments.into_iter();
    let alpha = iter.next().unwrap();
    let numeric: String = iter.collect();
    if numeric.is_empty() {
        return None;
    }

    let mut prefix = match alpha.as_str() {
        "AM" => "AMP".to_string(),
        "AD" => "ADL".to_string(),
        _ => alpha,
    };

    let alpha_len = prefix.chars().count();
    let numeric_len = numeric.chars().count();
    let padding = VEHICLE_LABEL_WIDTH.saturating_sub(alpha_len + numeric_len);
    prefix.extend(std::iter::repeat_n(' ', padding));

    Some(format!("{prefix}{numeric}"))
}

fn vehicle_capacity(vehicle: &FleetVehicle) -> Option<(i64, i64)> {
    vehicle.capacity.as_ref().map(|capacity| (capacity.seating, capacity.total))
}

/// Resolve the GTFS stop identifier for the Dilax event waypoint.
///
/// # Errors
///
/// Returns an error when the waypoint is missing, provider requests fail, or no stop
/// matching the Dilax waypoint can be determined.
async fn stop_id(
    vehicle_id: &str, event: &DilaxMessage, http: &impl HttpRequest,
) -> Result<String> {
    let vehicle_id_owned = vehicle_id.to_string();

    let Some(waypoint) = event.wpt.as_ref() else {
        return Err(Error::ProcessingError(format!(
            "dilax-adapter event missing waypoint data for vehicle {vehicle_id_owned}"
        )));
    };

    let stops =
        gtfs::location_stops(&waypoint.lat, &waypoint.lon, STOP_SEARCH_DISTANCE_METERS, http)
            .await
            .map_err(|err| {
                Error::ProcessingError(format!(
                    "failed to look up stops for vehicle {vehicle_id_owned}: {err}"
                ))
            })?;
    if stops.is_empty() {
        return Err(Error::ProcessingError(format!(
            "stop id unavailable for vehicle {vehicle_id_owned}"
        )));
    }

    let stop_types = gtfs::stop_types(http).await.map_err(|err| {
        Error::ProcessingError(format!(
            "failed to look up stop types for vehicle {vehicle_id_owned}: {err}"
        ))
    })?;
    if stop_types.is_empty() {
        return Err(Error::ProcessingError(format!(
            "train stop types unavailable for vehicle {vehicle_id_owned}"
        )));
    }

    for stop in &stops {
        debug!(vehicle_id = %vehicle_id, stop = ?stop);

        if let Some(code) = stop.stop_code.as_deref()
            && is_station(&stop_types, code)
        {
            info!(vehicle_id = %vehicle_id, stop_id = %stop.stop_id, stop_code = code);
            return Ok(stop.stop_id.clone());
        }
    }

    Err(Error::ProcessingError(format!("stop id unavailable for vehicle {vehicle_id_owned}")))
}

fn is_station(stop_types: &[StopTypeEntry], stop_code: &str) -> bool {
    stop_types.iter().any(|entry| {
        entry.parent_stop_code.as_deref() == Some(stop_code)
            && entry.route_type == Some(StopType::Train as u32)
    })
}
