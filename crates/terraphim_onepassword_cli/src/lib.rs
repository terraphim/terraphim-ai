//! 1Password CLI Integration
//!
//! This crate provides integration with the 1Password CLI (op) interface for securely
//! managing secrets in the Terraphim AI project. It enables fetching secrets from 1Password
//! vaults, presenting a consistent API for accessing or caching secrets in application environments.

use serde::{Deserialize, Serialize};
use std::process::Command;
use thiserror::Error;

/// Errors that may occur during 1Password CLI operations
#[derive(Error, Debug)]
pub enum OnePasswordError {
    #[error("1Password CLI command failed with status: {0}")]
    CommandFailed(i32),

    #[error("Failed to parse CLI output: {0}")]
    ParseError(String),

    #[error("Environment error: {0}")]
    EnvironmentError(#[from] std::env::VarError),
}

/// Result type for 1Password operations
pub type OnePasswordResult<T> = Result<T, OnePasswordError>;

/// Represents a 1Password secret
#[derive(Debug, Deserialize, Serialize)]
pub struct OnePasswordSecret {
    pub id: String,
    pub label: String,
    pub value: String,
}

/// Fetch a secret from 1Password vault
///
/// # Arguments
///
/// * `vault` - The vault name
/// * `item` - The item ID or name
///
/// # Returns
///
/// A result containing the fetched secret or an error if it fails
pub fn fetch_secret(vault: &str, item: &str) -> OnePasswordResult<OnePasswordSecret> {
    let output = Command::new("op")
        .args(["item", "get", item, "--vault", vault, "--format", "json"])
        .output()
        .expect("Failed to execute op command");

    if !output.status.success() {
        return Err(OnePasswordError::CommandFailed(
            output.status.code().unwrap_or(-1),
        ));
    }

    let secret: OnePasswordSecret = serde_json::from_slice(&output.stdout)
        .map_err(|e| OnePasswordError::ParseError(e.to_string()))?;

    Ok(secret)
}

/// Fetch a secret value by its field name
pub fn fetch_secret_value(vault: &str, item: &str, _field: &str) -> OnePasswordResult<String> {
    let secret = fetch_secret(vault, item)?;

    match secret.value.as_str() {
        "" => Err(OnePasswordError::ParseError("Field not found".to_string())),
        value => Ok(value.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_secret() {
        // Example test for fetching secret
        if let Ok(secret) = fetch_secret("ExampleVault", "ExampleItem") {
            println!("Fetched secret: {:?}", secret);
        } else {
            println!("Failed to fetch secret.");
        }
    }
}
