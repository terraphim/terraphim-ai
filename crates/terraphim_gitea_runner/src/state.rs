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
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }
}
