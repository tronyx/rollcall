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
