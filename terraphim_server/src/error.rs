use axum::http::StatusCode;
use axum::{
    Json,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use terraphim_service::error::{CommonError, ErrorCategory, TerraphimError};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum Status {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "partial_success")]
    PartialSuccess,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub status: Status,
    pub message: String,
    pub category: Option<String>,
    pub recoverable: Option<bool>,
}

// Make our own error that wraps `anyhow::Error`.
#[derive(Debug)]
pub struct ApiError(pub StatusCode, pub anyhow::Error);

// Tell axum how to convert `ApiError` into a response.
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // Try to extract terraphim error information using chain inspection
        let mut category = None;
        let mut recoverable = None;

        // Check error chain for TerraphimError implementations
        let mut current_error: &dyn std::error::Error = self.1.as_ref();
        loop {
            // Check for specific service error types
            if let Some(service_err) =
                current_error.downcast_ref::<terraphim_service::ServiceError>()
            {
                category = Some(format!("{:?}", service_err.category()).to_lowercase());
                recoverable = Some(service_err.is_recoverable());
                break;
            }

            // Continue down the error chain
            match current_error.source() {
                Some(source) => current_error = source,
                None => break,
            }
        }

        (
            self.0,
            Json(ErrorResponse {
                status: Status::Error,
                message: self.1.to_string(),
                category,
                recoverable,
            }),
        )
            .into_response()
    }
}

fn status_code_from_category(category: ErrorCategory) -> StatusCode {
    match category {
        ErrorCategory::Validation | ErrorCategory::Configuration => StatusCode::BAD_REQUEST,
        ErrorCategory::Auth => StatusCode::UNAUTHORIZED,
        ErrorCategory::Network | ErrorCategory::Integration => StatusCode::BAD_GATEWAY,
        ErrorCategory::Storage | ErrorCategory::System => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn status_code_from_error(error: &anyhow::Error) -> StatusCode {
    for cause in error.chain() {
        if let Some(service_err) = cause.downcast_ref::<terraphim_service::ServiceError>() {
            return status_code_from_category(service_err.category());
        }

        if let Some(common_err) = cause.downcast_ref::<CommonError>() {
            return status_code_from_category(common_err.category());
        }
    }

    StatusCode::INTERNAL_SERVER_ERROR
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to
// turn them into `Result<_, ApiError>`.
// That way you don't need to do that manually (e.g. `map_err(ApiError::from)`).
impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        let error = err.into();
        let status = status_code_from_error(&error);
        ApiError(status, error)
    }
}

pub type Result<T> = std::result::Result<T, ApiError>;
