/*!
# Terraphim 1Password CLI Integration

This crate provides secure integration with 1Password CLI for secret management in Terraphim AI.
*/

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::sync::LazyLock;
use thiserror::Error;
use tokio::process::Command;

/// Errors that can occur during 1Password operations
#[derive(Error, Debug)]
pub enum OnePasswordError {
    #[error(
        "1Password CLI not found. Please install from https://developer.1password.com/docs/cli/get-started/"
    )]
    CliNotFound,

    #[error("Not authenticated with 1Password CLI. Run: op signin")]
    NotAuthenticated,

    #[error("Invalid 1Password reference format: {reference}")]
    InvalidReference { reference: String },

    #[error("Secret not found: {reference}")]
    SecretNotFound { reference: String },

    #[error("1Password CLI command failed: {message}")]
    CommandFailed { message: String },

    #[error("Permission denied accessing vault or item: {reference}")]
    PermissionDenied { reference: String },

    #[error("JSON parsing error: {source}")]
    JsonError {
        #[from]
        source: serde_json::Error,
    },

    #[error("IO error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },
}

/// Result type for 1Password operations
pub type OnePasswordResult<T> = Result<T, OnePasswordError>;

/// Trait for loading secrets from various backends
#[async_trait::async_trait]
pub trait SecretLoader {
    /// Resolve a single secret reference to its actual value
    async fn resolve_secret(&self, reference: &str) -> OnePasswordResult<String>;

    /// Process a configuration string, resolving all 1Password references
    async fn process_config(&self, config: &str) -> OnePasswordResult<String>;

    /// Check if the secret backend is available and authenticated
    async fn is_available(&self) -> bool;
}

/// 1Password CLI implementation of SecretLoader
#[derive(Debug, Clone)]
pub struct OnePasswordLoader {
    timeout_seconds: u64,
}

impl Default for OnePasswordLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl OnePasswordLoader {
    /// Create a new 1Password loader with default settings
    pub fn new() -> Self {
        Self {
            timeout_seconds: 30,
        }
    }

    /// Check if 1Password CLI is installed
    pub async fn check_cli_installed(&self) -> bool {
        Command::new("op")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|status| status.success())
            .unwrap_or(false)
    }

    /// Check if authenticated with 1Password CLI
    pub async fn check_authenticated(&self) -> bool {
        Command::new("op")
            .arg("account")
            .arg("list")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|status| status.success())
            .unwrap_or(false)
    }

    /// Parse a 1Password reference into its components
    pub fn parse_reference(&self, reference: &str) -> OnePasswordResult<OnePasswordRef> {
        static OP_REGEX: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^op://([^/]+)/([^/]+)/([^/]+)$").expect("Invalid regex"));

        if !reference.starts_with("op://") {
            return Err(OnePasswordError::InvalidReference {
                reference: reference.to_string(),
            });
        }

        let captures =
            OP_REGEX
                .captures(reference)
                .ok_or_else(|| OnePasswordError::InvalidReference {
                    reference: reference.to_string(),
                })?;

        Ok(OnePasswordRef {
            vault: captures[1].to_string(),
            item: captures[2].to_string(),
            field: captures[3].to_string(),
        })
    }
}

