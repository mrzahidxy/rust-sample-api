use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Unauthorized,
    Forbidden,
    ServiceUnavailable,
    Conflict,
    NotFound(String),
    InternalServerError(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl AppError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::InternalServerError(message.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::ServiceUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Authentication is busy; retry later".into(),
            ),
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                "You do not have permission to access this resource".into(),
            ),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "Invalid credentials".into()),
            Self::Conflict => (StatusCode::CONFLICT, "Resource already exists".into()),
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::NotFound(message) => (StatusCode::NOT_FOUND, message),
            Self::InternalServerError(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
        };

        let body = Json(ErrorResponse { error: message });
        (status, body).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        if let Some(database) = error.as_database_error() {
            if database.is_unique_violation() {
                return Self::Conflict;
            }
            if database.is_foreign_key_violation() {
                return Self::bad_request("Resource has related records or an invalid reference");
            }
        }
        tracing::error!(error = %error, "database operation failed");
        Self::internal("Database operation failed")
    }
}
