//! All SQL lives here. Every query uses sqlx's compile-time checked macros;
//! run `cargo sqlx prepare` after changing one so CI can build offline.

pub mod filaments;
pub mod humidity;
pub mod lookups;
pub mod settings;
pub mod spools;
pub mod stats;
pub mod tuned;

/// Turns user text into an ILIKE pattern, escaping wildcards it contains.
pub(crate) fn contains_pattern(input: &str) -> Option<String> {
    let t = input.trim();
    if t.is_empty() {
        return None;
    }
    let escaped = t
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    Some(format!("%{escaped}%"))
}
