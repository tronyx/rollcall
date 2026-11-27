//! Brand names from domains and canonical spelling of known brands.

/// (alias substring in lowercase, canonical name, store domains)
const KNOWN: &[(&str, &str, &[&str])] = &[
    (
        "bambu",
        "Bambu Lab",
        &[
            "bambulab.com",
            "store.bambulab.com",
            "us.store.bambulab.com",
        ],
    ),
    ("prusament", "Prusament", &["prusa3d.com", "prusament.com"]),
    (
        "polymaker",
        "Polymaker",
        &["polymaker.com", "us.polymaker.com"],
    ),
    ("esun", "eSUN", &["esun3d.com", "esun3dstore.com"]),
    ("hatchbox", "HATCHBOX", &["hatchbox3d.com"]),
    ("sunlu", "SUNLU", &["sunlu.com", "store.sunlu.com"]),
    ("elegoo", "ELEGOO", &["elegoo.com", "us.elegoo.com"]),
    ("overture", "OVERTURE", &["overture3d.com"]),
    (
        "creality",
        "Creality",
        &["creality.com", "store.creality.com"],
    ),
    (
        "anycubic",
        "Anycubic",
        &["anycubic.com", "store.anycubic.com"],
    ),
    ("inland", "Inland", &[]),
    ("eryone", "ERYONE", &["eryone3d.com"]),
    ("protopasta", "Proto-pasta", &["proto-pasta.com"]),
    ("proto-pasta", "Proto-pasta", &[]),
    ("atomic", "Atomic Filament", &["atomicfilament.com"]),
    ("fiberlogy", "Fiberlogy", &["fiberlogy.com"]),
    ("fillamentum", "Fillamentum", &["fillamentum.com"]),
    ("3djake", "3DJake", &["3djake.com"]),
    ("kingroon", "Kingroon", &["kingroon.com"]),
    ("jayo", "JAYO", &[]),
    ("geeetech", "Geeetech", &["geeetech.com"]),
    ("amolen", "AMOLEN", &["amolen.com"]),
    ("siraya", "Siraya Tech", &["siraya.tech"]),
    ("matterhackers", "MatterHackers", &[]),
];

/// Brand for a store domain, e.g. `us.store.bambulab.com` → "Bambu Lab".
pub fn from_host(host: &str) -> Option<&'static str> {
    let host = host.trim_start_matches("www.").to_ascii_lowercase();
    KNOWN.iter().find_map(|(_, canon, domains)| {
        domains
            .iter()
            .any(|d| host == *d || host.ends_with(&format!(".{d}")))
            .then_some(*canon)
    })
}

/// Normalizes spelling ("bambu lab" → "Bambu Lab"); unknown brands pass through trimmed.
pub fn canonicalize(raw: &str) -> String {
    let lower = raw.trim().to_ascii_lowercase();
    // Amazon bylines: "Visit the SUNLU Store", "Brand: SUNLU".
    let cleaned = lower
        .trim_start_matches("visit the ")
        .trim_end_matches(" store")
        .trim_start_matches("brand:")
        .trim();
    KNOWN
        .iter()
        .find(|(alias, _, _)| cleaned.contains(alias))
        .map(|(_, canon, _)| canon.to_string())
        .unwrap_or_else(|| {
            let original = raw.trim();
            let start = original.len()
                - original
                    .trim_start_matches(|c: char| c.is_whitespace())
                    .len();
            // Keep the user's casing but strip the same prefixes/suffixes.
            let s = &original[start..];
            let s = strip_ci_prefix(s, "visit the ");
            let s = strip_ci_suffix(s, " store");
            strip_ci_prefix(s, "brand:").trim().to_string()
        })
}

/// First known brand mentioned anywhere in `text` (used on titles).
pub fn find_in_text(text: &str) -> Option<&'static str> {
    let lower = text.to_ascii_lowercase();
    KNOWN
        .iter()
        .filter(|(alias, _, _)| alias.len() >= 4)
        .find(|(alias, _, _)| lower.contains(alias))
        .map(|(_, canon, _)| *canon)
}

fn strip_ci_prefix<'a>(s: &'a str, p: &str) -> &'a str {
    match s.get(..p.len()) {
        Some(head) if head.eq_ignore_ascii_case(p) => &s[p.len()..],
        _ => s,
    }
}

fn strip_ci_suffix<'a>(s: &'a str, p: &str) -> &'a str {
    match s
        .len()
        .checked_sub(p.len())
        .and_then(|i| s.get(i..).map(|t| (i, t)))
    {
        Some((i, tail)) if tail.eq_ignore_ascii_case(p) => &s[..i],
        _ => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_and_names() {
        assert_eq!(from_host("us.store.bambulab.com"), Some("Bambu Lab"));
        assert_eq!(from_host("www.prusa3d.com"), Some("Prusament"));
        assert_eq!(from_host("amazon.com"), None);
        assert_eq!(canonicalize("Visit the SUNLU Store"), "SUNLU");
        assert_eq!(canonicalize("bambu lab"), "Bambu Lab");
        assert_eq!(canonicalize("Brand: Acme Filaments"), "Acme Filaments");
        assert_eq!(
            find_in_text("HATCHBOX PLA 3D Printer Filament"),
            Some("HATCHBOX")
        );
    }
}
