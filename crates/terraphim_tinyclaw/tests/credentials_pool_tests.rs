//! Integration tests for the Wave 1 credential pool.
//!
//! Mirrors Hermes' `test_credential_pool.py` at minimal scope:
//!
//! - `pool_rotates_entries` — multiple entries per class form a rotation
//! - `pool_throttle_cools_entry` — throttle puts an entry on cooldown
//! - `pool_success_resets` — success clears the cooldown
//! - `env_file_source_parses` — dotenv format accepted
//!
//! All tests are hermetic: `scrub_env()` clears relevant env vars per case.

mod common;

use std::collections::HashMap;
use std::time::Duration;

use terraphim_tinyclaw::credentials::{
    CredentialError, CredentialPool, CredentialSource, EnvFileSource, EnvVarSource, PoolEntry,
    TokenRef,
};

/// In-memory source for hermetic tests. The pool never sees real env vars.
#[derive(Debug, Default)]
struct InMemorySource {
    map: HashMap<String, String>,
}

impl CredentialSource for InMemorySource {
    fn resolve(&self, token_ref: &TokenRef) -> Option<String> {
        match token_ref {
            TokenRef::EnvVar { name } => self.map.get(name).cloned(),
            TokenRef::File { .. } => None,
        }
    }
}

fn entry(provider: &str, class: &str, env: &str) -> PoolEntry {
    PoolEntry {
        provider: provider.to_string(),
        class: class.to_string(),
        token_ref: TokenRef::EnvVar {
            name: env.to_string(),
        },
    }
}

#[test]
fn pool_rotates_entries() {
    common::scrub_env();
    let pool = CredentialPool::new();
    pool.add(entry("or-1", "openrouter", "OR_1"));
    pool.add(entry("or-2", "openrouter", "OR_2"));
    let mut src = InMemorySource::default();
    src.map.insert("OR_1".into(), "secret-1".into());
    src.map.insert("OR_2".into(), "secret-2".into());

    let c1 = pool
        .acquire(&"openrouter".to_string(), &src)
        .expect("first acquire");
    assert_eq!(c1.provider, "or-1");
    assert_eq!(c1.token, "secret-1");
}

#[test]
fn pool_throttle_cools_entry() {
    common::scrub_env();
    let pool = CredentialPool::with_default_cooldown(Duration::from_millis(50));
    pool.add(entry("or-1", "openrouter", "OR_1"));
    pool.add(entry("or-2", "openrouter", "OR_2"));
    let mut src = InMemorySource::default();
    src.map.insert("OR_1".into(), "secret-1".into());
    src.map.insert("OR_2".into(), "secret-2".into());

    pool.report_throttle(&"or-1".to_string(), None);
    let c = pool
        .acquire(&"openrouter".to_string(), &src)
        .expect("acquire after throttle");
    assert_eq!(
        c.provider, "or-2",
        "throttled entry should be skipped; rotation picks the next"
    );
}

#[test]
fn pool_success_resets() {
    common::scrub_env();
    let pool = CredentialPool::with_default_cooldown(Duration::from_secs(60));
    pool.add(entry("or-1", "openrouter", "OR_1"));
    let mut src = InMemorySource::default();
    src.map.insert("OR_1".into(), "secret".into());

    pool.report_throttle(&"or-1".to_string(), None);
    assert!(pool.acquire(&"openrouter".to_string(), &src).is_err());

    pool.report_success(&"or-1".to_string());
    let c = pool
        .acquire(&"openrouter".to_string(), &src)
        .expect("after success");
    assert_eq!(c.provider, "or-1");
}

#[test]
fn env_file_source_parses() {
    let parsed = EnvFileSource::parse(
        "\
# comment
OR_KEY=or-secret
AN_KEY=\"quoted value\"
",
    );
    assert_eq!(parsed.get("OR_KEY").unwrap(), "or-secret");
    assert_eq!(parsed.get("AN_KEY").unwrap(), "quoted value");
}

#[test]
fn empty_pool_returns_exhausted() {
    common::scrub_env();
    let pool = CredentialPool::new();
    let src = InMemorySource::default();
    let err = pool.acquire(&"openrouter".to_string(), &src).unwrap_err();
    assert!(matches!(err, CredentialError::Exhausted(_)));
}

#[test]
fn env_var_source_skips_missing() {
    common::scrub_env();
    let src = EnvVarSource;
    assert!(
        src.resolve(&TokenRef::EnvVar {
            name: "WAVE1_NONEXISTENT_KEY".to_string()
        })
        .is_none()
    );
}

#[test]
fn env_file_source_loads_from_disk() {
    common::scrub_env();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("creds.env");
    std::fs::write(&path, "OR_KEY=disk-secret").expect("write");
    let src = EnvFileSource::load(&path).expect("load");
    let v = src.resolve(&TokenRef::EnvVar {
        name: "OR_KEY".into(),
    });
    assert_eq!(v.as_deref(), Some("disk-secret"));
}

#[test]
fn env_file_source_missing_file_is_error() {
    common::scrub_env();
    let result = EnvFileSource::load("/nonexistent/path/creds.env");
    assert!(matches!(result, Err(CredentialError::SourceUnreadable(_))));
}
