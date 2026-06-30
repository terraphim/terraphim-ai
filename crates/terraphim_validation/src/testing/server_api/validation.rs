//! Response validation utilities for API testing
//!
//! This module provides traits and utilities for validating HTTP responses
//! and ensuring they conform to expected schemas and status codes.

use reqwest::Response;
use serde::de::DeserializeOwned;
use std::fmt;

/// Trait for validating HTTP responses
pub trait ResponseValidator {
    /// Validate that the response has the expected status code
    fn validate_status(self, expected: reqwest::StatusCode) -> Self;

    /// Validate that the response body can be deserialized to the expected type
    fn validate_json<T: DeserializeOwned>(self) -> Result<T, ValidationError>;

    /// Validate that the response is an error and return the error message
    fn validate_error_response(self) -> Result<Option<String>, ValidationError>;

    /// Validate response time is within acceptable limits
    fn validate_response_time(self, max_ms: u64) -> Self;
}

/// Validation error types
#[derive(Debug)]
pub enum ValidationError {
    /// HTTP request failed
    Request(reqwest::Error),
    /// JSON deserialization failed
    Json(serde_json::Error),
    /// Status code mismatch
    StatusMismatch {
        expected: reqwest::StatusCode,
        actual: reqwest::StatusCode,
    },
    /// Response time exceeded limit
    ResponseTimeExceeded { max_ms: u64, actual_ms: u64 },
    /// JSON Schema violation
    SchemaViolation(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::Request(e) => write!(f, "Request error: {}", e),
            ValidationError::Json(e) => write!(f, "JSON deserialization error: {}", e),
            ValidationError::StatusMismatch { expected, actual } => {
                write!(
                    f,
                    "Status code mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            ValidationError::ResponseTimeExceeded { max_ms, actual_ms } => {
                write!(
                    f,
                    "Response time {}ms exceeded limit {}ms",
                    actual_ms, max_ms
                )
            }
            ValidationError::SchemaViolation(msg) => {
                write!(f, "JSON Schema violation: {}", msg)
            }
        }
    }
}

impl std::error::Error for ValidationError {}

impl From<reqwest::Error> for ValidationError {
    fn from(err: reqwest::Error) -> Self {
        ValidationError::Request(err)
    }
}

impl From<serde_json::Error> for ValidationError {
    fn from(err: serde_json::Error) -> Self {
        ValidationError::Json(err)
    }
}

impl ResponseValidator for Response {
    fn validate_status(mut self, expected: reqwest::StatusCode) -> Self {
        let actual = self.status();
        if actual != expected {
            // Use blocking text extraction for panic message
            let text = tokio::runtime::Handle::current()
                .block_on(self.text())
                .unwrap_or_default();
            panic!(
                "Expected status {}, got {}. Response: {:?}",
                expected, actual, text
            );
        }
        self
    }

    fn validate_json<T: DeserializeOwned>(self) -> Result<T, ValidationError> {
        let text = tokio::runtime::Handle::current().block_on(self.text())?;
        serde_json::from_str(&text).map_err(ValidationError::Json)
    }

    fn validate_error_response(self) -> Result<Option<String>, ValidationError> {
        if self.status().is_success() {
            Ok(None)
        } else {
            let text = tokio::runtime::Handle::current().block_on(self.text())?;
            Ok(Some(text))
        }
    }

    fn validate_response_time(self, max_ms: u64) -> Self {
        // Note: Response time validation would require timing the request
        // This is a placeholder for future implementation
        self
    }
}

/// Implementation for axum_test::TestResponse
impl ResponseValidator for axum_test::TestResponse {
    fn validate_status(self, expected: reqwest::StatusCode) -> Self {
        let actual = self.status_code();
        if actual != expected {
            panic!("Expected status {}, got {}", expected, actual);
        }
        self
    }

    fn validate_json<T: DeserializeOwned>(self) -> Result<T, ValidationError> {
        Ok(self.json())
    }

    fn validate_error_response(self) -> Result<Option<String>, ValidationError> {
        if self.status_code().is_success() {
            Ok(None)
        } else {
            Ok(Some(self.text()))
        }
    }

    fn validate_response_time(self, _max_ms: u64) -> Self {
        // Note: Response time validation would require timing the request
        self
    }
}

