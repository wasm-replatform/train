use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::api::{BlockMgtClient, BlockMgtProvider, Clock};
use crate::config::Config;
use crate::error::Error;
use crate::provider::HttpRequest;
use crate::store::KvStore;
use crate::types::{VehicleAllocation, VehicleInfo, VehicleTripInfo};

const DIESEL_TRAIN_PREFIX: &str = "ADL";

#[allow(clippy::module_name_repetitions)]
#[derive(Clone)]
pub struct SystemClock {
    timezone: Tz,
}

impl SystemClock {
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn from_timezone(timezone: Tz) -> Self {
        Self { timezone }
    }

    #[must_use]
    pub fn from_config(config: &Config) -> Self {
        match config.timezone.parse::<Tz>() {
            Ok(tz) => Self::from_timezone(tz),
            Err(err) => {
                warn!(timezone = %config.timezone, error = %err, "Invalid timezone; defaulting to UTC");
                Self::from_timezone(chrono_tz::UTC)
            }
        }
    }
}

impl Clock for SystemClock {
    fn now_utc(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn timezone(&self) -> Tz {
        self.timezone
    }
}

pub type DefaultDetector<H> = DilaxLostConnectionsDetector<BlockMgtClient<H>, SystemClock>;

pub fn build_default_detector<H>(config: Config, store: KvStore, http: Arc<H>) -> DefaultDetector<H>
where
    H: HttpRequest + ?Sized,
{
    let block = Arc::new(BlockMgtClient::new(http));
    let clock = Arc::new(SystemClock::from_config(&config));
    DilaxLostConnectionsDetector::new(config, store, block, clock)
}

/// Executes the lost-connection detector with the default Dilax dependencies.
///
/// # Errors
///
/// Returns an error when the Dilax key-value store cannot be opened, when block
/// allocations cannot be refreshed, or when the detection pipeline encounters a failure.
pub async fn run_lost_connection_job<H>(http: Arc<H>) -> Result<Vec<LostConnectionDetection>>
where
    H: HttpRequest + ?Sized,
{
    info!("Starting Dilax lost connection job");
    let config = Config::default();
    let store = KvStore::open("dilax").context("opening dilax store")?;
    let detector = build_default_detector(config, store, http);
    detector.refresh_allocations().await.context("refreshing Dilax allocations")?;
    let detections = detector.detect().context("detecting Dilax lost connections")?;
    info!(count = detections.len(), "Completed Dilax lost connection job");
    Ok(detections)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LostConnectionDetection {
    pub detection_time: i64,
    pub allocation: VehicleAllocation,
    pub vehicle_trip_info: VehicleTripInfo,
}

fn log_detection(detection: &LostConnectionDetection, tz: Tz) {
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
        .and_then(|value| value.parse::<i64>().ok())
        .map_or_else(
            || String::from("Never received a Dilax message"),
            |ts| format_timestamp(ts, tz),
        );

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

fn format_timestamp(timestamp: i64, tz: Tz) -> String {
    DateTime::<Utc>::from_timestamp(timestamp, 0)
        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap())
        .with_timezone(&tz)
        .format("%Y-%m-%d %H:%M:%S %Z")
        .to_string()
}

pub struct DilaxLostConnectionsDetector<B, C>
where
    B: BlockMgtProvider + ?Sized,
    C: Clock + ?Sized,
{
    config: Config,
    store: KvStore,
    block: Arc<B>,
    clock: Arc<C>,
    allocations: Arc<RwLock<Vec<VehicleAllocation>>>,
}

impl<B, C> DilaxLostConnectionsDetector<B, C>
where
    B: BlockMgtProvider + ?Sized,
    C: Clock + ?Sized,
{
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(config: Config, store: KvStore, block: Arc<B>, clock: Arc<C>) -> Self {
        Self { config, store, block, clock, allocations: Arc::new(RwLock::new(Vec::new())) }
    }

    #[must_use]
    pub fn allocations(&self) -> Arc<RwLock<Vec<VehicleAllocation>>> {
        Arc::clone(&self.allocations)
    }

    /// Refreshes cached allocations for the current service day.
    ///
    /// # Errors
    ///
    /// Returns an error if the block management provider or backing store cannot be queried.
    pub async fn refresh_allocations(&self) -> Result<()> {
        let all_allocations = self.block.all_allocations().await?;
        info!(count = all_allocations.len(), "Loaded block allocations from provider");
        let tz: Tz = self.clock.timezone();
        let today = self.clock.now_utc().with_timezone(&tz);
        let service_date = today.format("%Y%m%d").to_string();

        let filtered: Vec<VehicleAllocation> = all_allocations
            .into_iter()
            .filter(|allocation| {
                allocation.service_date == service_date
                    && !allocation.vehicle_id.is_empty()
                    && !allocation.vehicle_label.starts_with(DIESEL_TRAIN_PREFIX)
            })
            .collect();

        info!(service_date = %service_date, cached = filtered.len(), "Caching allocations for today");
        if let Ok(mut slots) = self.allocations.write() {
            *slots = filtered;
        }

        Ok(())
    }

    /// Runs the lost-connection detection workflow.
    ///
    /// # Errors
    ///
    /// Returns an error when Redis access or candidate deserialization fails.
    pub fn detect(&self) -> Result<Vec<LostConnectionDetection>> {
        info!("Starting Dilax lost connection detection pass");
        let candidates = self.detect_candidates()?;
        debug!(candidate_count = candidates.len(), "Dilax detection candidates evaluated");
        if candidates.is_empty() {
            info!("No Dilax lost connection candidates found");
            return Ok(Vec::new());
        }

        let tz: Tz = self.clock.timezone();
        let now_local = self.clock.now_utc().with_timezone(&tz);
        let set_key =
            format!("{}{}", self.config.redis.lost_connections_set, now_local.format("%Y%m%d"));
        let mut existing: HashSet<String> = self.store.set_members(&set_key)?.into_iter().collect();
        let mut new_detections = Vec::new();

        for candidate in candidates {
            let vehicle_trip_key = format!(
                "{}|{}",
                candidate.vehicle_trip_info.vehicle_info.vehicle_id, candidate.allocation.trip_id
            );
            if existing.contains(&vehicle_trip_key) {
                info!(vehicle_trip_key = %vehicle_trip_key, "Lost connection already emitted");
                continue;
            }

            log_detection(&candidate, tz);

            info!(vehicle_trip_key = %vehicle_trip_key, "Emitting Dilax lost connection detection");

            let detail_key = format!("{set_key}:{vehicle_trip_key}");
            let payload =
                serde_json::to_string(&candidate).map_err(|err| Error::State(err.to_string()))?;
            self.store.add_to_set(&set_key, &vehicle_trip_key)?;
            self.store.set_expiry(&set_key, self.config.lost_connection_retention)?;
            self.store.set_string_with_ttl(
                &detail_key,
                &payload,
                self.config.lost_connection_retention,
            )?;

            existing.insert(vehicle_trip_key);
            new_detections.push(candidate);
        }

        info!(count = new_detections.len(), "Dilax lost connection detections recorded");
        Ok(new_detections)
    }

    fn detect_candidates(&self) -> Result<Vec<LostConnectionDetection>> {
        let detection_time = self.clock.now_utc().timestamp();
        info!(detection_time, "Evaluating Dilax lost connection candidates");
        let allocations = {
            let guard = self.allocations.read().expect("allocations lock poisoned");
            guard.clone()
        };

        debug!(
            detection_time,
            running_allocations = allocations.len(),
            "Evaluating lost Dilax connections"
        );

        let running: Vec<VehicleAllocation> = allocations
            .into_iter()
            .filter(|allocation| {
                allocation.start_datetime <= detection_time
                    && allocation.end_datetime >= detection_time
            })
            .collect();

        debug!(running_count = running.len(), "Dilax services currently running");

        let mut detections = Vec::new();
        for allocation in running {
            if let Some(info) = self.store.get_json_with_ttl::<VehicleTripInfo>(&format!(
                "{}:{}",
                self.config.redis.key_vehicle_trip_info, allocation.vehicle_id
            ))? {
                if info.trip_id.as_deref() == Some(&allocation.trip_id) {
                    let last_timestamp = info
                        .last_received_timestamp
                        .as_deref()
                        .and_then(|value| value.parse::<i64>().ok());
                    let lost = matches!(last_timestamp, Some(last)
                        if self.is_connection_lost(detection_time, last));
                    if lost {
                        debug!(vehicle_id = %allocation.vehicle_id, trip_id = %allocation.trip_id, detection_time, last_timestamp, "Dilax connection lost for matching trip");
                        detections.push(LostConnectionDetection {
                            detection_time,
                            allocation: allocation.clone(),
                            vehicle_trip_info: info,
                        });
                    }
                } else if let Some(detection) =
                    self.detect_for_allocation(detection_time, &allocation, Some(info))
                {
                    detections.push(detection);
                }
            } else if let Some(detection) =
                self.detect_for_allocation(detection_time, &allocation, None)
            {
                detections.push(detection);
            }
        }

        Ok(detections)
    }

    fn detect_for_allocation(
        &self, detection_time: i64, allocation: &VehicleAllocation,
        existing: Option<VehicleTripInfo>,
    ) -> Option<LostConnectionDetection> {
        if !self.is_connection_lost(detection_time, allocation.start_datetime) {
            return None;
        }

        let vehicle_trip_info = existing.unwrap_or_else(|| VehicleTripInfo {
            vehicle_info: VehicleInfo {
                vehicle_id: allocation.vehicle_id.clone(),
                label: Some(allocation.vehicle_label.clone()),
            },
            trip_id: Some(allocation.trip_id.clone()),
            stop_id: None,
            last_received_timestamp: None,
            dilax_message: None,
        });

        debug!(
            vehicle_label = %allocation.vehicle_label,
            vehicle_id = %allocation.vehicle_id,
            detection_time,
            start_time = allocation.start_datetime,
            "Dilax vehicle lost tracking"
        );

        Some(LostConnectionDetection {
            detection_time,
            allocation: allocation.clone(),
            vehicle_trip_info,
        })
    }

    #[allow(clippy::missing_const_for_fn)]
    fn is_connection_lost(&self, detection_time: i64, timestamp: i64) -> bool {
        let Ok(threshold) = i64::try_from(self.config.lost_connection_threshold.as_secs()) else {
            return true;
        };
        timestamp.saturating_add(threshold) <= detection_time
    }
}
