use crate::{
    AppState,
    auth::CurrentUser,
    error::AppResult,
    models::{Brand, Location, LocationShare, Material, NewLocation, ShareLocation},
    repo::lookups,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, put},
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
        .route("/locations/{id}/shares", get(shares).put(share))
        .route("/locations/{id}/shares/{user_id}", delete(unshare))
}

async fn brands(State(s): State<AppState>) -> AppResult<Json<Vec<Brand>>> {
    Ok(Json(lookups::list_brands(&s.pool).await?))
}

async fn materials(State(s): State<AppState>) -> AppResult<Json<Vec<Material>>> {
    Ok(Json(lookups::list_materials(&s.pool).await?))
}

async fn locations(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
) -> AppResult<Json<Vec<Location>>> {
    Ok(Json(lookups::list_locations(&s.pool, me.id).await?))
}

async fn create_location(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Json(body): Json<NewLocation>,
) -> AppResult<(StatusCode, Json<Location>)> {
    Ok((
        StatusCode::CREATED,
        Json(lookups::create_location(&s.pool, me.id, body).await?),
    ))
}

async fn update_location(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
    Json(body): Json<NewLocation>,
) -> AppResult<Json<Location>> {
    Ok(Json(
        lookups::update_location(&s.pool, me.id, id, body).await?,
    ))
}

async fn delete_location(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    lookups::delete_location(&s.pool, me.id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn shares(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<LocationShare>>> {
    Ok(Json(lookups::list_shares(&s.pool, me.id, id).await?))
}

async fn share(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
    Json(body): Json<ShareLocation>,
) -> AppResult<Json<Vec<LocationShare>>> {
    Ok(Json(lookups::share(&s.pool, me.id, id, body).await?))
}

async fn unshare(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    lookups::unshare(&s.pool, me.id, id, user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
