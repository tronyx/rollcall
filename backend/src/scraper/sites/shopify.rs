//! Shopify stores (Polymaker, eSUN, HATCHBOX and many others) expose product
//! JSON at `/products/<handle>.js`, which is more reliable than their HTML and
//! includes the selected variant (usually the color).

use crate::scraper::draft::Draft;
use rust_decimal::Decimal;
use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct Product {
    pub title: String,
    pub vendor: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub featured_image: Option<String>,
    #[serde(default)]
    pub variants: Vec<Variant>,
    #[serde(default)]
    pub options: Vec<ProductOption>,
}

#[derive(Debug, Deserialize)]
pub struct Variant {
    pub id: u64,
    pub title: String,
    /// Price in minor units (cents).
    pub price: Option<i64>,
    #[serde(default)]
    pub option1: Option<String>,
    #[serde(default)]
    pub option2: Option<String>,
    #[serde(default)]
    pub option3: Option<String>,
    #[serde(default)]
    pub featured_image: Option<VariantImage>,
}

#[derive(Debug, Deserialize)]
pub struct VariantImage {
    pub src: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ProductOption {
    Named { name: String },
    Plain(String),
}

impl ProductOption {
    fn name(&self) -> &str {
        match self {
            Self::Named { name } => name,
            Self::Plain(n) => n,
        }
    }
}

/// `https://shop/products/handle?variant=1` → `https://shop/products/handle.js`
pub fn json_url(page: &Url) -> Option<Url> {
    let path = page.path().trim_end_matches('/');
    let idx = path.find("/products/")?;
    let handle_part = &path[idx..];
    if handle_part.len() <= "/products/".len() || handle_part.ends_with(".js") {
        return None;
    }
    let mut u = page.clone();
    u.set_path(&format!("{path}.js"));
    u.set_query(None);
    u.set_fragment(None);
    Some(u)
}

pub fn looks_like_shopify(html: &str) -> bool {
    html.contains("cdn.shopify.com") || html.contains("Shopify.theme")
}

/// Returns the draft plus the description text for the regex pass.
pub fn extract(p: &Product, page: &Url) -> (Draft, String) {
    let wanted: Option<u64> = page
        .query_pairs()
        .find(|(k, _)| k == "variant")
        .and_then(|(_, v)| v.parse().ok());
    let variant = wanted
        .and_then(|id| p.variants.iter().find(|v| v.id == id))
        .or_else(|| p.variants.first());

    // Use the option named Color/Colour when present, else the variant title.
    let color_idx = p
        .options
        .iter()
        .position(|o| matches!(o.name().to_ascii_lowercase().as_str(), "color" | "colour"));
    let color = variant.and_then(|v| match color_idx {
        Some(0) => v.option1.clone(),
        Some(1) => v.option2.clone(),
        Some(2) => v.option3.clone(),
        _ => (p.variants.len() > 1 && v.title != "Default Title").then(|| v.title.clone()),
    });

    let mut image = variant
        .and_then(|v| v.featured_image.as_ref().map(|i| i.src.clone()))
        .or_else(|| p.featured_image.clone());
    if let Some(src) = image.as_mut().filter(|s| s.starts_with("//")) {
        *src = format!("https:{src}");
    }

    let draft = Draft {
        title: Some(p.title.clone()),
        brand_name: p.vendor.clone(),
        color_name: color,
        image_url: image,
        price: variant.and_then(|v| v.price).map(|c| Decimal::new(c, 2)),
        ..Default::default()
    };
    let text = p.description.as_deref().map(strip_tags).unwrap_or_default();
    (draft, text)
}

fn strip_tags(html: &str) -> String {
    scraper::Html::parse_fragment(html)
        .root_element()
        .text()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_url_from_product_page() {
        let u = Url::parse("https://us.polymaker.com/products/polyterra-pla?variant=42").unwrap();
        assert_eq!(
            json_url(&u).unwrap().as_str(),
            "https://us.polymaker.com/products/polyterra-pla.js"
        );
        assert!(json_url(&Url::parse("https://example.com/collections/all").unwrap()).is_none());
    }

    #[test]
    fn picks_requested_variant_color() {
        let p: Product =
            serde_json::from_str(include_str!("../../../tests/fixtures/shopify_product.json"))
                .unwrap();
        let u = Url::parse("https://shop.example/products/polyterra-pla?variant=222").unwrap();
        let (d, text) = extract(&p, &u);
        assert_eq!(d.brand_name.as_deref(), Some("Polymaker"));
        assert_eq!(d.color_name.as_deref(), Some("Fossil Grey"));
        assert_eq!(d.price, Some(Decimal::new(2299, 2)));
        assert_eq!(
            d.image_url.as_deref(),
            Some("https://cdn.shopify.com/grey.jpg")
        );
        assert!(text.contains("Nozzle Temperature"));
    }
}
