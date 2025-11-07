use std::borrow::ToOwned;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tracing::{debug, error, info, warn};

use crate::api::{BlockMgtProvider, CcStaticProvider, FleetProvider, GtfsStaticProvider};
use crate::config::Config;
use crate::error::Error;
use crate::occupancy::OccupancyStatus;
use crate::state::DilaxState;
use crate::store::KvStore;
use crate::types::{DilaxEnrichedEvent, DilaxEvent, FleetVehicle, StopTypeEntry, VehicleTripInfo};

const VEHICLE_TRIP_INFO_TTL: Duration = Duration::from_secs(2 * 24 * 60 * 60);
const STOP_SEARCH_DISTANCE_METERS: u32 = 150;
const VEHICLE_LABEL_WIDTH: usize = 14;
#[derive(Clone)]
pub struct DilaxProcessor {
    config: Config,
    store: KvStore,
    fleet: Arc<dyn FleetProvider>,
    cc_static: Arc<dyn CcStaticProvider>,
    gtfs: Arc<dyn GtfsStaticProvider>,
    block: Arc<dyn BlockMgtProvider>,
}

impl DilaxProcessor {
    #[allow(clippy::too_many_arguments)]
    pub fn with_providers(
        config: Config, store: KvStore, fleet: Arc<dyn FleetProvider>,
        cc_static: Arc<dyn CcStaticProvider>, gtfs: Arc<dyn GtfsStaticProvider>,
        block: Arc<dyn BlockMgtProvider>,
    ) -> Self {
        Self { config, store, fleet, cc_static, gtfs, block }
    }

    /// Enriches a Dilax event with vehicle, stop, trip, and occupancy information.
    ///
    /// # Errors
    /// Returns an error when one of the providers or the key-value store reports a failure
    /// while augmenting the incoming Dilax event.
    pub async fn process(&self, event: DilaxEvent) -> Result<DilaxEnrichedEvent> {
        let mut trip_id: Option<String> = None;
        let mut start_date: Option<String> = None;
        let mut start_time: Option<String> = None;

        let vehicle_label = Self::vehicle_label(&event);
        if vehicle_label.is_none() {
            warn!("Could not determine vehicle label from Dilax event: {:?}", event.device);
        }

        let mut vehicle: Option<FleetVehicle> = None;
        if let Some(label) = vehicle_label.as_deref() {
            if let Some(found) = self.lookup_vehicle(label).await? {
                vehicle = Some(found);
            } else {
                warn!(vehicle_label = %label, "Failed to resolve vehicle");
            }
        }

        let stop_id =
            self.lookup_stop_id(vehicle.as_ref().map(|fleet| fleet.id.as_str()), &event).await?;
        if stop_id.is_none() {
            if let Some(fleet) = vehicle.as_ref() {
                warn!(vehicle_id = %fleet.id, "Unable to resolve stop ID from Dilax event");
            } else {
                warn!("Unable to resolve stop ID from Dilax event without vehicle context");
            }
        }

        let Some(vehicle) = vehicle else {
            warn!("Failed to resolve vehicle for Dilax event; skipping passenger count processing");
            return Ok(DilaxEnrichedEvent { event, stop_id, trip_id, start_date, start_time });
        };
        let vehicle_id = vehicle.id.clone();

        let Some((vehicle_seating, vehicle_total)) = Self::vehicle_capacity(&vehicle) else {
            warn!(
                vehicle_id = %vehicle_id,
                "Vehicle lacks capacity information; skipping passenger count processing"
            );
            return Ok(DilaxEnrichedEvent { event, stop_id, trip_id, start_date, start_time });
        };

        if let Some(allocation) = self.block.allocation_by_vehicle(&vehicle_id).await? {
            trip_id = Some(allocation.trip_id.clone());
            start_date = Some(allocation.service_date.clone());
            start_time = Some(allocation.start_time.clone());
            debug!(vehicle_id = %vehicle_id, allocation = ?allocation, trip_id = ?trip_id);
        } else {
            warn!(vehicle_id = %vehicle_id, vehicle_label = ?vehicle_label, "Failed to resolve block allocation");
        }

        if let Err(error) = self.update_vehicle_state(
            &vehicle_id,
            trip_id.as_deref(),
            vehicle_seating,
            vehicle_total,
            &event,
        ) {
            error!(vehicle_id = %vehicle_id, error = ?error, "Failed to update Dilax vehicle state");
        }
        if let Err(error) = self.save_vehicle_trip_info(
            &vehicle_id,
            vehicle_label.as_deref(),
            trip_id.clone(),
            stop_id.clone(),
            &event,
        ) {
            error!(vehicle_id = %vehicle_id, error = ?error, "Failed to persist vehicle trip info");
        }

        Ok(DilaxEnrichedEvent { event, stop_id, trip_id, start_date, start_time })
    }

