use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "spool_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SpoolStatus {
    Sealed,
    Active,
    Drying,
    Empty,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "weight_event_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WeightEventKind {
    Initial,
    Print,
    QuickAdjust,
    WeighIn,
    Correction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "location_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum LocationKind {
    DryBox,
    AmsSlot,
    Printer,
    Dryer,
    Shelf,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "spec_source", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SpecSource {
    #[default]
    Manual,
    Scraped,
    Llm,
}

/// How a spool (or a piece of equipment) was acquired.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "acquisition_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AcquisitionKind {
    #[default]
    Bought,
    Gift,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "equipment_category", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum EquipmentCategory {
    Equipment,
    Accessory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "print_slot_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PrintSlotStatus {
    Pending,
    Logged,
    Skipped,
}
