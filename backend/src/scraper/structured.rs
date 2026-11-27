//! schema.org JSON-LD and OpenGraph/meta tag extraction.

use super::draft::Draft;
use rust_decimal::Decimal;
use scraper::{Html, Selector};
use serde_json::Value;
use std::sync::LazyLock;

static LD_JSON: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"script[type="application/ld+json"]"#).unwrap());
static META: LazyLock<Selector> = LazyLock::new(|| Selector::parse("meta").unwrap());

/// Finds the first schema.org `Product` (also inside `@graph` or arrays).
fn find_product(v: &Value) -> Option<&Value> {
    match v {
        Value::Array(items) => items.iter().find_map(find_product),
        Value::Object(map) => {
            let is_product = match map.get("@type") {
                Some(Value::String(t)) => t.eq_ignore_ascii_case("Product") || t == "ProductGroup",
                Some(Value::Array(ts)) => ts.iter().any(|t| t.as_str() == Some("Product")),
                _ => false,
            };
            if is_product {
                Some(v)
            } else {
                map.get("@graph").and_then(find_product)
            }
        }
        _ => None,
    }
}

fn text_of(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::String(s) => Some(s.clone()),
        Value::Object(m) => m.get("name").and_then(|n| n.as_str()).map(String::from),
        Value::Array(a) => a.iter().find_map(|x| text_of(Some(x))),
        _ => None,
    }
}

fn image_of(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::String(s) => Some(s.clone()),
        Value::Array(a) => a.iter().find_map(|x| image_of(Some(x))),
        Value::Object(m) => m
            .get("url")
            .or_else(|| m.get("contentUrl"))
            .and_then(|u| u.as_str())
            .map(String::from),
        _ => None,
    }
}

fn decimal_of(v: &Value) -> Option<Decimal> {
    match v {
        Value::String(s) => s.trim().parse().ok(),
        Value::Number(n) => n.to_string().parse().ok(),
        _ => None,
    }
}

/// Returns the draft plus extra "key: value" lines from `additionalProperty`,
/// which the caller feeds into the spec-text heuristics.
pub fn json_ld(doc: &Html) -> (Draft, String) {
    let mut spec_lines = String::new();
    for script in doc.select(&LD_JSON) {
        let raw = script.text().collect::<String>();
        let Ok(value) = serde_json::from_str::<Value>(raw.trim()) else {
            continue;
        };
        let Some(p) = find_product(&value) else {
            continue;
        };

        // Product groups list colors as variants; take the first variant's offer.
        let variant = p
            .get("hasVariant")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first());
        let offers = p
            .get("offers")
            .or_else(|| variant.and_then(|v| v.get("offers")))
            .map(|o| o.as_array().and_then(|a| a.first()).unwrap_or(o));

        if let Some(props) = p.get("additionalProperty").and_then(|v| v.as_array()) {
            for prop in props {
                if let (Some(k), Some(val)) = (text_of(prop.get("name")), prop.get("value")) {
                    let val = val
                        .as_str()
                        .map(String::from)
                        .unwrap_or_else(|| val.to_string());
                    spec_lines.push_str(&format!("{k}: {val}\n"));
                }
            }
        }

        let draft = Draft {
            title: text_of(p.get("name")),
            brand_name: text_of(p.get("brand")).or_else(|| text_of(p.get("manufacturer"))),
            material_code: None, // normalized by heuristics from `material` text below
            color_name: text_of(p.get("color"))
                .or_else(|| variant.and_then(|v| text_of(v.get("color")))),
            image_url: image_of(p.get("image")),
            price: offers.and_then(|o| o.get("price")).and_then(decimal_of),
            currency: offers
                .and_then(|o| o.get("priceCurrency"))
                .and_then(|c| c.as_str())
                .map(String::from),
            ..Default::default()
        };
        if let Some(m) = text_of(p.get("material")) {
            spec_lines.push_str(&format!("Material: {m}\n"));
        }
        if let Some(desc) = p.get("description").and_then(|d| d.as_str()) {
            spec_lines.push_str(desc);
            spec_lines.push('\n');
        }
        return (draft, spec_lines);
    }
    (Draft::default(), spec_lines)
}

pub fn meta_tags(doc: &Html) -> Draft {
    let mut d = Draft::default();
    for m in doc.select(&META) {
        let el = m.value();
        let key = el
            .attr("property")
            .or_else(|| el.attr("name"))
            .unwrap_or_default();
        let Some(content) = el.attr("content").map(str::trim).filter(|c| !c.is_empty()) else {
            continue;
        };
        let slot = match key.to_ascii_lowercase().as_str() {
            "og:title" | "twitter:title" => &mut d.title,
            "og:image" | "og:image:secure_url" | "twitter:image" => &mut d.image_url,
            "product:brand" | "og:brand" => &mut d.brand_name,
            "product:color" => &mut d.color_name,
            "product:price:currency" | "og:price:currency" => &mut d.currency,
            "product:price:amount" | "og:price:amount" => {
                if d.price.is_none() {
                    d.price = content.parse().ok();
                }
                continue;
            }
            _ => continue,
        };
        if slot.is_none() {
            *slot = Some(content.to_string());
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_ld_product_in_graph() {
        let html = r#"<html><head><script type="application/ld+json">
        {"@context":"https://schema.org","@graph":[{"@type":"WebPage"},
          {"@type":"Product","name":"PolyTerra PLA Charcoal Black","brand":{"@type":"Brand","name":"Polymaker"},
           "image":["https://cdn.example.com/a.jpg"],"color":"Charcoal Black",
           "offers":{"@type":"Offer","price":"19.99","priceCurrency":"USD"},
           "additionalProperty":[{"name":"Nozzle Temperature","value":"190-230°C"}]}]}
        </script></head></html>"#;
        let (d, spec) = json_ld(&Html::parse_document(html));
        assert_eq!(d.brand_name.as_deref(), Some("Polymaker"));
        assert_eq!(d.color_name.as_deref(), Some("Charcoal Black"));
        assert_eq!(
            d.image_url.as_deref(),
            Some("https://cdn.example.com/a.jpg")
        );
        assert_eq!(d.price, Some(Decimal::new(1999, 2)));
        assert!(spec.contains("Nozzle Temperature: 190-230°C"));
    }

    #[test]
    fn invalid_json_ld_is_skipped() {
        let html = r#"<script type="application/ld+json">{not json</script>
                      <meta property="og:title" content="eSUN PLA+ Black">"#;
        let doc = Html::parse_document(html);
        assert_eq!(json_ld(&doc).0, Draft::default());
        assert_eq!(meta_tags(&doc).title.as_deref(), Some("eSUN PLA+ Black"));
    }
}
