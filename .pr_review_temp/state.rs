//! Persisted runner identity (`.runner` file). The session token is sensitive
//! and is redacted from `Debug`/logs.

use crate::{Result, RunnerError};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Local runner state persisted between restarts.
#[derive(Clone, Serialize, Deserialize)]
pub struct RunnerState {
    pub uuid: String,
    /// Session token (salted server-side). Never logged.
    pub token: String,
    pub name: String,
    pub version: String,
    pub labels: Vec<String>,
    pub ephemeral: bool,
}

impl std::fmt::Debug for RunnerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnerState")
            .field("uuid", &self.uuid)
            .field("token", &"<redacted>")
            .field("name", &self.name)
            .field("version", &self.version)
            .field("labels", &self.labels)
            .field("ephemeral", &self.ephemeral)
            .finish()
    }
}

impl RunnerState {
    /// Load state from `path`, or `None` if the file does not exist.
    pub fn load(path: &Path) -> Result<Option<Self>> {
        match std::fs::read_to_string(path) {
            Ok(s) => serde_json::from_str(&s)
                .map(Some)
                .map_err(|e| RunnerError::State(format!("parse {}: {e}", path.display()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(RunnerError::State(format!("read {}: {e}", path.display()))),
        }
    }

    /// Persist state to `path` with owner-only permissions (0600 on unix).
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| RunnerError::State(format!("serialise: {e}")))?;
        std::fs::write(path, json)
            .map_err(|e| RunnerError::State(format!("write {}: {e}", path.display())))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)) {
                // Non-fatal: warn so operators know the file may be world-readable.
                log::warn!("could not set 0600 permissions on {}: {e}", path.display());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> RunnerState {
        RunnerState {
            uuid: "uuid-xyz".into(),
            token: "super-secret-token".into(),
            name: "terraphim-native-0".into(),
            version: "0.1.0".into(),
            labels: vec!["terraphim-native".into()],
            ephemeral: false,
        }
    }

    #[test]
    fn save_then_load_round_trips_identity() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".runner");
        let original = sample();
        original.save(&path).unwrap();

        let loaded = RunnerState::load(&path).unwrap().expect("state present");
        assert_eq!(loaded.uuid, original.uuid);
        assert_eq!(loaded.token, original.token);
        assert_eq!(loaded.name, original.name);
        assert_eq!(loaded.version, original.version);
        assert_eq!(loaded.labels, original.labels);
        assert_eq!(loaded.ephemeral, original.ephemeral);
    }

    #[test]
    fn load_absent_file_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("does-not-exist");
        assert!(RunnerState::load(&path).unwrap().is_none());
    }

    #[test]
    fn debug_redacts_token() {
        let dbg = format!("{:?}", sample());
        assert!(dbg.contains("<redacted>"), "Debug should redact: {dbg}");
        assert!(
            !dbg.contains("super-secret-token"),
            "token must not appear in Debug: {dbg}"
        );
        // Non-sensitive fields remain observable for diagnostics.
        assert!(dbg.contains("uuid-xyz"));
    }

    #[cfg(unix)]
    #[test]
    fn saved_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".runner");
        sample().save(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "expected 0600, got {:o}", mode & 0o777);
    }
}
