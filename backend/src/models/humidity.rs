use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One hygrometer reading for a spool.
#[derive(Debug, Clone, Serialize)]
pub struct HumidityReading {
    pub id: Uuid,
    pub spool_id: Uuid,
    pub measured_at: DateTime<Utc>,
    pub humidity_pct: Decimal,
    pub temperature_c: Option<Decimal>,
    pub note: Option<String>,
}

/// `POST /api/spools/{id}/humidity`. `measured_at` defaults to now.
#[derive(Debug, Clone, Deserialize)]
pub struct NewHumidityReading {
    pub measured_at: Option<DateTime<Utc>>,
    pub humidity_pct: Decimal,
    pub temperature_c: Option<Decimal>,
    pub note: Option<String>,
}