/// Validate a `serde_json::Value` against a JSON Schema draft-7 string.
///
/// Separated from the HTTP layer so it can be unit-tested without a real server.
fn validate_value_against_schema(
    value: &serde_json::Value,
    schema: &str,
) -> Result<(), ValidationError> {
    let schema_value: serde_json::Value = serde_json::from_str(schema)
        .map_err(|e| ValidationError::SchemaViolation(format!("Invalid schema JSON: {}", e)))?;

    let validator = jsonschema::validator_for(&schema_value).map_err(|e| {
        ValidationError::SchemaViolation(format!("Schema compilation error: {}", e))
    })?;

    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|e| e.to_string())
        .collect();
    if !errors.is_empty() {
        return Err(ValidationError::SchemaViolation(errors.join("; ")));
    }

    Ok(())
}

/// Validate that a JSON response matches a JSON schema
pub fn validate_json_schema<T: DeserializeOwned>(
    response: Response,
    schema: &str,
) -> Result<T, ValidationError> {
    let body_text = tokio::runtime::Handle::current().block_on(response.text())?;
    let body_value: serde_json::Value =
        serde_json::from_str(&body_text).map_err(ValidationError::Json)?;

    validate_value_against_schema(&body_value, schema)?;

    serde_json::from_value(body_value).map_err(ValidationError::Json)
}

/// Assert that two JSON values are equal (ignoring ordering)
pub fn assert_json_equal<T: serde::Serialize + serde::de::DeserializeOwned>(
    actual: &T,
    expected: &T,
) {
    let actual_json = serde_json::to_value(actual).unwrap();
    let expected_json = serde_json::to_value(expected).unwrap();

    if actual_json != expected_json {
        panic!(
            "JSON mismatch:\nExpected: {}\nActual: {}",
            serde_json::to_string_pretty(&expected_json).unwrap(),
            serde_json::to_string_pretty(&actual_json).unwrap()
        );
    }
}

/// Validate response headers
pub fn validate_response_headers(response: &Response, expected_headers: &[(&str, &str)]) {
    for (key, expected_value) in expected_headers {
        let actual_value = response.headers().get(*key).and_then(|v| v.to_str().ok());

        match actual_value {
            Some(value) if value == *expected_value => continue,
            Some(value) => panic!(
                "Header '{}' mismatch: expected '{}', got '{}'",
                key, expected_value, value
            ),
            None => panic!("Missing expected header: {}", key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PERSON_SCHEMA: &str = r#"{
        "type": "object",
        "required": ["name"],
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "integer"}
        }
    }"#;

    #[test]
    fn schema_validation_passes_for_matching_response() {
        let value = serde_json::json!({"name": "Alice", "age": 30});
        assert!(validate_value_against_schema(&value, PERSON_SCHEMA).is_ok());
    }

    #[test]
    fn schema_validation_passes_with_only_required_fields() {
        let value = serde_json::json!({"name": "Bob"});
        assert!(validate_value_against_schema(&value, PERSON_SCHEMA).is_ok());
    }

    #[test]
    fn schema_validation_fails_for_missing_required_field() {
        let value = serde_json::json!({"age": 42});
        let result = validate_value_against_schema(&value, PERSON_SCHEMA);
        assert!(
            matches!(result, Err(ValidationError::SchemaViolation(_))),
            "expected SchemaViolation, got {:?}",
            result
        );
    }

    #[test]
    fn schema_validation_fails_for_wrong_type() {
        let value = serde_json::json!({"name": 123});
        let result = validate_value_against_schema(&value, PERSON_SCHEMA);
        assert!(matches!(result, Err(ValidationError::SchemaViolation(_))));
    }

    #[test]
    fn schema_validation_fails_for_invalid_schema_string() {
        let value = serde_json::json!({"name": "Alice"});
        let result = validate_value_against_schema(&value, "not valid json{{{");
        assert!(matches!(result, Err(ValidationError::SchemaViolation(_))));
    }

    #[test]
    fn schema_violation_error_contains_details() {
        let value = serde_json::json!({"age": 42});
        let err = validate_value_against_schema(&value, PERSON_SCHEMA).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Schema violation") || msg.contains("schema") || msg.contains("name"),
            "error message should describe the violation: {}",
            msg
        );
    }
}
