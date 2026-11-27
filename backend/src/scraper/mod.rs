//! Product page metadata extraction for `POST /api/filaments/parse-url`.
//!
//! The pipeline runs extractors from most to least trustworthy and fills each
//! field from the first one that finds it:
//!
//! 1. Shopify product JSON (stores that expose `/products/<handle>.js`)
//! 2. schema.org JSON-LD `Product`
//! 3. Site adapters (Amazon)
//! 4. Generic spec tables / definition lists / "Key: Value" bullets
//! 5. OpenGraph and meta tags
//! 6. Regex over the product title
//! 7. Regex over visible page text
//! 8. Optional LLM pass (only if configured and essentials are still missing)
//! 9. URL and domain hints
//! 10. Derived values (swatch hex from the color name)
//!
//! It never fails because data is missing: the caller gets whatever was found,
//! the list of missing fields, and human-readable warnings.

pub mod brands;
pub mod colors;
pub mod draft;
pub mod fetch;
pub mod heuristics;
pub mod llm;
pub mod sites;
pub mod structured;

use crate::{
    config::{LlmConfig, ScraperConfig},
    error::{AppError, AppResult},
};
use draft::{Draft, FieldSource};
use scraper::Html;
use serde::Serialize;
use std::collections::BTreeMap;
use url::Url;

const MATERIAL_CODES: &[&str] = &[
    "PLA",
    "PLA+",
    "PLA-SILK",
    "PLA-MATTE",
    "PLA-CF",
    "PETG",
    "PETG-CF",
    "ABS",
    "ASA",
    "TPU",
    "PA",
    "PA-CF",
    "PC",
    "HIPS",
    "PVA",
];

#[derive(Debug, Serialize)]
pub struct ParseResult {
    pub url: String,
    pub final_url: Option<String>,
    /// The page was downloaded (even if it turned out to be a bot wall).
    pub fetched: bool,
    /// The site served a captcha / access-denied page.
    pub blocked: bool,
    /// True when brand, material, color and nozzle/bed temps were all found.
    pub complete: bool,
    pub draft: Draft,
    /// Which extractor supplied each field.
    pub sources: BTreeMap<String, FieldSource>,
    pub missing: Vec<&'static str>,
    pub warnings: Vec<String>,
}

pub struct ScraperService {
    client: reqwest::Client,
    cfg: ScraperConfig,
    llm: Option<llm::LlmExtractor>,
}

/// Everything pulled from the HTML in one synchronous pass (`Html` is not `Send`).
struct HtmlLayers {
    layers: Vec<(FieldSource, Draft)>,
    page_text: String,
    blocked: bool,
    shopify: bool,
}

impl ScraperService {
    pub fn new(cfg: &ScraperConfig, llm: Option<LlmConfig>) -> anyhow::Result<Self> {
        Ok(Self {
            client: fetch::build_client(cfg)?,
            cfg: cfg.clone(),
            llm: llm.map(llm::LlmExtractor::new).transpose()?,
        })
    }

