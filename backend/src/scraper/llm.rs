//! Optional LLM-assisted extraction through the Claude Messages API.
//!
//! Only runs when `ANTHROPIC_API_KEY` is set and the deterministic extractors
//! left important fields empty. It only fills gaps; it never overrides values
//! found in structured data.

use super::draft::Draft;
use crate::config::LlmConfig;
use anyhow::{Context, Result, anyhow, bail};
use reqwest::Client;
use serde_json::{Value, json};
use std::time::Duration;

const API_VERSION: &str = "2026-10-21";
/// Page text sent to the model. Product specs are near the top of the visible
/// text; capping keeps requests fast and cheap.
pub const MAX_PAGE_CHARS: usize = 60_000;

const SYSTEM_PROMPT: &str = "You extract 3D printer filament product data from web page text. \
Return only values that the page states or that are unambiguous from the product title. \
Use null for anything not stated; do not guess typical values for the material. \
material_code must be one of: PLA, PLA+, PLA-SILK, PLA-MATTE, PLA-CF, PETG, PETG-CF, ABS, ASA, \
TPU, PA, PA-CF, PC, HIPS, PVA, or null. Temperatures are in °C, speeds in mm/s, \
drying_time_min in minutes, nominal_weight_g is net filament weight in grams.";

pub struct LlmExtractor {
    client: Client,
    cfg: LlmConfig,
}

impl LlmExtractor {
    pub fn new(cfg: LlmConfig) -> Result<Self> {
        let client = Client::builder().timeout(Duration::from_secs(90)).build()?;
        Ok(Self { client, cfg })
    }

    pub async fn extract(&self, url: &str, page_text: &str) -> Result<Draft> {
        let schema = schema();
        let supports_fallbacks = ["claude-opus-5", "claude-fable-5", "claude-sonnet-5-5"]
            .iter()
            .any(|p| self.cfg.model.starts_with(p));

        let mut body = json!({
            "model": self.cfg.model,
            "max_tokens": 4096,
            "system": SYSTEM_PROMPT,
            "output_config": {
                "effort": "low",
                "format": { "type": "json_schema", "schema": schema }
            },
            "messages": [{
                "role": "user",
                "content": format!("Product page URL: {url}\n\nPage text:\n{page_text}")
            }]
        });
        let mut req = self
            .client
            .post(format!(
                "{}/v1/messages",
                self.cfg.base_url.trim_end_matches('/')
            ))
            .header("x-api-key", &self.cfg.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json");
        if supports_fallbacks {
            // Re-runs on another model if a safety classifier declines.
            body["fallbacks"] = json!("default");
            req = req.header("anthropic-beta", "server-side-fallback-2026-07-01");
        }

        let resp = req.json(&body).send().await.context("calling Claude API")?;
        let status = resp.status();
        let payload: Value = resp.json().await.context("reading Claude API response")?;
        if !status.is_success() {
            let msg = payload["error"]["message"]
                .as_str()
                .unwrap_or("unknown error");
            bail!("Claude API returned {status}: {msg}");
        }
        match payload["stop_reason"].as_str() {
            Some("refusal") => bail!("model declined the request"),
            Some("max_tokens") => bail!("model output was cut off"),
            _ => {}
        }
        let text = payload["content"]
            .as_array()
            .and_then(|blocks| {
                blocks
                    .iter()
                    .find(|b| b["type"] == "text")
                    .and_then(|b| b["text"].as_str())
            })
            .ok_or_else(|| anyhow!("no text block in Claude response"))?;
        serde_json::from_str::<Draft>(text).context("parsing structured output")
    }
}

fn schema() -> Value {
    let s = json!({ "type": ["string", "null"] });
    let i = json!({ "type": ["integer", "null"] });
    let n = json!({ "type": ["number", "null"] });
    let props = json!({
        "title": s, "brand_name": s, "material_code": s, "product_name": s,
        "color_name": s, "color_hex": s, "image_url": s,
        "diameter_mm": n, "nominal_weight_g": n, "price": n, "currency": s,
        "nozzle_temp_min_c": i, "nozzle_temp_max_c": i,
        "bed_temp_min_c": i, "bed_temp_max_c": i,
        "print_speed_min_mm_s": i, "print_speed_max_mm_s": i,
        "drying_temp_c": i, "drying_time_min": i
    });
    let required: Vec<&String> = props.as_object().unwrap().keys().collect();
    json!({
        "type": "object",
        "properties": props,
        "required": required,
        "additionalProperties": false
    })
}
