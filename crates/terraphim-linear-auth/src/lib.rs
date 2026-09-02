//! Auth — load the Linear API key without leaking it into logs or errors.
//!
//! Priority (per design v2 §8):
//!   1. `~/.linearctl/config.json` -> `profiles.default.apiKey`
//!   2. `$LINEAR_API_KEY`
//!   3. `op read op://Private/linear-api-key-all-teams/password` ONLY if stdout is a TTY
//!
//! In all cases, the returned key is wrapped in [`KeyRedacted`] so the
//! `Debug` impl never prints the key bytes.

use std::fmt;
use std::io::IsTerminal;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("no Linear API key found (tried ~/.linearctl/config.json, $LINEAR_API_KEY, op read)")]
    Missing,
    #[error("io error reading auth source: {0}")]
    Io(#[from] std::io::Error),
    #[error("~/.linearctl/config.json malformed: {0}")]
    ConfigParse(#[from] serde_json::Error),
    #[error("op read returned no key")]
    OpEmpty,
}

/// Wrapper around the API key that prints `[REDACTED]` in `Debug`.
#[derive(Clone)]
pub struct KeyRedacted(String);

impl KeyRedacted {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for KeyRedacted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl fmt::Display for KeyRedacted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct LinearctlConfig {
    /// The currently-active API key (what `lc` actually uses). Top-level field
    /// in `~/.linearctl/config.json` since lc 0.1.x. Prefer this when present.
    #[serde(rename = "currentKey")]
    current_key: Option<String>,
    #[serde(default)]
    profiles: std::collections::HashMap<String, LinearctlProfile>,
    #[serde(rename = "defaultProfile")]
    default_profile: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct LinearctlProfile {
    #[serde(rename = "apiKey")]
    api_key: Option<String>,
}

/// Write or update `~/.linearctl/config.json` with an API key.
///
/// If `profile` is `None`, writes to the top-level `currentKey` field (lc's
/// preferred location). If `Some(name)`, writes to `profiles[name].apiKey` and
/// sets `defaultProfile` to that name when it is the only profile.
pub fn init_config(profile: Option<&str>, api_key: &str) -> Result<(), AuthError> {
    let path = linearctl_path().ok_or(AuthError::Missing)?;
    let mut cfg = if path.exists() {
        let raw = std::fs::read_to_string(&path)?;
        serde_json::from_str::<LinearctlConfig>(&raw).unwrap_or_else(|_| LinearctlConfig {
            current_key: None,
            profiles: std::collections::HashMap::new(),
            default_profile: None,
        })
    } else {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        LinearctlConfig {
            current_key: None,
            profiles: std::collections::HashMap::new(),
            default_profile: None,
        }
    };

    match profile {
        None => cfg.current_key = Some(api_key.into()),
        Some(name) => {
            cfg.profiles.insert(
                name.into(),
                LinearctlProfile {
                    api_key: Some(api_key.into()),
                },
            );
            if cfg.default_profile.is_none() {
                cfg.default_profile = Some(name.into());
            }
        }
    }

    let raw = serde_json::to_string_pretty(&cfg)?;
    std::fs::write(&path, raw)?;
    Ok(())
}

/// List configured profile names.
pub fn list_profiles() -> Result<Vec<(String, bool)>, AuthError> {
    let path = match linearctl_path() {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path)?;
    let cfg: LinearctlConfig = serde_json::from_str(&raw)?;
    let default = cfg.default_profile.as_deref();
    let mut profiles: Vec<(String, bool)> = cfg
        .profiles
        .keys()
        .map(|name| (name.clone(), Some(name.as_str()) == default))
        .collect();
    profiles.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(profiles)
}

/// Delete a profile by name.
pub fn delete_profile(name: &str) -> Result<bool, AuthError> {
    let path = match linearctl_path() {
        Some(p) => p,
        None => return Ok(false),
    };
    if !path.exists() {
        return Ok(false);
    }
    let raw = std::fs::read_to_string(&path)?;
    let mut cfg: LinearctlConfig = serde_json::from_str(&raw)?;
    let removed = cfg.profiles.remove(name).is_some();
    if removed {
        if cfg.default_profile.as_deref() == Some(name) {
            cfg.default_profile = cfg.profiles.keys().next().cloned();
        }
        let raw = serde_json::to_string_pretty(&cfg)?;
        std::fs::write(&path, raw)?;
    }
    Ok(removed)
}

/// Set the default profile.
pub fn set_default_profile(name: &str) -> Result<(), AuthError> {
    let path = match linearctl_path() {
        Some(p) => p,
        None => return Err(AuthError::Missing),
    };
    if !path.exists() {
        return Err(AuthError::Missing);
    }
    let raw = std::fs::read_to_string(&path)?;
    let mut cfg: LinearctlConfig = serde_json::from_str(&raw)?;
    if !cfg.profiles.contains_key(name) {
        return Err(AuthError::Missing);
    }
    cfg.default_profile = Some(name.into());
    // Clear top-level currentKey so the default profile actually takes effect
    // (load_key prioritises currentKey over profiles[defaultProfile].apiKey).
    cfg.current_key = None;
    let raw = serde_json::to_string_pretty(&cfg)?;
    std::fs::write(&path, raw)?;
    Ok(())
}

/// Load the Linear API key. Tries three sources in order.
pub fn load_key() -> Result<KeyRedacted, AuthError> {
    // 1. ~/.linearctl/config.json
    if let Some(key) = load_from_linearctl_config()? {
        return Ok(KeyRedacted::new(key));
    }

    // 2. $LINEAR_API_KEY
    if let Ok(key) = std::env::var("LINEAR_API_KEY") {
        if !key.is_empty() {
            return Ok(KeyRedacted::new(key));
        }
    }

    // 3. `op read` — only in TTY (avoids the non-tty biometric trap
    //    documented in terraphim-rlm-preflight-notes).
    if std::io::stdout().is_terminal() {
        if let Ok(key) = try_op_read() {
            if !key.is_empty() {
                return Ok(KeyRedacted::new(key));
            }
        }
    }

    Err(AuthError::Missing)
}

fn linearctl_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    Some(home.join(".linearctl").join("config.json"))
}

fn load_from_linearctl_config() -> Result<Option<String>, AuthError> {
    let Some(path) = linearctl_path() else {
        return Ok(None);
    };
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path)?;
    let cfg: LinearctlConfig = serde_json::from_str(&raw)?;

    // Priority order matches `lc` itself (verified against linearctl 0.1.10
    // by reading ~/.linearctl/config.json):
    //   1. `currentKey` (top-level) — what `lc` actually uses for HTTP calls
    //   2. `profiles[defaultProfile].apiKey` — legacy fallback
    if let Some(k) = cfg.current_key {
        if !k.is_empty() {
            return Ok(Some(k));
        }
    }
    let default_name = cfg.default_profile.as_deref().unwrap_or("default");
    let key = cfg
        .profiles
        .get(default_name)
        .and_then(|p| p.api_key.clone());
    Ok(key)
}

fn try_op_read() -> Result<String, AuthError> {
    let out = std::process::Command::new("op")
        .args(["read", "op://Private/linear-api-key-all-teams/password"])
        .output()?;
    if !out.status.success() {
        return Err(AuthError::OpEmpty);
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Global mutex so HOME-mutating tests run one at a time.
/// (Tests in the same crate run in parallel by default; this serializes
/// tests that touch the process-global HOME env var.)
#[allow(dead_code)]
pub static HOME_LOCK: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Helper: write a fake config.json to a temp HOME and return the path.
    fn write_fake_config(home: &std::path::Path, json: &str) {
        let dir = home.join(".linearctl");
        std::fs::create_dir_all(&dir).unwrap();
        let mut f = std::fs::File::create(dir.join("config.json")).unwrap();
        f.write_all(json.as_bytes()).unwrap();
    }

    /// RAII guard that saves HOME, sets it to a temp dir, and restores on drop.
    /// Prevents test pollution (other tests setting HOME leak into us).
    struct FakeHome {
        prev: Option<String>,
    }
    impl FakeHome {
        fn new(dir: &std::path::Path) -> Self {
            let prev = std::env::var("HOME").ok();
            std::env::set_var("HOME", dir);
            Self { prev }
        }
    }
    impl Drop for FakeHome {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
        }
    }

    #[test]
    fn prefers_current_key_over_profile_api_key() {
        let _lock = HOME_LOCK.lock();
        // Verifies the auth.rs preference: top-level currentKey wins over
        // profiles[defaultProfile].apiKey (matches `lc` 0.1.10 behavior).
        let dir = tempfile::tempdir().unwrap();
        let _home = FakeHome::new(dir.path());
        write_fake_config(
            dir.path(),
            r#"{
                "currentKey": "current_key_wins",
                "profiles": { "default": { "apiKey": "profile_key_loses" } },
                "defaultProfile": "default"
            }"#,
        );
        let key = load_key().expect("key loaded");
        assert_eq!(key.expose(), "current_key_wins");
    }

    #[test]
    fn falls_back_to_profile_api_key_when_no_current_key() {
        let _lock = HOME_LOCK.lock();
        // Legacy config: no currentKey, only profiles[].apiKey.
        let dir = tempfile::tempdir().unwrap();
        let _home = FakeHome::new(dir.path());
        write_fake_config(
            dir.path(),
            r#"{
                "profiles": { "default": { "apiKey": "legacy_profile_key" } },
                "defaultProfile": "default"
            }"#,
        );
        let key = load_key().expect("key loaded");
        assert_eq!(key.expose(), "legacy_profile_key");
    }

    #[test]
    fn respects_default_profile_field() {
        let _lock = HOME_LOCK.lock();
        // When currentKey is missing and defaultProfile points to a non-default
        // profile, we should read that profile's apiKey.
        let dir = tempfile::tempdir().unwrap();
        let _home = FakeHome::new(dir.path());
        write_fake_config(
            dir.path(),
            r#"{
                "profiles": {
                    "default": { "apiKey": "default_key" },
                    "work": { "apiKey": "work_key" }
                },
                "defaultProfile": "work"
            }"#,
        );
        let key = load_key().expect("key loaded");
        assert_eq!(key.expose(), "work_key");
    }

    #[test]
    fn key_redacted_debug_hides_value() {
        let key = KeyRedacted::new("lin_api_secret_value_here");
        let dbg = format!("{:?}", key);
        assert!(!dbg.contains("lin_api_secret_value_here"));
        assert!(dbg.contains("[REDACTED]"));
    }

    #[test]
    fn key_redacted_display_hides_value() {
        let key = KeyRedacted::new("lin_api_secret_value_here");
        let disp = format!("{}", key);
        assert!(!disp.contains("lin_api_secret_value_here"));
    }

    #[test]
    fn key_redacted_expose_returns_value() {
        let key = KeyRedacted::new("real_value");
        assert_eq!(key.expose(), "real_value");
    }

    #[test]
    fn init_config_writes_current_key_and_profiles_round_trip() {
        let _lock = HOME_LOCK.lock();
        let dir = tempfile::tempdir().unwrap();
        let _home = FakeHome::new(dir.path());

        // init with a profile sets it as default and loads via defaultProfile.
        init_config(Some("work"), "lin_api_work").expect("init work profile");
        let profiles = list_profiles().expect("list profiles");
        assert!(profiles.iter().any(|(n, _)| n == "work"));
        let key = load_key().expect("load work key via default profile");
        assert_eq!(key.expose(), "lin_api_work");

        // init without a profile writes currentKey, which takes priority.
        init_config(None, "lin_api_default").expect("init default");
        let key = load_key().expect("load default key");
        assert_eq!(key.expose(), "lin_api_default");

        // switching default profile clears currentKey so the profile is used.
        set_default_profile("work").expect("set default");
        let key = load_key().expect("load work key via default profile");
        assert_eq!(key.expose(), "lin_api_work");

        let removed = delete_profile("work").expect("delete work");
        assert!(removed);
        let profiles = list_profiles().expect("list profiles after delete");
        assert!(!profiles.iter().any(|(n, _)| n == "work"));
    }
}
