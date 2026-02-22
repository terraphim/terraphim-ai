//! Anthropic API key creation from OAuth access tokens.
//!
//! After obtaining an OAuth access token via the Claude PKCE flow,
//! this module can create a permanent API key using Anthropic's
//! `create_api_key` endpoint. The API key uses standard `x-api-key`
//! auth and works with existing genai infrastructure.

use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::oauth::error::{OAuthError, OAuthResult};

/// Anthropic API key creation endpoint.
const CREATE_API_KEY_ENDPOINT: &str =
    "https://api.anthropic.com/api/oauth/claude_cli/create_api_key";

/// Response from the Anthropic create_api_key endpoint.
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyResponse {
    /// The created API key value (e.g. "sk-ant-api03-...")
    pub api_key: String,
    /// Display name of the created key
    #[serde(default)]
    pub name: Option<String>,
}

/// Create an Anthropic API key from an OAuth access token.
///
/// Calls the Anthropic `create_api_key` endpoint with the Bearer token
/// to generate a permanent API key that can be used with standard
/// `x-api-key` authentication.
///
/// Requires the `org:create_api_key` scope in the OAuth token.
pub async fn create_anthropic_api_key(
    http_client: &reqwest::Client,
    access_token: &str,
    key_name: &str,
) -> OAuthResult<String> {
    info!(key_name = %key_name, "Creating Anthropic API key from OAuth token");

    let response = http_client
        .post(CREATE_API_KEY_ENDPOINT)
        .bearer_auth(access_token)
        .json(&serde_json::json!({ "name": key_name }))
        .send()
        .await
        .map_err(OAuthError::HttpError)?;

    let status = response.status();
    let body = response.text().await.map_err(OAuthError::HttpError)?;

    if !status.is_success() {
        warn!(
            status = %status,
            body = %body,
            "Failed to create Anthropic API key"
        );
        return Err(OAuthError::FlowFailed(format!(
            "API key creation failed (HTTP {}): {}",
            status, body
        )));
    }

    let key_response: CreateApiKeyResponse = serde_json::from_str(&body).map_err(|e| {
        OAuthError::FlowFailed(format!("Failed to parse create_api_key response: {}", e))
    })?;

    debug!("Successfully created Anthropic API key");

    Ok(key_response.api_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_api_key_response_deserialization() {
        let json = r#"{"api_key": "sk-ant-api03-test-key-123", "name": "terraphim-proxy"}"#;
        let response: CreateApiKeyResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.api_key, "sk-ant-api03-test-key-123");
        assert_eq!(response.name, Some("terraphim-proxy".to_string()));
    }

    #[test]
    fn test_create_api_key_response_minimal() {
        let json = r#"{"api_key": "sk-ant-api03-minimal"}"#;
        let response: CreateApiKeyResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.api_key, "sk-ant-api03-minimal");
        assert!(response.name.is_none());
    }

    #[test]
    fn test_create_api_key_endpoint_constant() {
        assert!(CREATE_API_KEY_ENDPOINT.starts_with("https://api.anthropic.com"));
        assert!(CREATE_API_KEY_ENDPOINT.contains("create_api_key"));
    }
}