    pub async fn parse(&self, raw_url: &str) -> AppResult<ParseResult> {
        let url = Url::parse(raw_url.trim())
            .map_err(|_| AppError::BadRequest("that doesn't look like a valid URL".into()))?;
        let target = fetch::validate_target(&url, self.cfg.allow_private_hosts)
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;

        let mut warnings = Vec::new();
        let mut draft = Draft::default();
        let mut sources = BTreeMap::new();
        let mut final_url = url.clone();
        let mut fetched = false;
        let mut blocked = false;
        let mut page_text = String::new();
        let mut html_layers = Vec::new();

        let download = match target {
            fetch::Target::Ok => fetch::get(&self.client, &url, self.cfg.max_body_bytes)
                .await
                .map_err(|e| e.to_string()),
            fetch::Target::Unresolvable(e) => Err(e),
        };
        match download {
            Ok(page) => {
                fetched = true;
                final_url = page.final_url.clone();
                let mut extracted = extract_html(&page.final_url, &page.body);
                blocked = extracted.blocked || fetch::looks_blocked(page.status, &page.body);
                if blocked {
                    warnings.push(
                        "The site blocked automated access, so only details from the link itself \
                         were filled in. Please fill in the rest."
                            .into(),
                    );
                } else if !page.status.is_success() {
                    warnings.push(format!("The page returned HTTP {}.", page.status.as_u16()));
                }
                if extracted.shopify {
                    if let Some(layer) = self.shopify_layer(&page.final_url).await {
                        html_layers.push(layer);
                    }
                }
                if !blocked {
                    html_layers.append(&mut extracted.layers);
                    page_text = extracted.page_text;
                }
            }
            Err(e) => warnings.push(format!(
                "Couldn't download the page ({e}), so only details from the link were filled in."
            )),
        }

        // Structured layers first, then the title-based pass on whatever title we have.
        let (before_title, after_title): (Vec<_>, Vec<_>) = html_layers
            .into_iter()
            .partition(|(s, _)| *s < FieldSource::Title);
        for (src, layer) in before_title {
            draft.fill_from(layer, src, &mut sources);
        }
        if let Some(title) = draft.title.clone() {
            draft.fill_from(title_layer(&title), FieldSource::Title, &mut sources);
        }
        for (src, layer) in after_title {
            draft.fill_from(layer, src, &mut sources);
        }

        if let Some(llm) = &self.llm {
            if !draft.has_essentials() && !page_text.trim().is_empty() {
                let text = truncate(&page_text, llm::MAX_PAGE_CHARS);
                match llm.extract(final_url.as_str(), text).await {
                    Ok(layer) => draft.fill_from(layer, FieldSource::Llm, &mut sources),
                    Err(e) => {
                        tracing::warn!(error = %e, "LLM extraction failed");
                        warnings.push("The AI-assisted extraction step failed; showing what the page parser found.".into());
                    }
                }
            }
        }

        draft.fill_from(url_layer(&final_url), FieldSource::Url, &mut sources);
        if final_url != url {
            draft.fill_from(url_layer(&url), FieldSource::Url, &mut sources);
        }
        finalize(&mut draft, &mut sources);

        let missing = draft.missing();
        let complete = draft.has_essentials();
        if !complete && !blocked && fetched {
            warnings.push("Some details weren't on the page. Check the highlighted fields.".into());
        }
        Ok(ParseResult {
            url: url.to_string(),
            final_url: fetched.then(|| final_url.to_string()),
            fetched,
            blocked,
            complete,
            draft,
            sources,
            missing,
            warnings,
        })
    }

    async fn shopify_layer(&self, page_url: &Url) -> Option<(FieldSource, Draft)> {
        let json_url = sites::shopify::json_url(page_url)?;
        let resp = fetch::get(&self.client, &json_url, self.cfg.max_body_bytes)
            .await
            .ok()?;
        if !resp.status.is_success() {
            return None;
        }
        let product: sites::shopify::Product = serde_json::from_str(&resp.body).ok()?;
        let (mut d, text) = sites::shopify::extract(&product, page_url);
        let specs = heuristics::extract(&text);
        d.fill_from(specs, FieldSource::ShopifyJson, &mut BTreeMap::new());
        Some((FieldSource::ShopifyJson, d))
    }
}

/// Runs every HTML-based extractor. Pure and synchronous, so it is easy to
/// test against saved fixture pages.
fn extract_html(url: &Url, body: &str) -> HtmlLayers {
    let doc = Html::parse_document(body);
    let host = url.host_str().unwrap_or_default();
    let mut layers = Vec::new();
    let mut scratch = BTreeMap::new();
    let mut blocked = false;

    let (mut ld, ld_text) = structured::json_ld(&doc);
    ld.fill_from(
        heuristics::extract(&ld_text),
        FieldSource::JsonLd,
        &mut scratch,
    );
    layers.push((FieldSource::JsonLd, ld));

    if sites::amazon::matches_host(host) {
        blocked = sites::amazon::is_blocked(&doc);
        let (mut d, text) = sites::amazon::extract(&doc);
        d.fill_from(
            heuristics::extract(&text),
            FieldSource::SiteAdapter,
            &mut scratch,
        );
        layers.push((FieldSource::SiteAdapter, d));
    }

    let pairs = sites::spec_table::pairs(&doc);
    let (mut table, table_text) = sites::spec_table::extract(&pairs);
    table.fill_from(
        heuristics::extract(&table_text),
        FieldSource::SpecTable,
        &mut scratch,
    );
    layers.push((FieldSource::SpecTable, table));

    layers.push((FieldSource::MetaTags, structured::meta_tags(&doc)));

    let page_text = visible_text(&doc);
    let mut text_layer = heuristics::extract(&page_text);
    // Body text mentions other materials ("also try our PETG"); trust the title for material.
    text_layer.material_code = None;
    text_layer.nominal_weight_g = None;
    layers.push((FieldSource::PageText, text_layer));

    HtmlLayers {
        layers,
        page_text,
        blocked,
        shopify: sites::shopify::looks_like_shopify(body),
    }
}

