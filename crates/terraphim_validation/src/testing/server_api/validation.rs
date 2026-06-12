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
    /// Response body violates the JSON Schema contract
    SchemaViolation {
        message: String,
        violations: Vec<String>,
    },
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
            ValidationError::SchemaViolation {
                message,
                violations,
            } => {
                write!(f, "Schema violation: {}", message)?;
                for v in violations {
                    write!(f, "\n  - {}", v)?;
                }
                Ok(())
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

/// Validate that a JSON response matches a JSON schema.
///
/// Parses `schema` as a JSON Schema document, validates the response body
/// against it, and then deserialises the body to `T`.  Returns
/// [`ValidationError::SchemaViolation`] if the body fails schema validation.
pub fn validate_json_schema<T: DeserializeOwned>(
    response: Response,
    schema: &str,
) -> Result<T, ValidationError> {
    let body_text = tokio::runtime::Handle::current().block_on(response.text())?;
    let instance: serde_json::Value =
        serde_json::from_str(&body_text).map_err(ValidationError::Json)?;

    let schema_value: serde_json::Value =
        serde_json::from_str(schema).map_err(|e| ValidationError::SchemaViolation {
            message: format!("invalid schema JSON: {e}"),
            violations: vec![],
        })?;

    let validator =
        jsonschema::validator_for(&schema_value).map_err(|e| ValidationError::SchemaViolation {
            message: format!("failed to compile schema: {e}"),
            violations: vec![],
        })?;

    let violations: Vec<String> = validator
        .iter_errors(&instance)
        .map(|e| e.to_string())
        .collect();

    if !violations.is_empty() {
        return Err(ValidationError::SchemaViolation {
            message: format!("{} schema violation(s)", violations.len()),
            violations,
        });
    }

    serde_json::from_value(instance).map_err(ValidationError::Json)
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

/// Validate a JSON value (already parsed) against a JSON Schema string.
///
/// Shared helper used internally and in unit tests; does not consume an HTTP
/// response.
pub fn validate_value_against_schema(
    instance: &serde_json::Value,
    schema: &str,
) -> Result<(), ValidationError> {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema).map_err(|e| ValidationError::SchemaViolation {
            message: format!("invalid schema JSON: {e}"),
            violations: vec![],
        })?;

    let validator =
        jsonschema::validator_for(&schema_value).map_err(|e| ValidationError::SchemaViolation {
            message: format!("failed to compile schema: {e}"),
            violations: vec![],
        })?;

    let violations: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| e.to_string())
        .collect();

    if !violations.is_empty() {
        return Err(ValidationError::SchemaViolation {
            message: format!("{} schema violation(s)", violations.len()),
            violations,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const PERSON_SCHEMA: &str = r#"{
        "type": "object",
        "required": ["name", "age"],
        "properties": {
            "name": { "type": "string" },
            "age":  { "type": "integer", "minimum": 0 }
        },
        "additionalProperties": false
    }"#;

    #[test]
    fn valid_instance_passes_schema() {
        let instance = json!({ "name": "Alice", "age": 30 });
        assert!(
            validate_value_against_schema(&instance, PERSON_SCHEMA).is_ok(),
            "valid instance should pass schema"
        );
    }

    #[test]
    fn missing_required_field_returns_schema_violation() {
        let instance = json!({ "name": "Bob" }); // age is missing
        let err = validate_value_against_schema(&instance, PERSON_SCHEMA)
            .expect_err("missing required field must fail schema validation");
        match err {
            ValidationError::SchemaViolation { violations, .. } => {
                assert!(!violations.is_empty(), "expected at least one violation");
                let combined = violations.join("\n");
                assert!(
                    combined.contains("age"),
                    "violation message should mention missing 'age': {combined}"
                );
            }
            other => panic!("expected SchemaViolation, got: {other}"),
        }
    }

    #[test]
    fn wrong_type_returns_schema_violation() {
        let instance = json!({ "name": "Carol", "age": "not-a-number" });
        let err = validate_value_against_schema(&instance, PERSON_SCHEMA)
            .expect_err("wrong type must fail schema validation");
        assert!(
            matches!(err, ValidationError::SchemaViolation { .. }),
            "expected SchemaViolation"
        );
    }

    #[test]
    fn additional_property_returns_schema_violation() {
        let instance = json!({ "name": "Dave", "age": 25, "extra": "not allowed" });
        let err = validate_value_against_schema(&instance, PERSON_SCHEMA)
            .expect_err("additional property must fail schema validation");
        assert!(
            matches!(err, ValidationError::SchemaViolation { .. }),
            "expected SchemaViolation"
        );
    }

    #[test]
    fn invalid_schema_json_returns_error() {
        let instance = json!({});
        let err = validate_value_against_schema(&instance, "not valid json {{{")
            .expect_err("bad schema JSON must return an error");
        assert!(
            matches!(err, ValidationError::SchemaViolation { .. }),
            "expected SchemaViolation for unparseable schema"
        );
    }
}
