use std::sync::Arc;

use anyhow::{Result, anyhow};
use chrono::Utc;
use dashmap::DashMap;
use tracing::{debug, error, info, warn};

use crate::cache::CacheRepository;
use crate::config::{CACHE_TTL_SIGN_ON, CACHE_TTL_TRIP_SERIAL, Config};
use crate::data_access::TripAccess;
use crate::locks::KeyLocker;
use crate::model::events::{DecodedSerialData, SmartrakEvent};
use crate::model::trip::TripInstance;
use crate::provider::AdapterProvider;

#[derive(Debug, Clone)]
pub struct SerialDataProcessor<P: AdapterProvider> {
    config: Arc<Config>,
    trip_access: TripAccess<P>,
    cache: Arc<CacheRepository<P::Cache>>,
    locker: KeyLocker,
    vehicle_timestamps: Arc<DashMap<String, i64>>,
}

impl<P: AdapterProvider> SerialDataProcessor<P> {
    pub fn new(
        config: Arc<Config>, trip_access: TripAccess<P>, cache: Arc<CacheRepository<P::Cache>>,
    ) -> Self {
        Self {
            config,
            trip_access,
            cache,
            locker: KeyLocker::new(),
            vehicle_timestamps: Arc::new(DashMap::new()),
        }
    }

    pub async fn process(&self, event: &SmartrakEvent) -> Result<()> {
        debug!(event = ?event, "serial data event received");

        let Some(vehicle_id) =
            event.remote_data.external_id.as_deref().filter(|value| !value.is_empty())
        else {
            warn!("serial data missing vehicle identifier");
            return Ok(());
        };

        if !self.is_valid(event, vehicle_id) {
            return Ok(());
        }

        let vehicle_id = vehicle_id.to_string();
        let guard = self.locker.lock(format!("serialData:{vehicle_id}"));
        let guard = guard.await;
        let result = self.process_locked(&vehicle_id, event).await;
        drop(guard);

        if let Err(err) = result {
            error!(vehicle = %vehicle_id, error = %err, "failed to process serial data event");
        }

        Ok(())
    }

    fn is_valid(&self, event: &SmartrakEvent, vehicle_id: &str) -> bool {
        if event.serial_data.decoded_serial_data.is_none() {
            debug!(vehicle = vehicle_id, "decoded serial data missing");
            return false;
        }

        let Some(timestamp) = event.message_data.timestamp else {
            warn!(vehicle = vehicle_id, "serial data missing timestamp");
            return false;
        };

        let now_secs = Utc::now().timestamp();
        let event_secs = timestamp.timestamp();
        if event_secs - now_secs > self.config.serial_data_filter_threshold {
            info!(
                vehicle = vehicle_id,
                event_time = event_secs,
                threshold = self.config.serial_data_filter_threshold,
                "serial data event filtered due to future timestamp"
            );
            return false;
        }

        true
    }

    async fn process_locked(&self, vehicle_id: &str, event: &SmartrakEvent) -> Result<()> {
        let timestamp = event
            .message_data
            .timestamp
            .ok_or_else(|| anyhow!("serial data missing timestamp"))?
            .timestamp();

        if self.is_old(vehicle_id, timestamp) {
            info!(vehicle = vehicle_id, "serial data event skipped as older than cached timestamp");
            return Ok(());
        }

        let decoded = event
            .serial_data
            .decoded_serial_data
            .as_ref()
            .ok_or_else(|| anyhow!("decoded serial data missing"))?;

        self.allocate_vehicle_to_trip(vehicle_id, decoded, timestamp).await
    }

    fn is_old(&self, vehicle_id: &str, timestamp: i64) -> bool {
        if let Some(prev) = self.vehicle_timestamps.get(vehicle_id) {
            if timestamp <= *prev {
                return true;
            }
        }
        self.vehicle_timestamps.insert(vehicle_id.to_string(), timestamp);
        false
    }

    async fn allocate_vehicle_to_trip(
        &self, vehicle_id: &str, decoded: &DecodedSerialData, timestamp: i64,
    ) -> Result<()> {
        let trip_key = self.config.trip_key(vehicle_id);
        let sign_on_key = self.config.sign_on_key(vehicle_id);

        let Some(trip_identifier) = decoded.trip_identifier() else {
            self.cache.delete(&sign_on_key).await?;
            self.cache.delete(&trip_key).await?;
            return Ok(());
        };
        let trip_identifier = trip_identifier.to_string();

        let prev_trip = self.cache.get_json::<TripInstance>(&trip_key).await?;

        let candidate = match prev_trip {
            Some(prev) => {
                if prev.trip_id == trip_identifier {
                    return Ok(());
                }

                match self
                    .trip_access
                    .get_nearest_trip_instance(&trip_identifier, timestamp, self.config.timezone)
                    .await?
                {
                    Some(trip) if !trip.has_error() => Some(trip),
                    _ => {
                        self.cache.delete(&sign_on_key).await?;
                        self.cache.delete(&trip_key).await?;
                        return Ok(());
                    }
                }
            }
            None => {
                self.trip_access
                    .get_nearest_trip_instance(&trip_identifier, timestamp, self.config.timezone)
                    .await?
            }
        };

        let Some(trip) = candidate else {
            return Ok(());
        };

        if trip.has_error() {
            return Ok(());
        }

        info!(monotonic_counter.smartrak_trip_descriptors = 1, source = "serial");
        self.cache.set_ex(&sign_on_key, CACHE_TTL_SIGN_ON, timestamp.to_string()).await?;
        self.cache.set_json_ex(&trip_key, CACHE_TTL_TRIP_SERIAL, &trip).await?;
        Ok(())
    }
}
