//! Amazon product pages. Amazon often serves a robot check to non-browser
//! clients; `is_blocked` detects that so the pipeline can fall back.

use super::spec_table::clean_text;
use crate::scraper::draft::Draft;
use scraper::{Html, Selector};
use std::sync::LazyLock;

macro_rules! sel {
    ($s:expr) => {
        LazyLock::new(|| Selector::parse($s).unwrap())
    };
}

static TITLE: LazyLock<Selector> = sel!("#productTitle");
static BYLINE: LazyLock<Selector> = sel!("#bylineInfo");
static IMAGE: LazyLock<Selector> = sel!("#landingImage, #imgBlkFront");
static PRICE: LazyLock<Selector> =
    sel!("#corePrice_feature_div .a-offscreen, .a-price .a-offscreen");
static SPEC_ROWS: LazyLock<Selector> = sel!(
    "#productOverview_feature_div tr, #productDetails_techSpec_section_1 tr, \
     #productDetails_detailBullets_sections1 tr, #technicalSpecifications_section_1 tr"
);
static CELLS: LazyLock<Selector> = sel!("th, td");
static DETAIL_BULLETS: LazyLock<Selector> = sel!("#detailBullets_feature_div li");
static FEATURE_BULLETS: LazyLock<Selector> = sel!("#feature-bullets li");
static CAPTCHA: LazyLock<Selector> = sel!("form[action*='validateCaptcha']");

pub fn matches_host(host: &str) -> bool {
    let h = host.to_ascii_lowercase();
    h.split('.').any(|part| part == "amazon") || h == "amzn.to" || h == "a.co"
}

pub fn is_blocked(doc: &Html) -> bool {
    doc.select(&CAPTCHA).next().is_some()
}

/// Returns the draft plus spec/bullet text for the regex pass.
pub fn extract(doc: &Html) -> (Draft, String) {
    let first = |s: &Selector| {
        doc.select(s)
            .next()
            .map(clean_text)
            .filter(|t| !t.is_empty())
    };
    let mut d = Draft {
        title: first(&TITLE),
        brand_name: first(&BYLINE),
        image_url: doc.select(&IMAGE).next().and_then(|img| {
            let e = img.value();
            e.attr("data-old-hires")
                .filter(|s| !s.is_empty())
                .or_else(|| e.attr("src"))
                .map(String::from)
        }),
        ..Default::default()
    };
    if let Some(p) = first(&PRICE) {
        let digits: String = p
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        d.price = digits.parse().ok();
        if p.starts_with('$') {
            d.currency = Some("USD".into());
        }
    }

    let mut text = String::new();
    for row in doc.select(&SPEC_ROWS) {
        let cells: Vec<String> = row.select(&CELLS).map(clean_text).collect();
        if let [k, v] = cells.as_slice() {
            text.push_str(&format!("{k}: {v}\n"));
            match k.to_ascii_lowercase().as_str() {
                "brand" | "manufacturer" => d.brand_name = Some(v.clone()),
                "color" => d.color_name = d.color_name.take().or(Some(v.clone())),
                _ => {}
            }
        }
    }
    for li in doc
        .select(&DETAIL_BULLETS)
        .chain(doc.select(&FEATURE_BULLETS))
    {
        text.push_str(&clean_text(li));
        text.push('\n');
    }
    (d, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_page() {
        let html = include_str!("../../../tests/fixtures/amazon_product.html");
        let doc = Html::parse_document(html);
        assert!(!is_blocked(&doc));
        let (d, text) = extract(&doc);
        assert_eq!(d.brand_name.as_deref(), Some("SUNLU"));
        assert_eq!(d.color_name.as_deref(), Some("Black"));
        assert!(d.title.unwrap().contains("PETG"));
        assert_eq!(
            d.image_url.as_deref(),
            Some("https://m.media-amazon.com/images/I/big.jpg")
        );
        assert!(text.contains("Printing Temperature"));
    }

    #[test]
    fn robot_check() {
        let doc = Html::parse_document(
            r#"<form method="get" action="/errors/validateCaptcha"><input name="amzn"></form>"#,
        );
        assert!(is_blocked(&doc));
        assert!(matches_host("www.amazon.co.uk"));
        assert!(!matches_host("notamazon.com"));
    }
}
