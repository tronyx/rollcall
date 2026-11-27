use super::PrintSlotStatus;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A print logged from the slicer, with the filament slots it used.
#[derive(Debug, Clone, Serialize)]
pub struct Print {
    pub id: Uuid,
    pub name: String,
    pub slicer: Option<String>,
    pub printer_preset: Option<String>,
    pub created_at: DateTime<Utc>,
    pub slots: Vec<PrintSlot>,
}

/// One filament a print used, and the spool it was taken from.
#[derive(Debug, Clone, Serialize)]
pub struct PrintSlot {
    #[serde(skip)]
    pub print_id: Uuid,
    pub slot: i16,
    pub grams: Decimal,
    pub profile_name: Option<String>,
    pub filament_type: Option<String>,
    pub color_hex: Option<String>,
    pub status: PrintSlotStatus,
    pub spool_id: Option<Uuid>,
    pub weight_event_id: Option<i64>,
    /// The Rollcall filament this slicer preset is known to be, if any.
    pub profile_filament_id: Option<Uuid>,
    /// The spool, for showing it without another request.
    pub spool: Option<PrintSpool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrintSpool {
    pub label_code: Option<String>,
    pub brand_name: String,
    pub material_code: String,
    pub product_name: Option<String>,
    pub color_name: String,
    pub color_hex: Option<String>,
    pub remaining_weight_g: Decimal,
}

/// `GET /api/prints`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PrintFilter {
    /// Only prints with a slot that still needs a spool.
    #[serde(default)]
    pub pending: bool,
    pub limit: Option<i64>,
}

/// `PUT /api/prints/{id}/slots/{slot}`: take it from a spool, or skip it.
#[derive(Debug, Clone, Deserialize)]
pub struct AssignSlot {
    pub spool_id: Option<Uuid>,
    #[serde(default)]
    pub skip: bool,
    /// Remember the slot's slicer preset as this spool's filament, so the
    /// next print with it is logged by itself. Defaults to true.
    pub remember: Option<bool>,
}

/// `PUT /api/slicer-profiles`: this slicer preset is this filament.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlicerProfile {
    pub name: String,
    pub filament_id: Uuid,
}
