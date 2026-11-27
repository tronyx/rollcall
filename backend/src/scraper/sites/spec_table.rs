//! Generic key/value spec extraction: `<table>` rows, `<dl>` lists and
//! "Key: Value" list items. Most manufacturer pages use one of these.

use crate::scraper::draft::Draft;
use scraper::{ElementRef, Html, Selector};
use std::sync::LazyLock;

static ROWS: LazyLock<Selector> = LazyLock::new(|| Selector::parse("tr").unwrap());
static CELLS: LazyLock<Selector> = LazyLock::new(|| Selector::parse("th, td").unwrap());
static DT: LazyLock<Selector> = LazyLock::new(|| Selector::parse("dt").unwrap());
static ITEMS: LazyLock<Selector> = LazyLock::new(|| Selector::parse("li, p").unwrap());

pub fn clean_text(el: ElementRef<'_>) -> String {
    el.text()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(['\u{200e}', '\u{200f}', '\u{a0}'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// All key/value pairs found on the page, in document order.
pub fn pairs(doc: &Html) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for row in doc.select(&ROWS) {
        let cells: Vec<String> = row.select(&CELLS).map(clean_text).collect();
        if let [k, v] = cells.as_slice() {
            push(&mut out, k, v);
        }
    }
    for dt in doc.select(&DT) {
        if let Some(dd) = dt
            .next_siblings()
            .filter_map(ElementRef::wrap)
            .find(|e| e.value().name() == "dd")
        {
            push(&mut out, &clean_text(dt), &clean_text(dd));
        }
    }
    for item in doc.select(&ITEMS) {
        let text = clean_text(item);
        if text.len() > 160 {
            continue;
        }
        if let Some((k, v)) = text.split_once(':') {
            push(&mut out, k, v);
        }
    }
    out
}

fn push(out: &mut Vec<(String, String)>, k: &str, v: &str) {
    let k = k.trim().trim_end_matches(':').trim();
    let v = v.trim();
    if !k.is_empty() && k.len() <= 60 && !v.is_empty() && v.len() <= 200 {
        out.push((k.to_string(), v.to_string()));
    }
}

/// Maps well-known keys directly; returns the draft and the pairs flattened
/// to "key: value" lines for the regex pass.
pub fn extract(pairs: &[(String, String)]) -> (Draft, String) {
    let mut d = Draft::default();
    let mut text = String::new();
    for (k, v) in pairs {
        text.push_str(&format!("{k}: {v}\n"));
        let key = k.to_ascii_lowercase();
        let slot = if key == "brand" || key == "manufacturer" || key == "brand name" {
            &mut d.brand_name
        } else if key == "color" || key == "colour" || key == "color name" {
            &mut d.color_name
        } else {
            continue;
        };
        if slot.is_none() {
            *slot = Some(v.clone());
        }
    }
    (d, text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scraper::heuristics;

    #[test]
    fn tables_and_lists() {
        let html = r#"
          <table><tr><th>Brand</th><td>OVERTURE</td></tr>
                 <tr><td>Color</td><td>Space Gray</td></tr>
                 <tr><td>Printing Temperature</td><td>200-220 °C</td></tr></table>
          <dl><dt>Bed temperature</dt><dd>50-60°C</dd></dl>
          <ul><li>Diameter: 1.75 mm</li></ul>"#;
        let pairs = pairs(&Html::parse_document(html));
        let (d, text) = extract(&pairs);
        assert_eq!(d.brand_name.as_deref(), Some("OVERTURE"));
        assert_eq!(d.color_name.as_deref(), Some("Space Gray"));
        let h = heuristics::extract(&text);
        assert_eq!(h.nozzle_temp_min_c, Some(200));
        assert_eq!(h.bed_temp_max_c, Some(60));
        assert!(h.diameter_mm.is_some());
    }
}
