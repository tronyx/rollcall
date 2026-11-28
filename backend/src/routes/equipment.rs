use crate::{
    AppState,
    auth::CurrentUser,
    error::AppResult,
    models::{Equipment, EquipmentFilter, PhotoOwner, SaveEquipment},
    repo::{equipment, photos},
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
    CurrentUser(me): CurrentUser,
    Query(filter): Query<EquipmentFilter>,
) -> AppResult<Json<Vec<Equipment>>> {
    Ok(Json(equipment::list(&s.pool, me.id, &filter).await?))
}

async fn show(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Equipment>> {
    Ok(Json(equipment::get(&s.pool, me.id, id).await?))
}

async fn create(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Json(body): Json<SaveEquipment>,
) -> AppResult<(StatusCode, Json<Equipment>)> {
    Ok((
        StatusCode::CREATED,
        Json(equipment::save(&s.pool, me.id, None, body).await?),
    ))
}

async fn update(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
    Json(body): Json<SaveEquipment>,
) -> AppResult<Json<Equipment>> {
    Ok(Json(equipment::save(&s.pool, me.id, Some(id), body).await?))
}

async fn remove(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let owner = PhotoOwner {
        equipment_id: Some(id),
        ..Default::default()
    };
    let files = photos::file_names(&s.pool, &owner).await?;
    equipment::delete(&s.pool, me.id, id).await?;
    photos::remove_files(&s.uploads, &files).await;
    Ok(StatusCode::NO_CONTENT)
}
