use crate::{
    AppState,
    error::AppResult,
    models::{Brand, Location, Material, NewLocation},
    repo::lookups,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
};
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/brands", get(brands))
        .route("/materials", get(materials))
        .route("/locations", get(locations).post(create_location))
        .route(
            "/locations/{id}",
            put(update_location).delete(delete_location),
        )
}

async fn brands(State(s): State<AppState>) -> AppResult<Json<Vec<Brand>>> {
    Ok(Json(lookups::list_brands(&s.pool).await?))
}

async fn materials(State(s): State<AppState>) -> AppResult<Json<Vec<Material>>> {
    Ok(Json(lookups::list_materials(&s.pool).await?))
}

async fn locations(State(s): State<AppState>) -> AppResult<Json<Vec<Location>>> {
    Ok(Json(lookups::list_locations(&s.pool).await?))
}

async fn create_location(
    State(s): State<AppState>,
    Json(body): Json<NewLocation>,
) -> AppResult<(StatusCode, Json<Location>)> {
    Ok((
        StatusCode::CREATED,
        Json(lookups::create_location(&s.pool, body).await?),
    ))
}

async fn update_location(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<NewLocation>,
) -> AppResult<Json<Location>> {
    Ok(Json(lookups::update_location(&s.pool, id, body).await?))
}

async fn delete_location(State(s): State<AppState>, Path(id): Path<Uuid>) -> AppResult<StatusCode> {
    lookups::delete_location(&s.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
