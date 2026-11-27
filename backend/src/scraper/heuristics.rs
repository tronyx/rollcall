//! Regex extraction over free text (titles, spec tables flattened to
//! "key: value" lines, and visible page text).

use super::draft::Draft;
use regex::Regex;
use rust_decimal::Decimal;
use std::sync::LazyLock;

macro_rules! re {
    ($s:expr) => {
        LazyLock::new(|| Regex::new($s).expect("valid regex"))
    };
}

/// Ordered most specific first; codes match the seeded `materials` table.
static MATERIALS: LazyLock<Vec<(&'static str, Regex)>> = LazyLock::new(|| {
    [
        ("PETG-CF", r"(?i)\bPETG[\s-]*CF\b|\bPETG\b.{0,25}carbon[\s-]*fib"),
        ("PLA-CF", r"(?i)\bPLA[\s-]*CF\b|\bPLA\b.{0,25}carbon[\s-]*fib"),
        ("PA-CF", r"(?i)\b(?:PA\d{0,2}|PAHT|nylon)[\s-]*CF\b|\b(?:PA\d{0,2}|nylon)\b.{0,25}carbon[\s-]*fib"),
        ("PLA-SILK", r"(?i)\bsilk\b.{0,25}\bPLA\b|\bPLA\b.{0,25}\bsilk\b"),
        ("PLA-MATTE", r"(?i)\bmatte?\b.{0,25}\bPLA\b|\bPLA\b.{0,25}\bmatte?\b"),
        ("PLA+", r"(?i)\bPLA\s*(?:\+|plus\b|pro\b)|\btough\s*PLA\b|\bPLA\s*tough\b"),
        ("PETG", r"(?i)\bPET[\s-]?G\b"),
        ("ASA", r"(?i)\bASA\b"),
        ("ABS", r"(?i)\bABS\b"),
        ("TPU", r"(?i)\bTPU\b|\bTPE\b|\b(?:85|90|95)\s*A\b"),
        ("PC", r"(?i)\bpolycarbonate\b|\bPC\b"),
        ("PA", r"(?i)\bnylon\b|\bPA(?:6|12|66)?\b"),
        ("HIPS", r"(?i)\bHIPS\b"),
        ("PVA", r"(?i)\bPVA\b"),
        ("PLA", r"(?i)\bPLA\b"),
    ]
    .into_iter()
    .map(|(code, pat)| (code, Regex::new(pat).expect("valid regex")))
    .collect()
});

static NOZZLE_LABEL: LazyLock<Regex> = re!(
    r"(?i)(?:nozzle|print(?:ing)?|extru(?:sion|der|ding)|hot\s*end)\s*temp(?:erature)?s?|temp(?:erature)?\s*\(?\s*nozzle"
);
static BED_LABEL: LazyLock<Regex> = re!(
    r"(?i)(?:hot\s*bed|heat(?:ed)?\s*bed|bed|build\s*plate|plate|platform)\s*temp(?:erature)?s?"
);
static SPEED_LABEL: LazyLock<Regex> = re!(r"(?i)print(?:ing)?\s*speed|recommended\s*speed");
static DRY_LABEL: LazyLock<Regex> = re!(r"(?i)dry(?:ing)?\b|dehydrat|bake");

static TEMP_RANGE: LazyLock<Regex> =
    re!(r"(?i)(\d{2,3})\s*(?:°|º|℃|deg)?\s*C?\s*(?:-|–|—|~|to|/)\s*(\d{2,3})\s*(?:°|º|℃|deg|C\b)");
static TEMP_SINGLE: LazyLock<Regex> = re!(r"(?i)(\d{2,3})\s*(?:°\s*C|º\s*C|℃|deg\s*C|°)");
static SPEED_RANGE: LazyLock<Regex> =
    re!(r"(?i)(\d{1,4})\s*(?:mm/s)?\s*(?:-|–|—|~|to)\s*(\d{1,4})\s*mm\s*/\s*s");
static SPEED_SINGLE: LazyLock<Regex> =
    re!(r"(?i)(?:≤|<=|up\s*to|max(?:imum)?)?\s*(\d{1,4})\s*mm\s*/\s*s");
static DRY_COND: LazyLock<Regex> = re!(
    r"(?i)(\d{2,3})\s*(?:°|º|℃|deg)?\s*C?\s*(?:,|/|x|\*|-|–)?\s*(?:for\s*)?(\d{1,2}(?:\.\d)?)\s*(?:-\s*\d{1,2}\s*)?(?:h\b|hr|hrs|hours?)"
);
static DIAMETER: LazyLock<Regex> = re!(r"(?i)\b(1\.75|2\.85|3\.00?)\s*mm\b");
static WEIGHT_KG: LazyLock<Regex> = re!(r"(?i)\b(\d{1,2}(?:\.\d{1,2})?)\s*kg\b");
static WEIGHT_G: LazyLock<Regex> = re!(r"(?i)\b(\d{3,4})\s*g\b");

