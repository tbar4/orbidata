use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("upstream request failed: {0}")]
    Upstream(#[from] reqwest::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),

    #[error("service unavailable: {0}")]
    Unavailable(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Upstream(e) => (StatusCode::BAD_GATEWAY, e.to_string()),
            AppError::Serialization(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::Unavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg.clone()),
        };

        let body = Json(json!({
            "error": {
                "code": status.as_u16(),
                "message": message,
            }
        }));

        (status, body).into_response()
    }
}
