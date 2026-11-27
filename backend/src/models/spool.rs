use super::{AcquisitionKind, LocationKind, NewFilament, SpoolStatus, WeightEventKind};
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One spool as shown in the inventory list: spool fields plus the joined
/// filament, brand, material and location names needed for cards and filters.
#[derive(Debug, Clone, Serialize)]
pub struct SpoolView {
    pub id: Uuid,
    pub label_code: Option<String>,
    pub status: SpoolStatus,
    pub initial_weight_g: Decimal,
    pub remaining_weight_g: Decimal,
    pub tare_weight_g: Option<Decimal>,
    pub remaining_pct: Decimal,
    pub is_low_stock: bool,
    pub purchase_price: Option<Decimal>,
    pub currency: String,
    pub purchase_date: Option<NaiveDate>,
    pub vendor: Option<String>,
    pub acquisition: AcquisitionKind,
    pub discounted: bool,
    pub discount_note: Option<String>,
    pub lot_number: Option<String>,
    pub opened_at: Option<DateTime<Utc>>,
    pub last_dried_at: Option<DateTime<Utc>>,
    pub emptied_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    pub filament_id: Uuid,
    pub product_name: Option<String>,
    pub color_name: String,
    pub color_hex: Option<String>,
    pub color_hex_secondary: Option<String>,
    pub diameter_mm: Decimal,
    pub image_url: Option<String>,
    pub brand_id: Uuid,
    pub brand_name: String,
    pub material_id: Uuid,
    pub material_code: String,
    pub location_id: Option<Uuid>,
    pub location_name: Option<String>,
    pub location_kind: Option<LocationKind>,
}

/// Query-string filters for `GET /api/spools`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SpoolFilter {
    /// Free text over brand, product, color, label code and notes.
    pub q: Option<String>,
    /// Material code, e.g. `PETG`. Matches the exact code.
    pub material: Option<String>,
    /// Matches either the material code or its family (`PLA` also finds `PLA-SILK`).
    pub material_family: Option<String>,
    pub brand_id: Option<Uuid>,
    pub location_id: Option<Uuid>,
    pub status: Option<SpoolStatus>,
    /// Substring match on color name.
    pub color: Option<String>,
    #[serde(default)]
    pub low_stock: bool,
    /// Include empty and archived spools. Off by default.
    #[serde(default)]
    pub include_empty: bool,
}

/// `POST /api/spools`. Give either `filament_id` (existing product) or
/// `filament` (inline new product, e.g. from the URL parser).
#[derive(Debug, Clone, Deserialize)]
pub struct CreateSpool {
    pub filament_id: Option<Uuid>,
    pub filament: Option<NewFilament>,
    pub label_code: Option<String>,
    pub location_id: Option<Uuid>,
    #[serde(default = "default_status")]
    pub status: SpoolStatus,
    pub initial_weight_g: Decimal,
    /// Defaults to `initial_weight_g` (a new roll).
    pub remaining_weight_g: Option<Decimal>,
    pub tare_weight_g: Option<Decimal>,
    pub purchase_price: Option<Decimal>,
    pub currency: Option<String>,
    pub purchase_date: Option<NaiveDate>,
    pub vendor: Option<String>,
    /// Bought (the default), a gift, or another way.
    #[serde(default)]
    pub acquisition: AcquisitionKind,
    #[serde(default)]
    pub discounted: bool,
    /// What the discount was. Ignored unless `discounted`.
    pub discount_note: Option<String>,
    pub lot_number: Option<String>,
    pub notes: Option<String>,
}

fn default_status() -> SpoolStatus {
    SpoolStatus::Sealed
}

/// `PUT /api/spools/:id`. Replaces the editable fields. Remaining weight is
/// changed only through the weight endpoint so every change is logged.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSpool {
    pub filament_id: Uuid,
    pub label_code: Option<String>,
    pub location_id: Option<Uuid>,
    pub status: SpoolStatus,
    pub initial_weight_g: Decimal,
    pub tare_weight_g: Option<Decimal>,
    pub purchase_price: Option<Decimal>,
    pub currency: String,
    pub purchase_date: Option<NaiveDate>,
    pub vendor: Option<String>,
    /// Bought (the default), a gift, or another way.
    #[serde(default)]
    pub acquisition: AcquisitionKind,
    #[serde(default)]
    pub discounted: bool,
    /// What the discount was. Ignored unless `discounted`.
    pub discount_note: Option<String>,
    pub lot_number: Option<String>,
    pub last_dried_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

/// `POST /api/spools/:id/weight`.
///
/// * `print`, `quick_adjust`, `correction`: send `delta_g` (negative = used).
/// * `weigh_in`: send `gross_reading_g` (scale reading incl. spool); the spool
///   must have a tare weight.
#[derive(Debug, Clone, Deserialize)]
pub struct WeightAdjustment {
    pub kind: WeightEventKind,
    pub delta_g: Option<Decimal>,
    pub gross_reading_g: Option<Decimal>,
    pub print_name: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeightEvent {
    pub id: i64,
    pub spool_id: Uuid,
    pub kind: WeightEventKind,
    pub delta_g: Decimal,
    pub weight_after_g: Decimal,
    pub gross_reading_g: Option<Decimal>,
    pub print_name: Option<String>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}
