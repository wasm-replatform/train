use credibil_api::{Body, Handler, Request, Response};
use tracing::{debug, info, warn};

use crate::block_mgt::{self, FleetVehicle};
use crate::error::Error;
use crate::gtfs::{self, StopType, StopTypeEntry};
use crate::provider::{HttpRequest, Provider};
use crate::trip_state::{VehicleInfo, VehicleTripInfo};
use crate::types::{DilaxEnrichedEvent, DilaxMessage};
use crate::{Result, trip_state};

const STOP_SEARCH_DISTANCE_METERS: u32 = 150;
const VEHICLE_LABEL_WIDTH: usize = 14;

async fn handle(
    _owner: &str, request: DilaxMessage, provider: &impl Provider,
) -> Result<Response<DilaxEnrichedEvent>> {
    let enriched = process(request, provider).await?;
    Ok(enriched.into())
}

impl<P: Provider> Handler<DilaxEnrichedEvent, P> for Request<DilaxMessage> {
    type Error = Error;

    // TODO: implement "owner"
    async fn handle(self, owner: &str, provider: &P) -> Result<Response<DilaxEnrichedEvent>> {
        handle(owner, self.body, provider).await
    }
}

impl Body for DilaxMessage {}

// Enriches a Dilax event with vehicle, stop, trip, and occupancy information.
///
/// # Errors
///
/// Returns an error when one of the providers or the key-value store reports a failure
/// while augmenting the incoming Dilax event.
pub async fn process(event: DilaxMessage, provider: &impl Provider) -> Result<DilaxEnrichedEvent> {
    let mut trip_id: Option<String> = None;
    let mut start_date: Option<String> = None;
    let mut start_time: Option<String> = None;

    // TODO: replace all warning and error tracing with strongly typed errors
    //  that can be handled at a higher level
    let vehicle_label = vehicle_label(&event);
    if vehicle_label.is_none() {
        warn!("Could not determine vehicle label from Dilax event: {:?}", event.device);
    }

    let vehicle = if let Some(label) = &vehicle_label {
        block_mgt::vehicle(label, provider).await.unwrap_or_else(|_| {
            warn!(vehicle_label = %label, "Failed to resolve vehicle");
            None
        })
    } else {
        None
    };

    let stop_id =
        stop_id(vehicle.as_ref().map(|fleet| fleet.id.as_str()), &event, provider).await?;
    if stop_id.is_none() {
        if let Some(fleet) = vehicle.as_ref() {
            warn!(vehicle_id = %fleet.id, "Unable to resolve stop ID from Dilax event");
        } else {
            warn!("Unable to resolve stop ID from Dilax event without vehicle context");
        }
    }

    let Some(vehicle) = &vehicle else {
        warn!("Failed to resolve vehicle for Dilax event; skipping passenger count processing");
        return Ok(DilaxEnrichedEvent { event, stop_id, trip_id, start_date, start_time });
    };
    let vehicle_id = vehicle.id.clone();

    let Some((vehicle_seating, vehicle_total)) = vehicle_capacity(vehicle) else {
        warn!(
            vehicle_id = %vehicle_id,
            "Vehicle lacks capacity information; skipping passenger count processing"
        );
        return Ok(DilaxEnrichedEvent { event, stop_id, trip_id, start_date, start_time });
    };

    if let Some(allocation) = block_mgt::vehicle_allocation(&vehicle_id, provider)
        .await
        .map_err(|e| Error::Internal(e.to_string()))?
    {
        trip_id = Some(allocation.trip_id.clone());
        start_date = Some(allocation.service_date.clone());
        start_time = Some(allocation.start_time.clone());
        debug!(vehicle_id = %vehicle_id, allocation = ?allocation, trip_id = ?trip_id);
    } else {
        warn!(vehicle_id = %vehicle_id, vehicle_label = ?vehicle_label, "Failed to resolve block allocation");
    }

    trip_state::update_vehicle(
        &vehicle_id,
        trip_id.as_deref(),
        vehicle_seating,
        vehicle_total,
        &event,
        provider,
    )
    .await
    .map_err(|e| Error::Internal(format!("Failed to update Dilax vehicle state: {e}")))?;

    let vt = VehicleTripInfo {
        vehicle_info: VehicleInfo { vehicle_id, label: vehicle_label },
        trip_id: trip_id.clone(),
        stop_id: stop_id.clone(),
        last_received_timestamp: Some(event.clock.utc.clone()),
        dilax_message: Some(event.clone()),
    };
    trip_state::set_trip(vt, provider)
        .await
        .map_err(|e| Error::Internal(format!("Failed to persist vehicle trip info: {e}")))?;

    Ok(DilaxEnrichedEvent { event, stop_id, trip_id, start_date, start_time })
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

async fn stop_id(
    vehicle_id: Option<&str>, event: &DilaxMessage, http: &impl HttpRequest,
) -> Result<Option<String>> {
    let vehicle_for_logs = vehicle_id.unwrap_or("unknown");
    let Some(waypoint) = event.wpt.as_ref() else {
        warn!(vehicle_id = %vehicle_for_logs, "Dilax event missing waypoint data");
        return Ok(None);
    };

    let stops =
        gtfs::location_stops(&waypoint.lat, &waypoint.lon, STOP_SEARCH_DISTANCE_METERS, http)
            .await
            .map_err(|e| Error::Internal(e.to_string()))?;
    if stops.is_empty() {
        return Ok(None);
    }

    let stop_types = gtfs::stop_types(http).await.map_err(|e| Error::Internal(e.to_string()))?;
    if stop_types.is_empty() {
        warn!(vehicle_id = %vehicle_for_logs, "GTFS train stop types unavailable");
        return Ok(None);
    }

    for stop in &stops {
        debug!(vehicle_id = %vehicle_for_logs, stop = ?stop);

        if let Some(code) = stop.stop_code.as_deref()
            && is_station(&stop_types, code)
        {
            info!(vehicle_id = %vehicle_for_logs, stop_id = %stop.stop_id, stop_code = code);
            return Ok(Some(stop.stop_id.clone()));
        }
    }

    Ok(None)
}

fn is_station(stop_types: &[StopTypeEntry], stop_code: &str) -> bool {
    stop_types.iter().any(|entry| {
        entry.parent_stop_code.as_deref() == Some(stop_code)
            && entry.route_type == Some(StopType::Train as u32)
    })
}
