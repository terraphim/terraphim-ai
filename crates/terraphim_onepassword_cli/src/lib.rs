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
    #[error("1Password CLI not found. Please install from https://developer.1password.com/docs/cli/get-started/")]
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