#[async_trait::async_trait]
impl SecretLoader for OnePasswordLoader {
    async fn resolve_secret(&self, reference: &str) -> OnePasswordResult<String> {
        if !reference.starts_with("op://") {
            return Ok(reference.to_string());
        }

        if !self.check_cli_installed().await {
            return Err(OnePasswordError::CliNotFound);
        }

        if !self.check_authenticated().await {
            return Err(OnePasswordError::NotAuthenticated);
        }

        let op_ref = self.parse_reference(reference)?;

        let output = tokio::time::timeout(
            tokio::time::Duration::from_secs(self.timeout_seconds),
            Command::new("op")
                .arg("item")
                .arg("get")
                .arg(&op_ref.item)
                .arg("--vault")
                .arg(&op_ref.vault)
                .arg("--field")
                .arg(&op_ref.field)
                .output(),
        )
        .await
        .map_err(|_| OnePasswordError::CommandFailed {
            message: format!("Command timed out after {} seconds", self.timeout_seconds),
        })?
        .map_err(|e| OnePasswordError::CommandFailed {
            message: format!("Failed to execute op command: {}", e),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            if stderr.contains("not found") {
                return Err(OnePasswordError::SecretNotFound {
                    reference: reference.to_string(),
                });
            }

            if stderr.contains("permission denied") {
                return Err(OnePasswordError::PermissionDenied {
                    reference: reference.to_string(),
                });
            }

            if stderr.contains("not signed in") {
                return Err(OnePasswordError::NotAuthenticated);
            }

            return Err(OnePasswordError::CommandFailed {
                message: stderr.to_string(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim().to_string())
    }

    async fn process_config(&self, config: &str) -> OnePasswordResult<String> {
        static OP_REGEX: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#""op://[^"]+""#).expect("Invalid regex"));

        let mut result = config.to_string();
        let matches: Vec<_> = OP_REGEX.find_iter(config).collect();

        for mat in matches {
            let reference = &config[mat.start() + 1..mat.end() - 1];

            match self.resolve_secret(reference).await {
                Ok(value) => {
                    result =
                        result.replace(&format!("\"{}\"", reference), &format!("\"{}\"", value));
                }
                Err(e) => {
                    log::error!("Failed to resolve secret {}: {}", reference, e);
                    return Err(e);
                }
            }
        }

        Ok(result)
    }

    async fn is_available(&self) -> bool {
        self.check_cli_installed().await && self.check_authenticated().await
    }
}

/// Environment variable fallback implementation
#[derive(Debug, Clone, Default)]
pub struct EnvironmentLoader;

impl EnvironmentLoader {
    /// Creates a new `EnvironmentLoader`.
    pub fn new() -> Self {
        Self
    }

    fn reference_to_env_var(&self, reference: &str) -> OnePasswordResult<String> {
        if !reference.starts_with("op://") {
            return Ok(reference.to_string());
        }

        static OP_REGEX: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^op://([^/]+)/([^/]+)/([^/]+)$").expect("Invalid regex"));

        let captures =
            OP_REGEX
                .captures(reference)
                .ok_or_else(|| OnePasswordError::InvalidReference {
                    reference: reference.to_string(),
                })?;

        let vault = captures[1].to_uppercase().replace("-", "_");
        let item = captures[2].to_uppercase().replace("-", "_");
        let field = captures[3].to_uppercase();

        Ok(format!("{}_{}_{}", vault, item, field))
    }
}

#[async_trait::async_trait]
impl SecretLoader for EnvironmentLoader {
    async fn resolve_secret(&self, reference: &str) -> OnePasswordResult<String> {
        if !reference.starts_with("op://") {
            return Ok(reference.to_string());
        }

        let env_var = self.reference_to_env_var(reference)?;

        std::env::var(&env_var).map_err(|_| OnePasswordError::SecretNotFound {
            reference: format!("{} (env var: {})", reference, env_var),
        })
    }

    async fn process_config(&self, config: &str) -> OnePasswordResult<String> {
        static OP_REGEX: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#""op://[^"]+""#).expect("Invalid regex"));

        let mut result = config.to_string();
        let matches: Vec<_> = OP_REGEX.find_iter(config).collect();

        for mat in matches {
            let reference = &config[mat.start() + 1..mat.end() - 1];

            if let Ok(value) = self.resolve_secret(reference).await {
                result = result.replace(&format!("\"{}\"", reference), &format!("\"{}\"", value));
            }
        }

        Ok(result)
    }

    async fn is_available(&self) -> bool {
        true
    }
}

/// Parsed 1Password reference
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnePasswordRef {
    pub vault: String,
    pub item: String,
    pub field: String,
}

impl std::fmt::Display for OnePasswordRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "op://{}/{}/{}", self.vault, self.item, self.field)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- OnePasswordRef parsing tests ---

    #[test]
    fn test_parse_valid_reference() {
        let loader = OnePasswordLoader::new();
        let parsed = loader
            .parse_reference("op://my-vault/my-item/my-field")
            .unwrap();
        assert_eq!(parsed.vault, "my-vault");
        assert_eq!(parsed.item, "my-item");
        assert_eq!(parsed.field, "my-field");
    }

    #[test]
    fn test_parse_reference_display_round_trip() {
        let loader = OnePasswordLoader::new();
        let reference = "op://vault/item/field";
        let parsed = loader.parse_reference(reference).unwrap();
        assert_eq!(parsed.to_string(), reference);
    }

    #[test]
    fn test_parse_reference_missing_prefix() {
        let loader = OnePasswordLoader::new();
        let result = loader.parse_reference("vault/item/field");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_reference_missing_field() {
        let loader = OnePasswordLoader::new();
        let result = loader.parse_reference("op://vault/item");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_reference_extra_components() {
        let loader = OnePasswordLoader::new();
        let result = loader.parse_reference("op://vault/item/field/extra");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_reference_empty_components() {
        let loader = OnePasswordLoader::new();
        // Regex requires at least one char per component
        let result = loader.parse_reference("op:////");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_reference_with_special_chars() {
        let loader = OnePasswordLoader::new();
        let parsed = loader
            .parse_reference("op://my-vault_123/my.item/api-key")
            .unwrap();
        assert_eq!(parsed.vault, "my-vault_123");
        assert_eq!(parsed.item, "my.item");
        assert_eq!(parsed.field, "api-key");
    }

    #[test]
    fn test_onepassword_ref_equality() {
        let ref1 = OnePasswordRef {
            vault: "v".to_string(),
            item: "i".to_string(),
            field: "f".to_string(),
        };
        let ref2 = OnePasswordRef {
            vault: "v".to_string(),
            item: "i".to_string(),
            field: "f".to_string(),
        };
        assert_eq!(ref1, ref2);
    }

    #[test]
    fn test_onepassword_ref_serde_round_trip() {
        let op_ref = OnePasswordRef {
            vault: "my-vault".to_string(),
            item: "my-item".to_string(),
            field: "password".to_string(),
        };
        let json = serde_json::to_string(&op_ref).unwrap();
        let deserialized: OnePasswordRef = serde_json::from_str(&json).unwrap();
        assert_eq!(op_ref, deserialized);
    }

    // --- EnvironmentLoader tests ---

    #[test]
    fn test_reference_to_env_var_basic() {
        let loader = EnvironmentLoader::new();
        let env_var = loader
            .reference_to_env_var("op://my-vault/my-item/password")
            .unwrap();
        assert_eq!(env_var, "MY_VAULT_MY_ITEM_PASSWORD");
    }

    #[test]
    fn test_reference_to_env_var_hyphens_replaced() {
        let loader = EnvironmentLoader::new();
        let env_var = loader
            .reference_to_env_var("op://my-vault/my-item/api-key")
            .unwrap();
        assert_eq!(env_var, "MY_VAULT_MY_ITEM_API-KEY");
    }

    #[test]
    fn test_reference_to_env_var_non_reference_passthrough() {
        let loader = EnvironmentLoader::new();
        let result = loader.reference_to_env_var("plain-text").unwrap();
        assert_eq!(result, "plain-text");
    }

    #[test]
    fn test_reference_to_env_var_invalid_format() {
        let loader = EnvironmentLoader::new();
        let result = loader.reference_to_env_var("op://vault-only");
        assert!(result.is_err());
    }

    // --- OnePasswordLoader constructor tests ---

    #[test]
    fn test_loader_default_timeout() {
        let loader = OnePasswordLoader::new();
        assert_eq!(loader.timeout_seconds, 30);
    }

    #[test]
    fn test_loader_default_trait() {
        let loader = OnePasswordLoader::default();
        assert_eq!(loader.timeout_seconds, 30);
    }

    // --- Error display tests ---

    #[test]
    fn test_error_display_cli_not_found() {
        let err = OnePasswordError::CliNotFound;
        let msg = err.to_string();
        assert!(msg.contains("1Password CLI not found"));
    }

    #[test]
    fn test_error_display_not_authenticated() {
        let err = OnePasswordError::NotAuthenticated;
        let msg = err.to_string();
        assert!(msg.contains("Not authenticated"));
    }

    #[test]
    fn test_error_display_invalid_reference() {
        let err = OnePasswordError::InvalidReference {
            reference: "bad-ref".to_string(),
        };
        assert!(err.to_string().contains("bad-ref"));
    }

    #[test]
    fn test_error_display_secret_not_found() {
        let err = OnePasswordError::SecretNotFound {
            reference: "op://v/i/f".to_string(),
        };
        assert!(err.to_string().contains("op://v/i/f"));
    }

    // --- SecretLoader trait tests for EnvironmentLoader ---

    #[tokio::test]
    async fn test_env_loader_non_reference_passthrough() {
        let loader = EnvironmentLoader::new();
        let result = loader.resolve_secret("not-a-reference").await.unwrap();
        assert_eq!(result, "not-a-reference");
    }

    #[tokio::test]
    async fn test_env_loader_resolve_from_env() {
        let loader = EnvironmentLoader::new();
        // Set a test env var
        // SAFETY: test is single-threaded and env var is unique to this test
        unsafe {
            std::env::set_var("TEST_VAULT_TEST_ITEM_API_KEY", "secret123");
        }
        let result = loader
            .resolve_secret("op://test-vault/test-item/api_key")
            .await
            .unwrap();
        assert_eq!(result, "secret123");
        unsafe {
            std::env::remove_var("TEST_VAULT_TEST_ITEM_API_KEY");
        }
    }

    #[tokio::test]
    async fn test_env_loader_missing_env_var() {
        let loader = EnvironmentLoader::new();
        let result = loader.resolve_secret("op://nonexistent/vault/field").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_env_loader_is_always_available() {
        let loader = EnvironmentLoader::new();
        assert!(loader.is_available().await);
    }

    #[tokio::test]
    async fn test_env_loader_process_config_no_references() {
        let loader = EnvironmentLoader::new();
        let config = r#"{"key": "plain-value"}"#;
        let result = loader.process_config(config).await.unwrap();
        assert_eq!(result, config);
    }

    #[tokio::test]
    async fn test_env_loader_process_config_with_reference() {
        let loader = EnvironmentLoader::new();
        // SAFETY: test is single-threaded and env var is unique to this test
        unsafe {
            std::env::set_var("MYVAULT_MYITEM_TOKEN", "resolved-token");
        }
        let config = r#"{"api_token": "op://myvault/myitem/token"}"#;
        let result = loader.process_config(config).await.unwrap();
        assert_eq!(result, r#"{"api_token": "resolved-token"}"#);
        unsafe {
            std::env::remove_var("MYVAULT_MYITEM_TOKEN");
        }
    }

    // --- OnePasswordLoader resolve_secret passthrough ---

    #[tokio::test]
    async fn test_op_loader_non_reference_passthrough() {
        let loader = OnePasswordLoader::new();
        let result = loader.resolve_secret("plain-value").await.unwrap();
        assert_eq!(result, "plain-value");
    }
}
