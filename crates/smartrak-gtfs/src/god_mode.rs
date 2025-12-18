use std::env;
use std::sync::LazyLock;

use dashmap::DashMap;

use crate::{EventType, SmarTrakMessage};

#[derive(Default)]
pub struct GodMode {
    overrides: DashMap<String, String>,
}

impl GodMode {
    pub fn reset_all(&self) {
        self.overrides.clear();
    }

    pub fn reset_vehicle(&self, vehicle_id: &str) {
        self.overrides.remove(vehicle_id);
    }

    pub fn set_vehicle_to_trip(&self, vehicle_id: impl Into<String>, trip_id: impl Into<String>) {
        self.overrides.insert(vehicle_id.into(), trip_id.into());
    }

    #[must_use]
    pub fn describe(&self) -> String {
        let map: Vec<(String, String)> = self
            .overrides
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        serde_json::to_string(&map).unwrap_or_default()
    }

    pub fn preprocess(&self, event: &mut SmarTrakMessage) {
        if event.event_type != EventType::SerialData {
            return;
        }

        let Some(remote_data) = event.remote_data.as_ref() else {
            return;
        };

        let Some(vehicle_id) = remote_data.external_id.as_deref() else {
            return;
        };

        let Some(serial) = event.serial_data.as_mut() else {
            return;
        };

        let Some(decoded) = serial.decoded_serial_data.as_mut() else {
            return;
        };

        if let Some(override_trip) = self.overrides.get(vehicle_id) {
            let value = override_trip.value();

            decoded.line_id = None;

            if value == "empty" {
                decoded.trip_id = None;
                decoded.trip_number = None;
            } else {
                let override_trip = value.clone();
                decoded.trip_id = Some(override_trip.clone());
                decoded.trip_number = Some(override_trip);
            }
        }
    }
}

fn env_truthy(key: &str) -> bool {
    env::var(key).ok().is_some_and(|value| {
        let normalized = value.trim().to_ascii_lowercase();
        matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
    })
}

static GOD_MODE_ENABLED: LazyLock<bool> =
    LazyLock::new(|| env_truthy("SMARTRAK_GOD_MODE") || env_truthy("GOD_MODE"));
static GOD_MODE_INSTANCE: LazyLock<GodMode> = LazyLock::new(GodMode::default);

/// Returns the global God Mode instance when the feature flag is enabled.
#[must_use]
pub fn god_mode() -> Option<&'static GodMode> {
    (*GOD_MODE_ENABLED).then(|| &*GOD_MODE_INSTANCE)
}
