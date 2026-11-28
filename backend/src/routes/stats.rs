use crate::{AppState, auth::CurrentUser, error::AppResult, models::InventoryStats, repo::stats};
use axum::{Json, Router, extract::State, routing::get};

pub fn routes() -> Router<AppState> {
    Router::new().route("/stats", get(inventory))
}

async fn inventory(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
) -> AppResult<Json<InventoryStats>> {
    Ok(Json(stats::inventory_stats(&s.pool, me.id).await?))
}
