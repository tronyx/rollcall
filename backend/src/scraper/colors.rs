//! Color name detection and a best-guess swatch hex for common names.

/// Longer names first so "light blue" wins over "blue".
const COLORS: &[(&str, &str)] = &[
    ("charcoal", "#36454F"),
    ("jade white", "#F4F4F0"),
    ("ivory white", "#FFFFF0"),
    ("bone white", "#E3DAC9"),
    ("cool white", "#F5F7FA"),
    ("light blue", "#ADD8E6"),
    ("sky blue", "#87CEEB"),
    ("navy blue", "#1F2A44"),
    ("royal blue", "#4169E1"),
    ("dark blue", "#00008B"),
    ("cyan", "#00B7EB"),
    ("turquoise", "#40E0D0"),
    ("teal", "#008080"),
    ("light gray", "#D3D3D3"),
    ("light grey", "#D3D3D3"),
    ("dark gray", "#545454"),
    ("dark grey", "#545454"),
    ("space gray", "#717378"),
    ("lime green", "#32CD32"),
    ("olive green", "#6B8E23"),
    ("dark green", "#006400"),
    ("mint", "#98FF98"),
    ("forest green", "#228B22"),
    ("bambu green", "#00AE42"),
    ("hot pink", "#FF69B4"),
    ("magenta", "#FF00FF"),
    ("lavender", "#B57EDC"),
    ("violet", "#8F00FF"),
    ("burgundy", "#800020"),
    ("maroon", "#800000"),
    ("coral", "#FF7F50"),
    ("peach", "#FFCBA4"),
    ("beige", "#F5F5DC"),
    ("tan", "#D2B48C"),
    ("khaki", "#C3B091"),
    ("cream", "#FFFDD0"),
    ("bronze", "#CD7F32"),
    ("copper", "#B87333"),
    ("gold", "#D4AF37"),
    ("silver", "#C0C0C0"),
    ("black", "#000000"),
    ("white", "#FFFFFF"),
    ("grey", "#808080"),
    ("gray", "#808080"),
    ("red", "#D32F2F"),
    ("orange", "#FF8C00"),
    ("yellow", "#FFD700"),
    ("green", "#2E8B57"),
    ("blue", "#1E63D6"),
    ("purple", "#7B2FBE"),
    ("pink", "#F4A6C1"),
    ("brown", "#7B4A2A"),
    ("transparent", "#E8F0F2"),
    ("clear", "#E8F0F2"),
    ("natural", "#EFE8D8"),
];

/// Best-guess hex for a color name (matches the most specific known phrase).
pub fn hex_for(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    COLORS
        .iter()
        .filter(|(n, _)| contains_word(&lower, n))
        .max_by_key(|(n, _)| n.len())
        .map(|(_, h)| *h)
}

/// Finds a color phrase in a product title, keeping the word before a
/// basic color (e.g. "Galaxy Black", "Matte Charcoal").
pub fn find_in_title(title: &str) -> Option<String> {
    let words: Vec<&str> = title
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | '|' | '(' | ')' | '/'))
        .filter(|w| !w.is_empty())
        .collect();
    let idx = words.iter().rposition(|w| hex_for(w).is_some())?;
    let prev = idx.checked_sub(1).map(|i| words[i]).filter(|p| {
        p.chars().all(|c| c.is_alphabetic())
            && p.chars().next().is_some_and(char::is_uppercase)
            && !STOP_WORDS.contains(&p.to_ascii_lowercase().as_str())
    });
    Some(match prev {
        Some(p) => format!("{p} {}", words[idx]),
        None => words[idx].to_string(),
    })
}

/// Words that precede colors in titles but are not part of the color name.
const STOP_WORDS: &[&str] = &[
    "pla", "petg", "abs", "asa", "tpu", "filament", "spool", "basic", "matte", "silk", "plus",
    "pro", "and", "with", "the", "for", "printer", "hf", "cf", "nylon", "kg",
];

fn contains_word(haystack: &str, needle: &str) -> bool {
    haystack.match_indices(needle).any(|(i, _)| {
        let before = haystack[..i].chars().next_back();
        let after = haystack[i + needle.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_lookup() {
        assert_eq!(hex_for("Charcoal Black"), Some("#36454F"));
        assert_eq!(hex_for("Light Blue"), Some("#ADD8E6"));
        assert_eq!(hex_for("Galaxy Black"), Some("#000000"));
        assert_eq!(hex_for("Redwood"), None);
    }

    #[test]
    fn title_colors() {
        assert_eq!(
            find_in_title("HATCHBOX PLA 1.75mm Filament, Black, 1kg").as_deref(),
            Some("Black")
        );
        assert_eq!(
            find_in_title("eSUN PLA+ Galaxy Blue 1kg").as_deref(),
            Some("Galaxy Blue")
        );
        assert_eq!(
            find_in_title("Bambu Lab PLA Basic Jade White").as_deref(),
            Some("Jade White")
        );
        assert_eq!(find_in_title("Polymaker Matte Red").as_deref(), Some("Red"));
    }
}
