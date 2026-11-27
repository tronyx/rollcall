use super::SpecSource;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Manufacturer-recommended print specs. Shared by the filament row, the
/// create request and the URL parser output so the shapes always match.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FilamentSpecs {
    pub nozzle_temp_min_c: Option<i16>,
    pub nozzle_temp_max_c: Option<i16>,
    pub bed_temp_min_c: Option<i16>,
    pub bed_temp_max_c: Option<i16>,
    pub print_speed_min_mm_s: Option<i16>,
    pub print_speed_max_mm_s: Option<i16>,
    pub drying_temp_c: Option<i16>,
    pub drying_time_min: Option<i32>,
    pub density_g_cm3: Option<Decimal>,
    /// HueForge transmission distance (mm).
    pub td_value: Option<Decimal>,
}

/// A filament product joined with its brand and material names.
#[derive(Debug, Clone, Serialize)]
pub struct Filament {
    pub id: Uuid,
    pub brand_id: Uuid,
    pub brand_name: String,
    pub material_id: Uuid,
    pub material_code: String,
    pub product_name: Option<String>,
    pub color_name: String,
    pub color_hex: Option<String>,
    pub color_hex_secondary: Option<String>,
    pub diameter_mm: Decimal,
    pub nominal_weight_g: Option<Decimal>,
    pub typical_tare_g: Option<Decimal>,
    #[serde(flatten)]
    pub specs: FilamentSpecs,
    pub source_url: Option<String>,
    pub image_url: Option<String>,
    pub spec_source: SpecSource,
    pub scraped_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Inline filament definition used when adding a spool of a product that is
/// not in the catalog yet. Brand is matched case-insensitively and created if
/// missing; material must be an existing code (e.g. "PETG").
#[derive(Debug, Clone, Deserialize)]
pub struct NewFilament {
    pub brand_name: String,
    pub material_code: String,
    pub product_name: Option<String>,
    pub color_name: String,
    pub color_hex: Option<String>,
    pub color_hex_secondary: Option<String>,
    #[serde(default = "default_diameter")]
    pub diameter_mm: Decimal,
    pub nominal_weight_g: Option<Decimal>,
    pub typical_tare_g: Option<Decimal>,
    #[serde(flatten)]
    pub specs: FilamentSpecs,
    pub source_url: Option<String>,
    pub image_url: Option<String>,
    #[serde(default)]
    pub spec_source: SpecSource,
    /// Raw parser output, stored for later re-mapping.
    pub scrape_raw: Option<serde_json::Value>,
}

fn default_diameter() -> Decimal {
    Decimal::new(175, 2)
}

impl NewFilament {
    /// Normalizes user input before it hits database constraints.
    pub fn normalize(&mut self) {
        // Known brands get one spelling ("bambu lab" → "Bambu Lab").
        self.brand_name = crate::scraper::brands::canonicalize(&self.brand_name);
        self.material_code = self.material_code.trim().to_ascii_uppercase();
        self.color_name = self.color_name.trim().to_string();
        self.product_name = trim_opt(self.product_name.take());
        self.color_hex = self.color_hex.take().and_then(|h| normalize_hex(&h));
        self.color_hex_secondary = self
            .color_hex_secondary
            .take()
            .and_then(|h| normalize_hex(&h));
        self.source_url = trim_opt(self.source_url.take());
        self.image_url = trim_opt(self.image_url.take());
    }
}

pub(crate) fn trim_opt(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Accepts `#abc`, `abc`, `#aabbcc` or `aabbcc`; returns `#AABBCC`.
pub fn normalize_hex(input: &str) -> Option<String> {
    let s = input.trim().trim_start_matches('#');
    let full = match s.len() {
        3 => s.chars().flat_map(|c| [c, c]).collect::<String>(),
        6 => s.to_string(),
        _ => return None,
    };
    full.chars()
        .all(|c| c.is_ascii_hexdigit())
        .then(|| format!("#{}", full.to_ascii_uppercase()))
}

#[cfg(test)]
mod tests {
    use super::normalize_hex;

    #[test]
    fn hex_normalization() {
        assert_eq!(normalize_hex("#fff").as_deref(), Some("#FFFFFF"));
        assert_eq!(normalize_hex("00ae42").as_deref(), Some("#00AE42"));
        assert_eq!(normalize_hex("#12345"), None);
        assert_eq!(normalize_hex("zzzzzz"), None);
    }
}
