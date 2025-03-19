use salvo::prelude::*;
use salvo::http::StatusCode;
use thiserror::Error;
use serde_json::json;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Bad request: {0}")]
    InvalidRequest(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Access denied: {0}")]
    AccessDenied(String),
    
    #[error("Resource already exists: {0}")]
    AlreadyExists(String),
    
    #[error("Invalid URI: {0}")]
    InvalidUri(String),
    
    #[error("Invalid MIME type: {0}")]
    InvalidMimeType(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("WebSocket error: {0}")]
    WebSocketError(String),
    
    #[error("Subscription error: {0}")]
    SubscriptionError(String),
}

impl ServerError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ServerError::NotFound(_) => StatusCode::NOT_FOUND,
            ServerError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            ServerError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::AccessDenied(_) => StatusCode::FORBIDDEN,
            ServerError::AlreadyExists(_) => StatusCode::CONFLICT,
            ServerError::InvalidUri(_) => StatusCode::BAD_REQUEST,
            ServerError::InvalidMimeType(_) => StatusCode::BAD_REQUEST,
            ServerError::SerializationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::WebSocketError(_) => StatusCode::BAD_REQUEST,
            ServerError::SubscriptionError(_) => StatusCode::BAD_REQUEST,
        }
    }
}

#[async_trait]
impl Writer for ServerError {
    async fn write(self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        res.status_code(self.status_code());
        res.render(Json(json!({
            "error": self.to_string()
        })));
    }
}

impl From<serde_json::Error> for ServerError {
    fn from(error: serde_json::Error) -> Self {
        ServerError::SerializationError(error.to_string())
    }
}

impl From<url::ParseError> for ServerError {
    fn from(error: url::ParseError) -> Self {
        ServerError::InvalidUri(error.to_string())
    }
}

impl From<std::io::Error> for ServerError {
    fn from(error: std::io::Error) -> Self {
        ServerError::Internal(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(ServerError::NotFound("test".to_string()).status_code(), StatusCode::NOT_FOUND);
        assert_eq!(ServerError::InvalidRequest("test".to_string()).status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(ServerError::Internal("test".to_string()).status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(ServerError::AccessDenied("test".to_string()).status_code(), StatusCode::FORBIDDEN);
        assert_eq!(ServerError::AlreadyExists("test".to_string()).status_code(), StatusCode::CONFLICT);
    }
} 