use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct InventoryStats {
    pub spools_in_stock: i64,
    pub spools_sealed: i64,
    pub spools_active: i64,
    pub spools_drying: i64,
    pub spools_empty: i64,
    pub remaining_kg: Decimal,
    pub low_stock_count: i64,
    pub low_stock_threshold_g: Decimal,
}
