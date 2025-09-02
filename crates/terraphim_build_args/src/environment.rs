/// Environment management for build configurations
///
/// This module provides functionality for managing build environments in the
/// Terraphim AI project, allowing for configuration of environment-specific
/// settings, environment validation, and loading environment variables.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Different environments that can be configured
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Environment {
    /// Name of the environment (e.g., development, production)
    pub name: String,

    /// Key-value pairs for environment variables
    pub variables: HashMap<String, String>,

    /// Description or metadata about the environment
    pub description: Option<String>,
}

impl Environment {
    /// Creates a new default environment
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an environment from a name
    pub fn from_name(name: &str) -> Self {
        Self {
            name: name.to_string(),
            variables: HashMap::new(),
            description: None,
        }
    }

    /// Sets an environment variable
    pub fn set_variable(&mut self, key: &str, value: &str) {
        self.variables.insert(key.to_string(), value.to_string());
    }

    /// Loads environment variables from a file (e.g., .env)
    pub fn load_from_file(&mut self, path: &str) -> Result<()> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::IoError(format!("Failed to read env file {}: {}", path, e)))?;

        for line in content.lines() {
            if line.trim().is_empty() || line.trim().starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                self.set_variable(key.trim(), value.trim());
            }
        }
        Ok(())
    }

    /// Validates the environment configuration
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(Error::environment("Environment name cannot be empty"));
        }
        Ok(())
    }

    /// Merges another environment into this one
    pub fn merge(&mut self, other: &Environment) {
        for (key, value) in &other.variables {
            self.set_variable(key, value);
        }
    }
}

impl std::str::FromStr for Environment {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(Environment::from_name(s))
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Environment: {}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_environment_creation() {
        let mut env = Environment::from_name("development");
        env.set_variable("KEY", "value");

        assert_eq!(env.name, "development");
        assert_eq!(env.variables.get("KEY"), Some(&"value".to_string()));
    }

    #[test]
    fn test_environment_file_loading() -> Result<()> {
        let mut env = Environment::new();
        let mut file = NamedTempFile::new()?;
        writeln!(file, "API_URL=http://localhost")?;
        writeln!(file, "DEBUG=true")?;
        env.load_from_file(file.path().to_str().unwrap())?;

        assert_eq!(
            env.variables.get("API_URL"),
            Some(&"http://localhost".to_string())
        );
        assert_eq!(env.variables.get("DEBUG"), Some(&"true".to_string()));
        Ok(())
    }

    #[test]
    fn test_environment_merge() {
        let mut env1 = Environment::from_name("prod");
        env1.set_variable("DB_HOST", "prod-db");

        let mut env2 = Environment::from_name("staging");
        env2.set_variable("DB_HOST", "staging-db");
        env2.set_variable("LOG_LEVEL", "info");

        env1.merge(&env2);

        // Staging variable should overwrite existing variables
        assert_eq!(
            env1.variables.get("DB_HOST"),
            Some(&"staging-db".to_string())
        );
        assert_eq!(env1.variables.get("LOG_LEVEL"), Some(&"info".to_string()));
    }

    #[test]
    fn test_invalid_environment_name() {
        let env = Environment::from_name("");
        assert!(env.validate().is_err());
    }
}
