use crate::{
    AppState,
    error::AppResult,
    models::{Equipment, EquipmentFilter, SaveEquipment},
    repo::equipment,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/equipment", get(list).post(create))
        .route("/equipment/{id}", get(show).put(update).delete(remove))
}

async fn list(
    State(s): State<AppState>,
    Query(filter): Query<EquipmentFilter>,
) -> AppResult<Json<Vec<Equipment>>> {
    Ok(Json(equipment::list(&s.pool, &filter).await?))
}

async fn show(State(s): State<AppState>, Path(id): Path<Uuid>) -> AppResult<Json<Equipment>> {
    Ok(Json(equipment::get(&s.pool, id).await?))
}

async fn create(
    State(s): State<AppState>,
    Json(body): Json<SaveEquipment>,
) -> AppResult<(StatusCode, Json<Equipment>)> {
    Ok((
        StatusCode::CREATED,
        Json(equipment::save(&s.pool, None, body).await?),
    ))
}

async fn update(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<SaveEquipment>,
) -> AppResult<Json<Equipment>> {
    Ok(Json(equipment::save(&s.pool, Some(id), body).await?))
}

async fn remove(State(s): State<AppState>, Path(id): Path<Uuid>) -> AppResult<StatusCode> {
    equipment::delete(&s.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