/// How far after a label to look for its value.
const LABEL_WINDOW: usize = 90;

pub fn detect_material(text: &str) -> Option<&'static str> {
    MATERIALS
        .iter()
        .find(|(_, re)| re.is_match(text))
        .map(|(c, _)| *c)
}

/// Text that follows each label match: up to the window size, the end of the
/// line, or the start of the next "...temperature" label, whichever is first.
fn after_labels<'a>(text: &'a str, label: &Regex) -> impl Iterator<Item = &'a str> {
    label.find_iter(text).map(move |m| {
        let end = floor_char_boundary(text, (m.end() + LABEL_WINDOW).min(text.len()));
        let w = &text[m.end()..end];
        // Only look for the next label after the first number, so "Drying temperature: 55°C"
        // is not cut off at its own "temperature".
        let first_digit = w.find(|c: char| c.is_ascii_digit()).unwrap_or(w.len());
        let cut = [
            w.find('\n'),
            NEXT_LABEL.find_at(w, first_digit).map(|x| x.start()),
        ]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(w.len());
        &w[..cut]
    })
}

static NEXT_LABEL: LazyLock<Regex> = re!(r"(?i)temp|speed|dry");

pub(crate) fn floor_char_boundary(s: &str, mut i: usize) -> usize {
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Lowest low and highest high of every range after the label, so
/// "35-45°C (Cool Plate), 55-65°C (PEI)" becomes 35-65.
fn temp_range_after(text: &str, label: &Regex) -> Option<(i16, i16)> {
    after_labels(text, label).find_map(|w| {
        let ranges: Vec<(i16, i16)> = TEMP_RANGE
            .captures_iter(w)
            .filter_map(|c| Some((c[1].parse().ok()?, c[2].parse().ok()?)))
            .collect();
        if !ranges.is_empty() {
            let lo = ranges.iter().map(|r| r.0.min(r.1)).min()?;
            let hi = ranges.iter().map(|r| r.0.max(r.1)).max()?;
            return Some((lo, hi));
        }
        let c = TEMP_SINGLE.captures(w)?;
        let v: i16 = c[1].parse().ok()?;
        Some((v, v))
    })
}

fn speed_after(text: &str) -> Option<(Option<i16>, i16)> {
    after_labels(text, &SPEED_LABEL).find_map(|w| {
        if let Some(c) = SPEED_RANGE.captures(w) {
            return Some((Some(c[1].parse().ok()?), c[2].parse().ok()?));
        }
        let c = SPEED_SINGLE.captures(w)?;
        Some((None, c[1].parse().ok()?))
    })
}

fn drying_after(text: &str) -> Option<(i16, i32)> {
    after_labels(text, &DRY_LABEL).find_map(|w| {
        let c = DRY_COND.captures(w)?;
        let temp: i16 = c[1].parse().ok()?;
        let hours: f32 = c[2].parse().ok()?;
        Some((temp, (hours * 60.0).round() as i32))
    })
}

pub fn detect_diameter(text: &str) -> Option<Decimal> {
    let caps: Vec<_> = DIAMETER.captures_iter(text).collect();
    // Prefer 1.75 when a page lists several (variant pickers often list both).
    let pick = caps.iter().find(|c| &c[1] == "1.75").or(caps.first())?;
    pick[1].parse().ok()
}

pub fn detect_weight_g(text: &str) -> Option<Decimal> {
    if let Some(c) = WEIGHT_KG.captures(text) {
        let kg: Decimal = c[1].parse().ok()?;
        if kg > Decimal::ZERO && kg <= Decimal::from(10) {
            return Some(kg * Decimal::from(1000));
        }
    }
    let g: Decimal = WEIGHT_G.captures(text)?[1].parse().ok()?;
    (g >= Decimal::from(100) && g <= Decimal::from(5000)).then_some(g)
}

/// Runs every regex over `text`. Used for titles, flattened spec tables and page text.
pub fn extract(text: &str) -> Draft {
    let mut d = Draft {
        // A "Material: ..." line beats a mention anywhere else in the text.
        material_code: text
            .lines()
            .filter(|l| {
                let l = l.trim_start().to_ascii_lowercase();
                l.starts_with("material")
                    || l.starts_with("filament type")
                    || l.starts_with("type:")
            })
            .find_map(detect_material)
            .or_else(|| detect_material(text))
            .map(String::from),
        diameter_mm: detect_diameter(text),
        nominal_weight_g: detect_weight_g(text),
        ..Default::default()
    };
    if let Some((lo, hi)) = temp_range_after(text, &NOZZLE_LABEL) {
        d.nozzle_temp_min_c = Some(lo);
        d.nozzle_temp_max_c = Some(hi);
    }
    if let Some((lo, hi)) = temp_range_after(text, &BED_LABEL) {
        d.bed_temp_min_c = Some(lo);
        d.bed_temp_max_c = Some(hi);
    }
    if let Some((lo, hi)) = speed_after(text) {
        d.print_speed_min_mm_s = lo;
        d.print_speed_max_mm_s = Some(hi);
    }
    if let Some((t, m)) = drying_after(text) {
        d.drying_temp_c = Some(t);
        d.drying_time_min = Some(m);
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materials_prefer_specific_variants() {
        assert_eq!(
            detect_material("Bambu Lab PETG-CF Filament"),
            Some("PETG-CF")
        );
        assert_eq!(detect_material("eSUN PLA+ 1.75mm"), Some("PLA+"));
        assert_eq!(detect_material("Silk Shiny PLA Gold"), Some("PLA-SILK"));
        assert_eq!(detect_material("Polymaker PolyLite ASA"), Some("ASA"));
        assert_eq!(detect_material("SUNLU TPU 95A"), Some("TPU"));
        assert_eq!(detect_material("Prusament PETG Jet Black"), Some("PETG"));
        assert_eq!(detect_material("HATCHBOX PLA Black"), Some("PLA"));
        assert_eq!(detect_material("random words"), None);
    }

    #[test]
    fn temperature_ranges() {
        let d = extract("Printing Temperature: 190-230°C\nBed Temperature: 45 – 65 °C");
        assert_eq!(
            (d.nozzle_temp_min_c, d.nozzle_temp_max_c),
            (Some(190), Some(230))
        );
        assert_eq!((d.bed_temp_min_c, d.bed_temp_max_c), (Some(45), Some(65)));

        let d = extract("Nozzle temperature 240 ℃, Hot bed temperature 70℃");
        assert_eq!(
            (d.nozzle_temp_min_c, d.nozzle_temp_max_c),
            (Some(240), Some(240))
        );
        assert_eq!(d.bed_temp_min_c, Some(70));

        let d = extract("Build Plate Temperature: 35-45°C (Cool Plate), 55-65°C (Textured PEI)");
        assert_eq!((d.bed_temp_min_c, d.bed_temp_max_c), (Some(35), Some(65)));

        let d = extract("Printing Temperature: 230-250°C, Bed Temperature: 70-80°C");
        assert_eq!(
            (d.nozzle_temp_min_c, d.nozzle_temp_max_c),
            (Some(230), Some(250))
        );
        assert_eq!((d.bed_temp_min_c, d.bed_temp_max_c), (Some(70), Some(80)));
    }

    #[test]
    fn speed_and_drying() {
        let d = extract("Print speed: 50-300 mm/s. Drying settings: 55°C, 8h");
        assert_eq!(
            (d.print_speed_min_mm_s, d.print_speed_max_mm_s),
            (Some(50), Some(300))
        );
        assert_eq!((d.drying_temp_c, d.drying_time_min), (Some(55), Some(480)));

        let d = extract("Recommended print speed ≤ 200 mm/s. Dry at 65 °C for 4-6 hours");
        assert_eq!(
            (d.print_speed_min_mm_s, d.print_speed_max_mm_s),
            (None, Some(200))
        );
        assert_eq!((d.drying_temp_c, d.drying_time_min), (Some(65), Some(240)));

        let d = extract("Drying temperature: 55°C for 8 hours");
        assert_eq!((d.drying_temp_c, d.drying_time_min), (Some(55), Some(480)));
    }

    #[test]
    fn diameter_and_weight() {
        assert_eq!(
            detect_diameter("Available in 2.85mm and 1.75 mm"),
            Some(Decimal::new(175, 2))
        );
        assert_eq!(detect_weight_g("PLA 1kg Spool"), Some(Decimal::from(1000)));
        assert_eq!(detect_weight_g("PETG 0.5 kg"), Some(Decimal::from(500)));
        assert_eq!(detect_weight_g("TPU 750g spool"), Some(Decimal::from(750)));
        assert_eq!(detect_weight_g("Ships in 2 days"), None);
    }

    #[test]
    fn non_ascii_text_does_not_panic() {
        let s = format!("Printing temperature {}", "é".repeat(200));
        let _ = extract(&s);
    }
}
