use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// API error rendered as `{"detail": "..."}`, matching the previous FastAPI shape.
#[derive(Debug)]
pub struct AppError {
    pub status: StatusCode,
    pub detail: String,
}

impl AppError {
    pub fn new(status: StatusCode, detail: impl Into<String>) -> Self {
        Self {
            status,
            detail: detail.into(),
        }
    }

    pub fn from_sqlx(err: sqlx::Error) -> Self {
        let message = err.to_string();
        if message.contains("UNIQUE constraint") {
            return Self::new(StatusCode::CONFLICT, "Already exists");
        }
        if message.contains("FOREIGN KEY constraint") {
            return Self::new(StatusCode::BAD_REQUEST, "Related record not found");
        }
        tracing::error!(error = %err, "database error");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        Self::from_sqlx(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "detail": self.detail }))).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
