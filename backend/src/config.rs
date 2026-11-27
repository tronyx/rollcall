use anyhow::{Context, Result};
use std::{env, path::PathBuf, time::Duration};

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    /// Directory holding the built SvelteKit app. When set, the backend serves it.
    pub static_dir: Option<PathBuf>,
    /// Where uploaded photos are stored. Created at startup if missing.
    pub upload_dir: PathBuf,
    /// Extra allowed CORS origin, e.g. the Vite dev server.
    pub cors_origin: Option<String>,
    pub scraper: ScraperConfig,
    pub llm: Option<LlmConfig>,
}

#[derive(Debug, Clone)]
pub struct ScraperConfig {
    pub user_agent: String,
    pub timeout: Duration,
    pub max_body_bytes: usize,
    /// Allow fetching private/loopback addresses. Off by default to prevent SSRF.
    pub allow_private_hosts: bool,
}

impl Default for ScraperConfig {
    fn default() -> Self {
        Self {
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                         (KHTML, like Gecko) Chrome/130.0 Safari/537.36 Rollcall/0.1"
                .into(),
            timeout: Duration::from_secs(12),
            max_body_bytes: 4 * 1024 * 1024,
            allow_private_hosts: false,
        }
    }
}

/// Optional LLM-assisted extraction. Enabled only when `ANTHROPIC_API_KEY` is set.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let database_url = env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
        let defaults = ScraperConfig::default();
        let scraper = ScraperConfig {
            user_agent: env::var("SCRAPER_USER_AGENT").unwrap_or(defaults.user_agent),
            timeout: env::var("SCRAPER_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .map(Duration::from_secs)
                .unwrap_or(defaults.timeout),
            max_body_bytes: defaults.max_body_bytes,
            allow_private_hosts: env_flag("SCRAPER_ALLOW_PRIVATE_HOSTS"),
        };
        let llm = env::var("ANTHROPIC_API_KEY")
            .ok()
            .filter(|k| !k.trim().is_empty())
            .map(|api_key| LlmConfig {
                api_key,
                model: env::var("LLM_MODEL").unwrap_or_else(|_| "claude-opus-5-5".into()),
                base_url: env::var("ANTHROPIC_BASE_URL")
                    .unwrap_or_else(|_| "https://api.anthropic.com".into()),
            });

        Ok(Self {
            database_url,
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into()),
            static_dir: env::var("STATIC_DIR").ok().map(PathBuf::from),
            upload_dir: env::var("UPLOAD_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("uploads")),
            cors_origin: env::var("CORS_ORIGIN").ok(),
            scraper,
            llm,
        })
    }
}

fn env_flag(name: &str) -> bool {
    matches!(
        env::var(name)
            .as_deref()
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Ok("1" | "true" | "yes")
    )
}
