use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Where a field value came from, highest confidence first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldSource {
    /// Shopify product JSON (`/products/<handle>.js`).
    ShopifyJson,
    /// schema.org JSON-LD `Product` block.
    JsonLd,
    /// A site adapter that knows the page layout (e.g. Amazon).
    SiteAdapter,
    /// Key/value spec table or list found on the page.
    SpecTable,
    /// OpenGraph / meta tags.
    MetaTags,
    /// Regex over the product title.
    Title,
    /// Regex over visible page text.
    PageText,
    /// LLM-assisted extraction (only when configured).
    Llm,
    /// Inferred from the URL or domain only.
    Url,
    /// Derived from another field, e.g. hex from a color name.
    Derived,
}

/// The parse result's pre-fill data. Field names match `NewFilament` so the
/// frontend can spread it straight into the "add spool" form.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Draft {
    pub title: Option<String>,
    pub brand_name: Option<String>,
    pub material_code: Option<String>,
    pub product_name: Option<String>,
    pub color_name: Option<String>,
    pub color_hex: Option<String>,
    pub image_url: Option<String>,
    pub diameter_mm: Option<Decimal>,
    pub nominal_weight_g: Option<Decimal>,
    pub price: Option<Decimal>,
    pub currency: Option<String>,
    pub nozzle_temp_min_c: Option<i16>,
    pub nozzle_temp_max_c: Option<i16>,
    pub bed_temp_min_c: Option<i16>,
    pub bed_temp_max_c: Option<i16>,
    pub print_speed_min_mm_s: Option<i16>,
    pub print_speed_max_mm_s: Option<i16>,
    pub drying_temp_c: Option<i16>,
    pub drying_time_min: Option<i32>,
}

/// Lists every field once so merge/missing logic cannot drift from the struct.
macro_rules! for_each_field {
    ($m:ident) => {
        $m!(
            title,
            brand_name,
            material_code,
            product_name,
            color_name,
            color_hex,
            image_url,
            diameter_mm,
            nominal_weight_g,
            price,
            currency,
            nozzle_temp_min_c,
            nozzle_temp_max_c,
            bed_temp_min_c,
            bed_temp_max_c,
            print_speed_min_mm_s,
            print_speed_max_mm_s,
            drying_temp_c,
            drying_time_min
        )
    };
}

impl Draft {
    /// Copies every field that is still empty here from `other`, recording
    /// `source` for each one filled. Earlier (more trusted) extractors win.
    pub fn fill_from(
        &mut self,
        other: Draft,
        source: FieldSource,
        sources: &mut BTreeMap<String, FieldSource>,
    ) {
        macro_rules! fill {
            ($($f:ident),*) => {$(
                if self.$f.is_none() {
                    if let Some(v) = other.$f {
                        self.$f = Some(v);
                        sources.insert(stringify!($f).to_string(), source);
                    }
                }
            )*};
        }
        for_each_field!(fill);
    }

    /// Names of fields that are still empty.
    pub fn missing(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        macro_rules! check {
            ($($f:ident),*) => {$( if self.$f.is_none() { out.push(stringify!($f)); } )*};
        }
        for_each_field!(check);
        out
    }

    /// True when the fields needed to save a spool and print with it are present.
    pub fn has_essentials(&self) -> bool {
        self.brand_name.is_some()
            && self.material_code.is_some()
            && self.color_name.is_some()
            && self.nozzle_temp_min_c.is_some()
            && self.bed_temp_min_c.is_some()
    }

    /// Fixes inverted ranges and drops values outside plausible bounds
    /// (mirrors the database CHECK constraints so the form never fails to save).
    pub fn sanitize(&mut self) {
        fn order<T: PartialOrd + Copy>(a: &mut Option<T>, b: &mut Option<T>) {
            if let (Some(x), Some(y)) = (*a, *b) {
                if x > y {
                    *a = Some(y);
                    *b = Some(x);
                }
            }
        }
        fn bound(v: &mut Option<i16>, lo: i16, hi: i16) {
            if v.is_some_and(|x| x < lo || x > hi) {
                *v = None;
            }
        }
        bound(&mut self.nozzle_temp_min_c, 150, 500);
        bound(&mut self.nozzle_temp_max_c, 150, 500);
        bound(&mut self.bed_temp_min_c, 0, 150);
        bound(&mut self.bed_temp_max_c, 0, 150);
        bound(&mut self.print_speed_min_mm_s, 1, 2000);
        bound(&mut self.print_speed_max_mm_s, 1, 2000);
        bound(&mut self.drying_temp_c, 30, 150);
        if self.drying_time_min.is_some_and(|m| m <= 0 || m > 72 * 60) {
            self.drying_time_min = None;
        }
        order(&mut self.nozzle_temp_min_c, &mut self.nozzle_temp_max_c);
        order(&mut self.bed_temp_min_c, &mut self.bed_temp_max_c);
        order(
            &mut self.print_speed_min_mm_s,
            &mut self.print_speed_max_mm_s,
        );
        for s in [
            &mut self.title,
            &mut self.brand_name,
            &mut self.product_name,
            &mut self.color_name,
        ] {
            if let Some(v) = s {
                let t = v.split_whitespace().collect::<Vec<_>>().join(" ");
                *s = (!t.is_empty()).then_some(t);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earlier_source_wins() {
        let mut d = Draft {
            brand_name: Some("Polymaker".into()),
            ..Default::default()
        };
        let mut src = BTreeMap::new();
        d.fill_from(
            Draft {
                brand_name: Some("Other".into()),
                color_name: Some("Red".into()),
                ..Default::default()
            },
            FieldSource::PageText,
            &mut src,
        );
        assert_eq!(d.brand_name.as_deref(), Some("Polymaker"));
        assert_eq!(d.color_name.as_deref(), Some("Red"));
        assert_eq!(src.get("color_name"), Some(&FieldSource::PageText));
        assert!(!src.contains_key("brand_name"));
    }

    #[test]
    fn sanitize_swaps_and_bounds() {
        let mut d = Draft {
            nozzle_temp_min_c: Some(230),
            nozzle_temp_max_c: Some(190),
            bed_temp_min_c: Some(900),
            ..Default::default()
        };
        d.sanitize();
        assert_eq!(
            (d.nozzle_temp_min_c, d.nozzle_temp_max_c),
            (Some(190), Some(230))
        );
        assert_eq!(d.bed_temp_min_c, None);
    }
}
