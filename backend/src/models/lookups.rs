use super::LocationKind;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Brand {
    pub id: Uuid,
    pub name: String,
    pub website: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Material {
    pub id: Uuid,
    pub code: String,
    pub display_name: String,
    pub family: String,
    pub density_g_cm3: Option<Decimal>,
    pub default_drying_temp_c: Option<i16>,
    pub default_drying_time_min: Option<i32>,
    pub is_abrasive: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Location {
    pub id: Uuid,
    pub name: String,
    pub kind: LocationKind,
    pub description: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewLocation {
    pub name: String,
    #[serde(default = "default_location_kind")]
    pub kind: LocationKind,
    pub description: Option<String>,
    #[serde(default)]
    pub sort_order: i32,
}

fn default_location_kind() -> LocationKind {
    LocationKind::Other
}
