//! Runner configuration.

use std::path::PathBuf;
use std::time::Duration;

/// VM execution mode for build steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VmMode {
    /// Run commands directly on the host (today's behaviour; fail-open default).
    #[default]
    Host,
    /// Run commands inside ephemeral Firecracker microVMs via fcctl-web.
    Firecracker,
}

impl VmMode {
    /// Parse from an environment variable string (case-insensitive).
    pub fn from_env_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "firecracker" | "fc" | "vm" => VmMode::Firecracker,
            _ => VmMode::Host,
        }
    }
}

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
    /// Belt-and-suspenders timeout wrapping only the pre-claim `FetchTask`
    /// request. It must never cancel an already-claimed task's worker lifecycle.
    /// Should exceed `http_request_timeout` so reqwest's own timeout fires first;
    /// defaults to `2 x http_request_timeout`.
    pub poll_timeout: Duration,
    /// Directory containing `command_policy.md` for the taxonomy-driven
    /// command allowlist. If `None`, the embedded default policy is used.
    pub taxonomy_dir: Option<PathBuf>,
    /// VM execution mode: `Host` (default, fail-open) or `Firecracker`.
    pub vm_mode: VmMode,
    /// fcctl-web base URL when `vm_mode == Firecracker`.
    pub fcctl_url: String,
    /// VM type to allocate from fcctl-web (must exist in images.yaml).
    pub fcctl_vm_type: String,
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
            vm_mode: VmMode::Host,
            fcctl_url: "http://127.0.0.1:8080".to_string(),
            fcctl_vm_type: "rust-ci".to_string(),
        }
    }
}

impl RunnerConfig {
    /// Whether this runner should execute work for `repo` (coexistence guard).
    pub fn accepts_repo(&self, repo: &str) -> bool {
        self.active_repos.is_empty() || self.active_repos.iter().any(|r| r == repo)
    }
}
