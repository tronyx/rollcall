use crate::{
    AppState,
    auth::CurrentUser,
    error::AppResult,
    models::{
        CreateSpool, PhotoOwner, SpoolFilter, SpoolView, UpdateSpool, WeightAdjustment, WeightEvent,
    },
    repo::{
        photos,
        spools::{self, WeightChange},
    },
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::Deserialize;
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/spools", get(list).post(create))
        .route("/spools/{id}", get(show).put(update).delete(remove))
        .route("/spools/{id}/weight", post(adjust_weight))
        .route("/spools/{id}/weight-events", get(history))
}

async fn list(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Query(f): Query<SpoolFilter>,
) -> AppResult<Json<Vec<SpoolView>>> {
    Ok(Json(spools::list_spools(&s.pool, me.id, &f).await?))
}

async fn show(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<SpoolView>> {
    Ok(Json(spools::get_spool(&s.pool, me.id, id).await?))
}

async fn create(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Json(body): Json<CreateSpool>,
) -> AppResult<(StatusCode, Json<SpoolView>)> {
    Ok((
        StatusCode::CREATED,
        Json(spools::create_spool(&s.pool, me.id, body).await?),
    ))
}

async fn update(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateSpool>,
) -> AppResult<Json<SpoolView>> {
    Ok(Json(spools::update_spool(&s.pool, me.id, id, body).await?))
}

async fn remove(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let owner = PhotoOwner {
        spool_id: Some(id),
        ..Default::default()
    };
    let files = photos::file_names(&s.pool, &owner).await?;
    spools::delete_spool(&s.pool, me.id, id).await?;
    photos::remove_files(&s.uploads, &files).await;
    Ok(StatusCode::NO_CONTENT)
}

async fn adjust_weight(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
    Json(body): Json<WeightAdjustment>,
) -> AppResult<Json<WeightChange>> {
    Ok(Json(spools::adjust_weight(&s.pool, me.id, id, body).await?))
}

#[derive(Deserialize)]
struct HistoryQuery {
    limit: Option<i64>,
}

async fn history(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
    Query(q): Query<HistoryQuery>,
) -> AppResult<Json<Vec<WeightEvent>>> {
    Ok(Json(
        spools::weight_history(&s.pool, me.id, id, q.limit.unwrap_or(50)).await?,
    ))
}
