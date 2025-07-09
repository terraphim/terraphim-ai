use thiserror::Error;
use std::string::FromUtf8Error;
use url::ParseError;

#[cfg(feature = "native")]
use reqwest::header::{InvalidHeaderValue, ToStrError};

/// Error type for the Atomic Server Client.
#[derive(Error, Debug)]
pub enum AtomicError {
    /// Error parsing a value
    #[error("Parse error: {0}")]
    Parse(String),

    /// Error with authentication
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Error with HTTP request/response
    #[cfg(feature = "native")]
    #[error("HTTP error: {0}")]
    Http(reqwest::Error),
    #[cfg(not(feature = "native"))]
    #[error("HTTP error: {0}")]
    Http(String),

    /// Error with JSON serialization/deserialization
    #[error("JSON error: {0}")]
    Json(serde_json::Error),

    /// Error with URL parsing
    #[error("URL parse error: {0}")]
    UrlParse(ParseError),

    /// Error with base64 decoding
    #[error("Base64 decode error: {0}")]
    Base64Decode(base64::DecodeError),

    /// Error with header value conversion (native only)
    #[cfg(feature = "native")]
    #[error("Header value error: {0}")]
    HeaderValue(InvalidHeaderValue),
    #[cfg(not(feature = "native"))]
    #[error("Header value error")]
    HeaderValue(String),

    /// Error with header value string conversion (native only)
    #[cfg(feature = "native")]
    #[error("Header to string error: {0}")]
    HeaderToStr(ToStrError),
    #[cfg(not(feature = "native"))]
    #[error("Header to string error")]
    HeaderToStr(String),

    /// Error with UTF-8 conversion
    #[error("UTF-8 conversion error: {0}")]
    Utf8(FromUtf8Error),

    /// Error with API response
    #[error("API error: {0}")]
    Api(String),
}

/// Result type for the Atomic Server Client.
#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, AtomicError>;

#[cfg(feature = "native")]
impl From<reqwest::Error> for AtomicError {
    fn from(err: reqwest::Error) -> Self {
        AtomicError::Http(err)
    }
}

impl From<serde_json::Error> for AtomicError {
    fn from(err: serde_json::Error) -> Self {
        AtomicError::Json(err)
    }
}

impl From<&str> for AtomicError {
    fn from(err: &str) -> Self {
        AtomicError::Authentication(err.to_string())
    }
}

impl From<String> for AtomicError {
    fn from(err: String) -> Self {
        AtomicError::Authentication(err)
    }
}

#[cfg(feature = "native")]
impl From<reqwest::header::InvalidHeaderValue> for AtomicError {
    fn from(err: reqwest::header::InvalidHeaderValue) -> Self {
        AtomicError::HeaderValue(err)
    }
}

impl From<base64::DecodeError> for AtomicError {
    fn from(err: base64::DecodeError) -> Self {
        AtomicError::Base64Decode(err)
    }
}

#[cfg(target_arch = "wasm32")]
impl From<wasm_bindgen::JsValue> for AtomicError {
    fn from(err: wasm_bindgen::JsValue) -> Self {
        AtomicError::Http(format!("WASM error: {:?}", err).into())
    }
}

#[cfg(target_arch = "wasm32")]
impl From<std::str::Utf8Error> for AtomicError {
    fn from(err: std::str::Utf8Error) -> Self {
        AtomicError::Http(format!("UTF8 error: {}", err).into())
    }
}

impl From<url::ParseError> for AtomicError {
    fn from(err: url::ParseError) -> Self {
        AtomicError::UrlParse(err)
    }
}

#[cfg(feature = "native")]
impl From<reqwest::header::ToStrError> for AtomicError {
    fn from(err: reqwest::header::ToStrError) -> Self {
        AtomicError::HeaderToStr(err)
    }
} 