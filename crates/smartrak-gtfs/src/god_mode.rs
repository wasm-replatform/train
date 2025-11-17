use dashmap::DashMap;

use crate::models::{EventType, SmartrakEvent};

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

    pub fn describe(&self) -> String {
        let map: Vec<(String, String)> = self
            .overrides
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        serde_json::to_string(&map).unwrap_or_default()
    }

    pub fn preprocess(&self, event: &mut SmartrakEvent) {
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
            if override_trip.value() == "empty" {
                decoded.trip_id = None;
                decoded.trip_number = None;
                decoded.line_id = None;
            } else {
                decoded.trip_id = Some(override_trip.clone());
                decoded.trip_number = Some(override_trip.clone());
                decoded.line_id = None;
            }
        }
    }
}
