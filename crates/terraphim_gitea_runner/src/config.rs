//! Runner configuration.

use std::path::PathBuf;
use std::time::Duration;

/// Configuration for the native Gitea runner daemon.
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    /// Gitea instance base URL, e.g. `https://git.terraphim.cloud`.
    pub instance_url: String,
    /// Org the runner is registered against (org-scoped registration).
    pub org: String,
    /// Registration token (from `op`); only needed on first registration.
    pub registration_token: Option<String>,
    /// Path to the persisted `.runner` state file.
    pub state_file: PathBuf,
    /// Labels advertised to Gitea (dedicated, e.g. `["terraphim-native"]`).
    pub labels: Vec<String>,
    /// Poll interval for `FetchTask`.
    pub poll_interval: Duration,
    /// Coexistence allowlist: only these repo names are executed during
    /// migration (empty = accept all the runner is offered). Guards against
    /// double-execution with the interim ADF lane.
    pub active_repos: Vec<String>,
    /// Optional legacy commit-status mirror (e.g. `adf/build`) posted alongside
    /// the native result during migration. `None` disables the mirror.
    pub legacy_status_mirror: Option<LegacyStatusMirrorConfig>,
}

/// Configuration for the optional legacy commit-status mirror.
#[derive(Debug, Clone)]
pub struct LegacyStatusMirrorConfig {
    /// Gitea API token used to POST commit statuses.
    pub token: String,
    /// Status context to write (e.g. `adf/build`).
    pub context: String,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            instance_url: "https://git.terraphim.cloud".to_string(),
            org: "terraphim".to_string(),
            registration_token: None,
            state_file: PathBuf::from(".runner"),
            labels: vec!["terraphim-native".to_string()],
            poll_interval: Duration::from_secs(3),
            active_repos: Vec::new(),
            legacy_status_mirror: None,
        }
    }
}

impl RunnerConfig {
    /// Whether this runner should execute work for `repo` (coexistence guard).
    pub fn accepts_repo(&self, repo: &str) -> bool {
        self.active_repos.is_empty() || self.active_repos.iter().any(|r| r == repo)
    }
}