    fn vehicle_label(event: &DilaxEvent) -> Option<String> {
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

    async fn lookup_vehicle(&self, label: &str) -> Result<Option<FleetVehicle>> {
        match self.fleet.train_by_label(label).await {
            Ok(vehicle) => Ok(vehicle),
            Err(error) => {
                error!(label = label, error = ?error, "Failed to query Fleet API");
                Err(error)
            }
        }
    }

    fn vehicle_capacity(vehicle: &FleetVehicle) -> Option<(i64, i64)> {
        vehicle.capacity.as_ref().map(|capacity| (capacity.seating, capacity.total))
    }

    async fn lookup_stop_id(
        &self, vehicle_id: Option<&str>, event: &DilaxEvent,
    ) -> Result<Option<String>> {
        let vehicle_for_logs = vehicle_id.unwrap_or("unknown");
        let Some(waypoint) = event.wpt.as_ref() else {
            warn!(vehicle_id = %vehicle_for_logs, "Dilax event missing waypoint data");
            return Ok(None);
        };

        info!(
            vehicle_id = %vehicle_for_logs,
            lat = %waypoint.lat,
            lon = %waypoint.lon,
            "Querying CC Static for stop info"
        );
        let stops = self
            .cc_static
            .stops_by_location(&waypoint.lat, &waypoint.lon, STOP_SEARCH_DISTANCE_METERS)
            .await?;
        if stops.is_empty() {
            return Ok(None);
        }

        let train_stop_types = self.gtfs.train_stop_types().await?;
        if train_stop_types.is_empty() {
            warn!(vehicle_id = %vehicle_for_logs, "GTFS train stop types unavailable");
            return Ok(None);
        }

        for stop in &stops {
            debug!(vehicle_id = %vehicle_for_logs, stop = ?stop);
            if let Some(code) = stop.stop_code.as_deref()
                && Self::is_train_station(&train_stop_types, code)
            {
                info!(vehicle_id = %vehicle_for_logs, stop_id = %stop.stop_id, stop_code = code);
                return Ok(Some(stop.stop_id.clone()));
            }
        }

        Ok(None)
    }

    fn is_train_station(train_stop_types: &[StopTypeEntry], stop_code: &str) -> bool {
        train_stop_types.iter().any(|entry| {
            entry.parent_stop_code.as_deref() == Some(stop_code)
                && entry.route_type == Some(crate::types::StopType::TrainStop as u32)
        })
    }

    fn update_vehicle_state(
        &self, vehicle_id: &str, trip_id: Option<&str>, seating_capacity: i64, total_capacity: i64,
        event: &DilaxEvent,
    ) -> Result<()> {
        let state_key = format!("{}:{}", self.config.redis.apc_vehicle_id_state_key, vehicle_id);
        let state_prev = self.store.get_with_ttl(&state_key)?;
        let mut state = if let Some(raw) = state_prev.as_deref() {
            serde_json::from_slice::<DilaxState>(raw).unwrap_or_default()
        } else {
            let mut new_state = DilaxState::default();
            self.migrate_legacy_keys(vehicle_id, &mut new_state)?;
            new_state
        };

        let Ok(token) = event.clock.utc.parse::<i64>() else {
            warn!(
                vehicle_id = %vehicle_id,
                token = %event.clock.utc,
                "Unable to parse Dilax clock token"
            );
            return Ok(());
        };

        if token <= state.token {
            warn!(vehicle_id = %vehicle_id, token = token, last_token = state.token, "Received duplicate or out-of-order Dilax message");
            return Ok(());
        }
        state.token = token;

        let mut reset_running_count = false;
        if let Some(trip_id) = trip_id {
            match &state.last_trip_id {
                None => state.last_trip_id = Some(trip_id.to_string()),
                Some(last) if last != trip_id => {
                    reset_running_count = true;
                    state.last_trip_id = Some(trip_id.to_string());
                }
                _ => {}
            }
        } else {
            reset_running_count = true;
        }

        if reset_running_count {
            state.count = 0;
            warn!(vehicle_id = %vehicle_id, "Reset running passenger count");
            Self::update_running_count(event, &mut state, vehicle_id, true);
        } else {
            Self::update_running_count(event, &mut state, vehicle_id, false);
        }

        Self::update_occupancy(&mut state, vehicle_id, seating_capacity, total_capacity);

        let state_json =
            serde_json::to_string(&state).map_err(|err| Error::State(err.to_string()))?;
        let last_value =
            self.store.replace_with_ttl(&state_key, state_json.as_bytes(), self.config.apc_ttl)?;
        if let (Some(before), Some(during)) = (state_prev.as_ref(), last_value.as_ref())
            && before != during
        {
            warn!(
                vehicle_id = %vehicle_id,
                previous = %String::from_utf8_lossy(before),
                replaced = %String::from_utf8_lossy(during),
                "State overwritten concurrently"
            );
        }

        if let Some(ref occupancy) = state.occupancy_status {
            let occupancy_key = format!("{}:{}", self.config.redis.key_occupancy, vehicle_id);
            self.store.set_string_with_ttl(
                &occupancy_key,
                occupancy,
                self.config.occupancy_state_ttl,
            )?;
        }

        let count_key = format!("{}:{}", self.config.redis.apc_vehicle_id_key, vehicle_id);
        self.store.set_string_with_ttl(
            &count_key,
            &state.count.to_string(),
            self.config.apc_ttl,
        )?;

        Ok(())
    }

    fn migrate_legacy_keys(&self, vehicle_id: &str, state: &mut DilaxState) -> Result<()> {
        let migration_key =
            format!("{}:{}", self.config.redis.apc_vehicle_id_migrated_key, vehicle_id);
        if self.store.get_string(&migration_key)?.is_some() {
            return Ok(());
        }

        let legacy_trip_key = format!("{}:{}", self.config.redis.apc_vehicle_trip_key, vehicle_id);
        if let Some(trip_id) = self.store.get_string(&legacy_trip_key)? {
            warn!(vehicle_id = %vehicle_id, trip_id = %trip_id, "Migrating legacy trip ID");
            state.last_trip_id = Some(trip_id);
        }

        let legacy_count_key = format!("{}:{}", self.config.redis.apc_vehicle_id_key, vehicle_id);
        if let Some(count) = self.store.get_string(&legacy_count_key)?
            && let Ok(count_int) = count.parse::<i64>()
        {
            warn!(vehicle_id = %vehicle_id, count = count_int, "Migrating legacy passenger count");
            state.count = count_int;
        }

        self.store.set_string(&migration_key, "true")?;
        Ok(())
    }

    fn update_running_count(
        event: &DilaxEvent, state: &mut DilaxState, vehicle_id: &str, skip_out: bool,
    ) {
        let mut total_in = 0_i64;
        let mut total_out = 0_i64;
        let mut total_out_no_skip = 0_i64;
        for door in &event.doors {
            total_in += i64::from(door.passengers_in);
            total_out_no_skip += i64::from(door.passengers_out);
            if !skip_out {
                total_out += i64::from(door.passengers_out);
            }
        }

        info!(
            vehicle_id = %vehicle_id,
            total_in,
            total_out,
            total_out_no_skip,
            skip_out,
            "Accumulated door counts"
        );

        let previous = state.count;
        let current = (previous - total_out).max(0) + total_in;
        if current < 0 {
            warn!(vehicle_id = %vehicle_id, count = current, "Calculated negative passenger count");
        }
        state.count = current.max(0);
        info!(vehicle_id = %vehicle_id, passenger_count = state.count, "Updated running passenger count");
    }

    fn update_occupancy(
        state: &mut DilaxState, vehicle_id: &str, seating_capacity: i64, total_capacity: i64,
    ) {
        let occupancy = if state.count < Self::occupancy_threshold(seating_capacity, 5) {
            OccupancyStatus::Empty
        } else if state.count < Self::occupancy_threshold(seating_capacity, 40) {
            OccupancyStatus::ManySeatsAvailable
        } else if state.count < Self::occupancy_threshold(seating_capacity, 90) {
            OccupancyStatus::FewSeatsAvailable
        } else if state.count < Self::occupancy_threshold(total_capacity, 90) {
            OccupancyStatus::StandingRoomOnly
        } else {
            OccupancyStatus::Full
        };

        info!(vehicle_id = %vehicle_id, occupancy = %occupancy, "Updated occupancy status");
        state.occupancy_status = Some(occupancy.to_string());
    }

    fn save_vehicle_trip_info(
        &self, vehicle_id: &str, vehicle_label: Option<&str>, trip_id: Option<String>,
        stop_id: Option<String>, event: &DilaxEvent,
    ) -> Result<()> {
        let key = format!("{}:{}", self.config.redis.key_vehicle_trip_info, vehicle_id);
        let payload = VehicleTripInfo {
            vehicle_info: crate::types::VehicleInfo {
                vehicle_id: vehicle_id.to_string(),
                label: vehicle_label.map(ToOwned::to_owned),
            },
            trip_id,
            stop_id,
            last_received_timestamp: Some(event.clock.utc.clone()),
            dilax_message: Some(event.clone()),
        };
        self.store.set_json_with_ttl(&key, &payload, VEHICLE_TRIP_INFO_TTL)?;
        Ok(())
    }

    const fn occupancy_threshold(base: i64, percent: i64) -> i64 {
        base.saturating_mul(percent).div_euclid(100)
    }
}
