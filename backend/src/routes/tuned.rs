use crate::{
    AppState,
    auth::CurrentUser,
    error::AppResult,
    models::{SaveTunedSettings, TunedSettings},
    repo::tuned,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get},
};
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/filaments/{id}/tuned", get(list).put(save))
        .route("/tuned/{id}", delete(remove))
}

async fn list(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<TunedSettings>>> {
    Ok(Json(tuned::list_for_filament(&s.pool, me.id, id).await?))
}

async fn save(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
    Json(body): Json<SaveTunedSettings>,
) -> AppResult<Json<TunedSettings>> {
    Ok(Json(tuned::save(&s.pool, me.id, id, body).await?))
}

async fn remove(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    tuned::delete(&s.pool, me.id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
