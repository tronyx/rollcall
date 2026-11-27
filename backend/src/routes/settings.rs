use crate::{
    AppState,
    error::AppResult,
    models::{AppSettings, NewPrinter, Printer},
    repo::settings,
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
        .route("/settings", get(show).put(update))
        .route("/printers", get(printers).post(create_printer))
        .route("/printers/{id}", put(update_printer).delete(delete_printer))
}

async fn show(State(s): State<AppState>) -> AppResult<Json<AppSettings>> {
    Ok(Json(settings::get_settings(&s.pool).await?))
}

async fn update(
    State(s): State<AppState>,
    Json(body): Json<AppSettings>,
) -> AppResult<Json<AppSettings>> {
    Ok(Json(settings::update_settings(&s.pool, body).await?))
}

async fn printers(State(s): State<AppState>) -> AppResult<Json<Vec<Printer>>> {
    Ok(Json(settings::list_printers(&s.pool).await?))
}

async fn create_printer(
    State(s): State<AppState>,
    Json(body): Json<NewPrinter>,
) -> AppResult<(StatusCode, Json<Printer>)> {
    Ok((
        StatusCode::CREATED,
        Json(settings::save_printer(&s.pool, None, body).await?),
    ))
}

async fn update_printer(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<NewPrinter>,
) -> AppResult<Json<Printer>> {
    Ok(Json(settings::save_printer(&s.pool, Some(id), body).await?))
}

async fn delete_printer(State(s): State<AppState>, Path(id): Path<Uuid>) -> AppResult<StatusCode> {
    settings::delete_printer(&s.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
