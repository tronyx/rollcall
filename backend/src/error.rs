use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    fn parts(&self) -> (StatusCode, &'static str, String) {
        match self {
            Self::NotFound(_) => (StatusCode::NOT_FOUND, "not_found", self.to_string()),
            Self::BadRequest(m) => (StatusCode::BAD_REQUEST, "bad_request", m.clone()),
            Self::Conflict(m) => (StatusCode::CONFLICT, "conflict", m.clone()),
            Self::Unauthorized(m) => (StatusCode::UNAUTHORIZED, "unauthorized", m.clone()),
            Self::Forbidden(m) => (StatusCode::FORBIDDEN, "forbidden", m.clone()),
            Self::Database(sqlx::Error::RowNotFound) => (
                StatusCode::NOT_FOUND,
                "not_found",
                "record not found".into(),
            ),
            Self::Database(sqlx::Error::Database(db)) => match db.code().as_deref() {
                // unique_violation
                Some("23505") => (
                    StatusCode::CONFLICT,
                    "conflict",
                    format!("already exists ({})", db.constraint().unwrap_or("unique")),
                ),
                // foreign_key_violation, check_violation, not_null_violation, invalid_text_representation
                Some("23503" | "23514" | "23502" | "22P02") => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "validation_failed",
                    format!(
                        "invalid value ({})",
                        db.constraint().unwrap_or(db.message())
                    ),
                ),
                _ => internal(),
            },
            Self::Database(_) | Self::Internal(_) => internal(),
        }
    }
}

fn internal() -> (StatusCode, &'static str, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal",
        "internal server error".into(),
    )
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = self.parts();
        if status.is_server_error() {
            tracing::error!(error = ?self, "request failed");
        }
        (
            status,
            Json(json!({ "error": { "code": code, "message": message } })),
        )
            .into_response()
    }
}
