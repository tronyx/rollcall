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

#[derive(Debug, Clone, Serialize)]
pub struct Printer {
    pub id: Uuid,
    pub name: String,
    pub model: Option<String>,
    pub nozzle_diameter_mm: Decimal,
    pub is_default: bool,
    pub notes: Option<String>,
}

/// `POST /api/printers` and `PUT /api/printers/{id}`.
#[derive(Debug, Clone, Deserialize)]
pub struct NewPrinter {
    pub name: String,
    pub model: Option<String>,
    #[serde(default = "default_nozzle")]
    pub nozzle_diameter_mm: Decimal,
    /// Making one printer the default clears it on the others.
    #[serde(default)]
    pub is_default: bool,
    pub notes: Option<String>,
}

fn default_nozzle() -> Decimal {
    Decimal::new(40, 2)
}

/// The single row of instance-wide settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub low_stock_threshold_g: Decimal,
    pub default_currency: String,
}
