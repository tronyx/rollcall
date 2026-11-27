use super::{AcquisitionKind, EquipmentCategory};
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A piece of equipment or an accessory, with its brand and printer names.
#[derive(Debug, Clone, Serialize)]
pub struct Equipment {
    pub id: Uuid,
    pub category: EquipmentCategory,
    pub kind: Option<String>,
    pub name: String,
    pub brand_id: Option<Uuid>,
    pub brand_name: Option<String>,
    pub printer_id: Option<Uuid>,
    pub printer_name: Option<String>,
    pub model: Option<String>,
    pub quantity: i32,
    pub retired: bool,
    pub acquisition: AcquisitionKind,
    pub acquired_on: Option<NaiveDate>,
    pub acquired_from: Option<String>,
    pub price: Option<Decimal>,
    pub currency: String,
    pub discounted: bool,
    pub discount_note: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// `POST /api/equipment` and `PUT /api/equipment/{id}`. The brand is given by
/// name and created if it's new, like a filament's.
#[derive(Debug, Clone, Deserialize)]
pub struct SaveEquipment {
    pub category: EquipmentCategory,
    pub kind: Option<String>,
    pub name: String,
    pub brand_name: Option<String>,
    pub printer_id: Option<Uuid>,
    pub model: Option<String>,
    #[serde(default = "one")]
    pub quantity: i32,
    #[serde(default)]
    pub retired: bool,
    #[serde(default)]
    pub acquisition: AcquisitionKind,
    pub acquired_on: Option<NaiveDate>,
    pub acquired_from: Option<String>,
    pub price: Option<Decimal>,
    /// Defaults to the instance's default currency.
    pub currency: Option<String>,
    #[serde(default)]
    pub discounted: bool,
    pub discount_note: Option<String>,
    pub notes: Option<String>,
}

fn one() -> i32 {
    1
}

/// Query-string filters for `GET /api/equipment`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct EquipmentFilter {
    pub category: Option<EquipmentCategory>,
    pub brand_id: Option<Uuid>,
    /// A printer's id, or `none` for items not tied to a printer.
    pub printer: Option<String>,
    /// Free text over name, kind, model, brand and notes.
    pub q: Option<String>,
    /// Include retired items. Off by default.
    #[serde(default)]
    pub include_retired: bool,
}
