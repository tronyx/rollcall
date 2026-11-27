use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Your tested settings for a filament, on one printer or (with no
/// `printer_id`) on any printer. Empty fields fall back to the filament's
/// manufacturer values.
#[derive(Debug, Clone, Serialize)]
pub struct TunedSettings {
    pub id: Uuid,
    pub filament_id: Uuid,
    pub printer_id: Option<Uuid>,
    pub printer_name: Option<String>,
    pub nozzle_temp_c: Option<i16>,
    pub first_layer_nozzle_temp_c: Option<i16>,
    pub bed_temp_c: Option<i16>,
    pub print_speed_mm_s: Option<i16>,
    pub max_volumetric_speed_mm3_s: Option<Decimal>,
    pub flow_ratio: Option<Decimal>,
    pub pressure_advance: Option<Decimal>,
    pub retraction_length_mm: Option<Decimal>,
    pub retraction_speed_mm_s: Option<i16>,
    pub fan_speed_pct: Option<i16>,
    pub drying_temp_c: Option<i16>,
    pub drying_time_min: Option<i32>,
    pub notes: Option<String>,
    pub verified_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

/// `PUT /api/filaments/{id}/tuned`: creates or replaces the settings for
/// `printer_id` (or for any printer, when it's null).
#[derive(Debug, Clone, Deserialize)]
pub struct SaveTunedSettings {
    pub printer_id: Option<Uuid>,
    pub nozzle_temp_c: Option<i16>,
    pub first_layer_nozzle_temp_c: Option<i16>,
    pub bed_temp_c: Option<i16>,
    pub print_speed_mm_s: Option<i16>,
    pub max_volumetric_speed_mm3_s: Option<Decimal>,
    pub flow_ratio: Option<Decimal>,
    pub pressure_advance: Option<Decimal>,
    pub retraction_length_mm: Option<Decimal>,
    pub retraction_speed_mm_s: Option<i16>,
    pub fan_speed_pct: Option<i16>,
    pub drying_temp_c: Option<i16>,
    pub drying_time_min: Option<i32>,
    pub notes: Option<String>,
    /// Marks the settings as tested. Keeps the first date it was set; false clears it.
    #[serde(default)]
    pub verified: bool,
}
