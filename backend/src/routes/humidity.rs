use crate::{
    AppState,
    error::AppResult,
    models::{HumidityReading, NewHumidityReading},
    repo::humidity,
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
        .route("/spools/{id}/humidity", get(list).post(add))
        .route("/humidity/{id}", delete(remove))
}

async fn list(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<HumidityReading>>> {
    Ok(Json(humidity::list(&s.pool, id).await?))
}

async fn add(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<NewHumidityReading>,
) -> AppResult<(StatusCode, Json<HumidityReading>)> {
    Ok((
        StatusCode::CREATED,
        Json(humidity::add(&s.pool, id, body).await?),
    ))
}

async fn remove(State(s): State<AppState>, Path(id): Path<Uuid>) -> AppResult<StatusCode> {
    humidity::delete(&s.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
