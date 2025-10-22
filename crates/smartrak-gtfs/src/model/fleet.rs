use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VehicleInfo {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub registration: Option<String>,
    #[serde(default)]
    pub capacity: VehicleCapacity,
    #[serde(rename = "type", default)]
    pub vehicle_type: VehicleType,
    #[serde(default)]
    pub tag: Option<String>,
}

impl VehicleInfo {
    pub fn matches_tag(&self, expected: &str) -> bool {
        self.tag.as_deref().map(|tag| tag.eq_ignore_ascii_case(expected)).unwrap_or(false)
    }

    pub fn id_or_label(&self) -> &str {
        self.label.as_deref().filter(|label| !label.is_empty()).unwrap_or(&self.id)
    }

    pub fn is_train(&self) -> bool {
        self.vehicle_type
            .category
            .as_deref()
            .map(|ty| ty.eq_ignore_ascii_case("train"))
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VehicleCapacity {
    #[serde(default)]
    pub seating: Option<u32>,
    #[serde(default)]
    pub standing: Option<u32>,
    #[serde(default)]
    pub total: Option<u32>,
}

impl VehicleCapacity {
    pub fn total_or(&self, fallback: u32) -> u32 {
        self.total.unwrap_or(fallback)
    }

    pub fn seating_or(&self, fallback: u32) -> u32 {
        self.seating.unwrap_or(fallback)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VehicleType {
    #[serde(rename = "type", default)]
    pub category: Option<String>,
}

pub struct Tags;

impl Tags {
    pub const CAF: &'static str = "CAF";
    pub const SMARTRAK: &'static str = "Smartrak";
}