/// The whole pipeline minus network calls (no Shopify JSON, no LLM). Used by
/// tests and handy for debugging a saved page.
pub fn extract_offline(url: &Url, html: &str) -> (Draft, BTreeMap<String, FieldSource>) {
    let layers = extract_html(url, html);
    let mut d = Draft::default();
    let mut src = BTreeMap::new();
    let (a, b): (Vec<_>, Vec<_>) = layers
        .layers
        .into_iter()
        .partition(|(s, _)| *s < FieldSource::Title);
    for (s, l) in a {
        d.fill_from(l, s, &mut src);
    }
    if let Some(t) = d.title.clone() {
        d.fill_from(title_layer(&t), FieldSource::Title, &mut src);
    }
    for (s, l) in b {
        d.fill_from(l, s, &mut src);
    }
    d.fill_from(url_layer(url), FieldSource::Url, &mut src);
    finalize(&mut d, &mut src);
    (d, src)
}

fn title_layer(title: &str) -> Draft {
    let mut d = heuristics::extract(title);
    d.brand_name = brands::find_in_text(title).map(String::from);
    d.color_name = colors::find_in_title(title);
    d
}

fn url_layer(url: &Url) -> Draft {
    let slug: String = url
        .path_segments()
        .map(|segs| segs.collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
        .replace(['-', '_', '+'], " ");
    let titled: String = slug
        .split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().chain(c).collect::<String>())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ");
    Draft {
        brand_name: url.host_str().and_then(brands::from_host).map(String::from),
        material_code: heuristics::detect_material(&titled).map(String::from),
        color_name: colors::find_in_title(&titled),
        ..Default::default()
    }
}

/// Normalization and derived fields after all extractors ran.
fn finalize(d: &mut Draft, sources: &mut BTreeMap<String, FieldSource>) {
    d.sanitize();
    if let Some(b) = d.brand_name.take() {
        let b = brands::canonicalize(&b);
        d.brand_name = (!b.is_empty()).then_some(b);
    }
    // Structured sources sometimes give "Polylactic Acid (PLA)"; reduce to a known code.
    if let Some(code) = d.material_code.take() {
        let upper = code.trim().to_ascii_uppercase();
        d.material_code = if MATERIAL_CODES.contains(&upper.as_str()) {
            Some(upper)
        } else {
            heuristics::detect_material(&code).map(String::from)
        };
    }
    if d.color_hex.is_none() {
        if let Some(hex) = d.color_name.as_deref().and_then(colors::hex_for) {
            d.color_hex = Some(hex.to_string());
            sources.insert("color_hex".into(), FieldSource::Derived);
        }
    }
    if let Some(hex) = d.color_hex.take() {
        d.color_hex = crate::models::normalize_hex(&hex);
    }
    // A short title makes a good product name once the brand is removed;
    // long marketplace titles are left for the user to shorten.
    if d.product_name.is_none() {
        if let Some(title) = d.title.as_deref().filter(|t| t.len() <= 60) {
            let name = match d.brand_name.as_deref() {
                Some(b)
                    if title
                        .to_ascii_lowercase()
                        .starts_with(&b.to_ascii_lowercase()) =>
                {
                    title[b.len()..].trim()
                }
                _ => title,
            };
            if !name.is_empty() {
                d.product_name = Some(name.to_string());
                sources.insert("product_name".into(), FieldSource::Derived);
            }
        }
    }
    if let Some(c) = d.currency.take() {
        let c = c.trim().to_ascii_uppercase();
        d.currency = (c.len() == 3 && c.chars().all(|ch| ch.is_ascii_alphabetic())).then_some(c);
    }
}

