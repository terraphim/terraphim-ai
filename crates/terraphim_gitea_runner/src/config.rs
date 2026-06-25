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
    /// API token for native commit-status posts when the per-job `github.token`
    /// lacks `statuses` scope (common on private repos). Set via
    /// `RUNNER_STATUS_TOKEN` or `GITEA_TOKEN`. `None` falls back to job token only.
    pub status_token: Option<String>,
    /// Timeout applied to each HTTP request to the Gitea RunnerService.
    /// A hung `FetchTask` call is aborted after this duration rather than
    /// blocking the poll loop indefinitely.
    pub http_request_timeout: Duration,
    /// Belt-and-suspenders timeout wrapping each `poll_once` call in
    /// `run_forever`. Should exceed `http_request_timeout` so reqwest's own
    /// timeout fires first; defaults to `2 x http_request_timeout`.
    pub poll_timeout: Duration,
    /// Directory containing `command_policy.md` for the taxonomy-driven
    /// command allowlist. If `None`, the embedded default policy is used.
    pub taxonomy_dir: Option<PathBuf>,
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
            status_token: None,
            http_request_timeout: Duration::from_secs(30),
            poll_timeout: Duration::from_secs(60),
            taxonomy_dir: None,
        }
    }
}

impl RunnerConfig {
    /// Whether this runner should execute work for `repo` (coexistence guard).
    pub fn accepts_repo(&self, repo: &str) -> bool {
        self.active_repos.is_empty() || self.active_repos.iter().any(|r| r == repo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_accepts_all_repos() {
        let cfg = RunnerConfig::default();
        assert!(cfg.accepts_repo("terraphim/terraphim-ai"));
        assert!(cfg.accepts_repo("any/repo"));
        assert!(cfg.active_repos.is_empty());
    }

    #[test]
    fn active_repos_allowlist_accepts_matching_repo() {
        let cfg = RunnerConfig {
            active_repos: vec!["terraphim/terraphim-ai".to_string()],
            ..RunnerConfig::default()
        };
        assert!(cfg.accepts_repo("terraphim/terraphim-ai"));
    }

    #[test]
    fn active_repos_allowlist_rejects_other_repo() {
        let cfg = RunnerConfig {
            active_repos: vec!["terraphim/terraphim-ai".to_string()],
            ..RunnerConfig::default()
        };
        assert!(!cfg.accepts_repo("other/repo"));
        assert!(!cfg.accepts_repo("terraphim/other-repo"));
    }

    #[test]
    fn multiple_active_repos_accept_each_listed_repo() {
        let cfg = RunnerConfig {
            active_repos: vec![
                "terraphim/terraphim-ai".to_string(),
                "terraphim/terraphim-agents".to_string(),
            ],
            ..RunnerConfig::default()
        };
        assert!(cfg.accepts_repo("terraphim/terraphim-ai"));
        assert!(cfg.accepts_repo("terraphim/terraphim-agents"));
        assert!(!cfg.accepts_repo("terraphim/other"));
    }

    #[test]
    fn default_config_has_expected_instance_url() {
        let cfg = RunnerConfig::default();
        assert_eq!(cfg.instance_url, "https://git.terraphim.cloud");
    }

    #[test]
    fn default_config_has_no_registration_token() {
        let cfg = RunnerConfig::default();
        assert!(cfg.registration_token.is_none());
    }

    #[test]
    fn default_config_has_no_status_token() {
        let cfg = RunnerConfig::default();
        assert!(cfg.status_token.is_none());
    }

    #[test]
    fn default_config_has_no_legacy_mirror() {
        let cfg = RunnerConfig::default();
        assert!(cfg.legacy_status_mirror.is_none());
    }

    #[test]
    fn default_config_poll_timeout_exceeds_http_timeout() {
        let cfg = RunnerConfig::default();
        assert!(
            cfg.poll_timeout > cfg.http_request_timeout,
            "poll_timeout must exceed http_request_timeout so reqwest fires first"
        );
    }

    #[test]
    fn default_config_has_terraphim_native_label() {
        let cfg = RunnerConfig::default();
        assert!(cfg.labels.contains(&"terraphim-native".to_string()));
    }
}
