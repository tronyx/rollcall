use crate::{
    AppState,
    error::AppResult,
    models::{Filament, NewFilament},
    repo::filaments,
    scraper::ParseResult,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use serde::Deserialize;
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/filaments", get(list))
        .route("/filaments/{id}", get(show).put(update))
        .route("/filaments/parse-url", post(parse_url))
}

async fn list(State(s): State<AppState>) -> AppResult<Json<Vec<Filament>>> {
    Ok(Json(filaments::list_filaments(&s.pool).await?))
}

async fn show(State(s): State<AppState>, Path(id): Path<Uuid>) -> AppResult<Json<Filament>> {
    Ok(Json(filaments::get_filament(&s.pool, id).await?))
}

/// `PUT /api/filaments/{id}`: same body as an inline filament on a new spool.
async fn update(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<NewFilament>,
) -> AppResult<Json<Filament>> {
    Ok(Json(filaments::update_filament(&s.pool, id, body).await?))
}

#[derive(Deserialize)]
struct ParseUrlRequest {
    url: String,
}

/// Fetches a product page and returns a pre-fill draft. Never saves anything.
async fn parse_url(
    State(s): State<AppState>,
    Json(body): Json<ParseUrlRequest>,
) -> AppResult<Json<ParseResult>> {
    Ok(Json(s.scraper.parse(&body.url).await?))
}