fn visible_text(doc: &Html) -> String {
    const MAX: usize = 300_000;
    let mut out = String::new();
    for node in doc.root_element().descendants() {
        let Some(text) = node.value().as_text() else {
            continue;
        };
        let hidden = node.ancestors().any(|a| {
            a.value().as_element().is_some_and(|e| {
                matches!(
                    e.name(),
                    "script" | "style" | "noscript" | "template" | "svg"
                )
            })
        });
        let t = text.trim();
        if hidden || t.is_empty() {
            continue;
        }
        out.push_str(t);
        out.push('\n');
        if out.len() > MAX {
            break;
        }
    }
    out
}

fn truncate(s: &str, max: usize) -> &str {
    &s[..heuristics::floor_char_boundary(s, s.len().min(max))]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(url: &str, html: &str) -> (Draft, BTreeMap<String, FieldSource>) {
        extract_offline(&Url::parse(url).unwrap(), html)
    }

    #[test]
    fn amazon_fixture_end_to_end() {
        let (d, src) = run(
            "https://www.amazon.com/dp/B000TEST",
            include_str!("../../tests/fixtures/amazon_product.html"),
        );
        assert_eq!(d.brand_name.as_deref(), Some("SUNLU"));
        assert_eq!(d.material_code.as_deref(), Some("PETG"));
        assert_eq!(d.color_name.as_deref(), Some("Black"));
        assert_eq!(d.color_hex.as_deref(), Some("#000000"));
        assert_eq!(
            (d.nozzle_temp_min_c, d.nozzle_temp_max_c),
            (Some(230), Some(250))
        );
        assert_eq!((d.bed_temp_min_c, d.bed_temp_max_c), (Some(70), Some(80)));
        assert_eq!(d.print_speed_max_mm_s, Some(100));
        assert_eq!((d.drying_temp_c, d.drying_time_min), (Some(65), Some(360)));
        assert_eq!(d.diameter_mm, Some(rust_decimal::Decimal::new(175, 2)));
        assert_eq!(d.price, Some(rust_decimal::Decimal::new(1799, 2)));
        assert!(d.has_essentials());
        assert_eq!(src.get("brand_name"), Some(&FieldSource::SiteAdapter));
        assert_eq!(src.get("color_hex"), Some(&FieldSource::Derived));
    }

    #[test]
    fn manufacturer_fixture_end_to_end() {
        let (d, src) = run(
            "https://store.example-filament.com/products/pla-basic",
            include_str!("../../tests/fixtures/manufacturer_jsonld.html"),
        );
        assert_eq!(d.brand_name.as_deref(), Some("Bambu Lab"));
        assert_eq!(d.material_code.as_deref(), Some("PLA"));
        assert_eq!(d.color_name.as_deref(), Some("Jade White"));
        assert_eq!(d.color_hex.as_deref(), Some("#F4F4F0"));
        assert_eq!(
            (d.nozzle_temp_min_c, d.nozzle_temp_max_c),
            (Some(190), Some(230))
        );
        assert_eq!((d.bed_temp_min_c, d.bed_temp_max_c), (Some(35), Some(65)));
        assert_eq!(d.nominal_weight_g, Some(rust_decimal::Decimal::from(1000)));
        assert_eq!(d.product_name.as_deref(), Some("PLA Basic"));
        assert_eq!(src.get("title"), Some(&FieldSource::JsonLd));
    }

    #[test]
    fn blocked_page_falls_back_to_url() {
        let url = Url::parse("https://us.store.bambulab.com/products/petg-hf-black").unwrap();
        let d = url_layer(&url);
        assert_eq!(d.brand_name.as_deref(), Some("Bambu Lab"));
        assert_eq!(d.material_code.as_deref(), Some("PETG"));
        assert_eq!(d.color_name.as_deref(), Some("Black"));
    }
}
