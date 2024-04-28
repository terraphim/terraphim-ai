use axum::{
    response::{IntoResponse, Response},
    Json,
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum Status {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub status: Status,
    pub message: String,
}

// Make our own error that wraps `anyhow::Error`.
#[derive(Debug)]
pub struct ApiError(pub StatusCode, pub anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.0,
            Json(ErrorResponse {
                status: Status::Error,
                message: self.1.to_string(),
            }),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to
// turn them into `Result<_, ApiError>`.
// That way you don't need to do that manually (e.g. `map_err(ApiError::from)`).
impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        ApiError(StatusCode::INTERNAL_SERVER_ERROR, err.into())
    }
}

pub type Result<T> = std::result::Result<T, ApiError>;
