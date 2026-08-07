//! Built-in credential sources.
//!
//! Wave 1 ships two sources that cover the vast majority of real-world
//! deployments:
//!
//! - `EnvVarSource` — reads from `std::env::var(name)`. This is the
//!   path tinyclaw used before the pool existed; it's preserved as the
//!   fallback when `credentials.enabled = false`.
//! - `EnvFileSource` — parses a `KEY=VALUE` file at construction time
//!   (dotenv-style). Matches Hermes' `~/.hermes/.env` convention.
//!
//! Custom sources (1Password CLI, Vault, AWS Secrets Manager) can be
//! plugged in by implementing the `CredentialSource` trait.

use std::collections::HashMap;
use std::path::PathBuf;

use super::pool::{CredentialError, CredentialSource, TokenRef};

/// Default credential source: env-var lookups only.
///
/// Cannot resolve `TokenRef::File` — those entries are skipped. Use
/// `EnvFileSource` if file-backed credentials are in play.
#[derive(Debug, Default, Clone)]
pub struct EnvVarSource;

impl EnvVarSource {
    /// Construct a new env-var source.
    pub fn new() -> Self {
        Self
    }
}

impl CredentialSource for EnvVarSource {
    fn resolve(&self, token_ref: &TokenRef) -> Option<String> {
        match token_ref {
            TokenRef::EnvVar { name } => std::env::var(name).ok(),
            TokenRef::File { .. } => None,
        }
    }
}

/// Default credential source: parses a `KEY=VALUE` file.
///
/// Line format (matches `dotenv`):
/// - `KEY=value`
/// - `KEY="quoted value"` — surrounding double or single quotes are stripped
/// - `# comment` and blank lines are skipped
/// - `export KEY=value` — optional `export` prefix is stripped
///
/// Parsing is done at construction time. The parsed map is cached for the
/// source's lifetime. Tests can swap the file once at construction; there is
/// no `reload()` method (Wave 1 scope).
#[derive(Debug, Clone)]
pub struct EnvFileSource {
    pairs: HashMap<String, String>,
    /// Path the file was loaded from. Retained for diagnostics and for
    /// `TokenRef::File` resolution when the token_ref points at this source's
    /// own path.
    path: PathBuf,
}

impl EnvFileSource {
    /// Load a `KEY=VALUE` file from disk. Returns an error if the file
    /// cannot be read; missing keys are NOT errors (they're just absent
    /// from the parsed map).
    pub fn load(path: impl Into<PathBuf>) -> Result<Self, CredentialError> {
        let path = path.into();
        let content = std::fs::read_to_string(&path)
            .map_err(|e| CredentialError::SourceUnreadable(format!("{}: {}", path.display(), e)))?;
        let pairs = Self::parse(&content);
        Ok(Self { pairs, path })
    }

    /// Parse the env-file content. Public so tests can construct sources
    /// without touching disk.
    pub fn parse(content: &str) -> HashMap<String, String> {
        let mut out = HashMap::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            // Strip optional `export ` prefix.
            let stripped = trimmed.strip_prefix("export ").unwrap_or(trimmed);
            if let Some((k, v)) = stripped.split_once('=') {
                let key = k.trim().to_string();
                let val = v.trim();
                // Strip surrounding double or single quotes if present.
                let val = if (val.starts_with('"') && val.ends_with('"') && val.len() >= 2)
                    || (val.starts_with('\'') && val.ends_with('\'') && val.len() >= 2)
                {
                    &val[1..val.len() - 1]
                } else {
                    val
                };
                out.insert(key, val.to_string());
            }
        }
        out
    }
}

impl CredentialSource for EnvFileSource {
    fn resolve(&self, token_ref: &TokenRef) -> Option<String> {
        match token_ref {
            TokenRef::EnvVar { name } => self.pairs.get(name).cloned(),
            TokenRef::File { path } => {
                if path == &self.path {
                    // Whole-file semantics: return the first key's value for
                    // simplicity. Consumers needing the full file should iterate
                    // `pairs` directly (not exposed yet; Wave 6 candidate).
                    self.pairs.values().next().cloned()
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_var_source_returns_present_var() {
        let src = EnvVarSource;
        // SAFETY: test-only env mutation, single-threaded test runner per
        // Wave 0 hermetic scrubber convention.
        unsafe {
            std::env::set_var("WAVE1_TEST_KEY", "present");
        }
        let resolved = src.resolve(&TokenRef::EnvVar {
            name: "WAVE1_TEST_KEY".into(),
        });
        assert_eq!(resolved.as_deref(), Some("present"));
        unsafe {
            std::env::remove_var("WAVE1_TEST_KEY");
        }
    }

    #[test]
    fn env_var_source_skips_missing() {
        let src = EnvVarSource;
        unsafe {
            std::env::remove_var("WAVE1_DEFINITELY_NOT_SET");
        }
        assert!(
            src.resolve(&TokenRef::EnvVar {
                name: "WAVE1_DEFINITELY_NOT_SET".into()
            })
            .is_none()
        );
    }

    #[test]
    fn env_var_source_cannot_read_files() {
        let src = EnvVarSource;
        assert!(
            src.resolve(&TokenRef::File {
                path: PathBuf::from("/tmp/x.env")
            })
            .is_none()
        );
    }

    #[test]
    fn env_file_source_parses_keyvalue_lines() {
        let parsed = EnvFileSource::parse(
            "\
# comment line
OR_KEY=or-secret
AN_KEY=\"quoted value\"

export ZED_KEY='single quoted'
",
        );
        assert_eq!(parsed.get("OR_KEY").unwrap(), "or-secret");
        assert_eq!(parsed.get("AN_KEY").unwrap(), "quoted value");
        assert_eq!(parsed.get("ZED_KEY").unwrap(), "single quoted");
    }

    #[test]
    fn env_file_source_loads_from_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("creds.env");
        std::fs::write(&path, "OR_KEY=disk-secret").expect("write");
        let src = EnvFileSource::load(&path).expect("load");
        let resolved = src.resolve(&TokenRef::EnvVar {
            name: "OR_KEY".into(),
        });
        assert_eq!(resolved.as_deref(), Some("disk-secret"));
    }

    #[test]
    fn env_file_source_missing_file_is_error() {
        let src = EnvFileSource::load("/nonexistent/path/creds.env");
        assert!(matches!(src, Err(CredentialError::SourceUnreadable(_))));
    }
}
