//! Rollcall backend: filament inventory API and product-page metadata parser.

pub mod config;
pub mod db;
pub mod error;
pub mod models;
pub mod repo;
pub mod routes;
pub mod scraper;

use sqlx::PgPool;
use std::sync::Arc;

/// Shared state handed to every axum handler.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub scraper: Arc<scraper::ScraperService>,
}
