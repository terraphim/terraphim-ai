//! Runner configuration.

use std::path::PathBuf;
use std::time::Duration;

/// Default per-request timeout shared by RunnerService and commit-status HTTP.
pub const DEFAULT_HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Default upper bound for each local Git subprocess used during checkout.
pub const DEFAULT_GIT_OPERATION_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// Default interval between nonterminal updates for a claimed task.
pub const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);

/// Gitea release against which the lease-refresh contract is verified.
pub const GITEA_LEASE_CONTRACT_VERSION: &str = "1.26.0";

/// Database timestamp refreshed by nonterminal `UpdateTask` in Gitea 1.26.0.
pub const GITEA_LEASE_TIMESTAMP_FIELD: &str = "action_task.updated";

/// Gitea 1.26.0's default `[actions].ZOMBIE_TASK_TIMEOUT`.
///
/// The deployed version is pinned in `docker-compose-resilient.yml`. In Gitea
/// v1.26.0, `UpdateTaskByState` handles `RESULT_UNSPECIFIED` by explicitly
/// updating `ActionTask.Updated` (the `action_task.updated` / `updated_unix`
/// column) so the zombie-task reaper does not expire the lease. The upstream
/// default cutoff is ten minutes:
/// <https://github.com/go-gitea/gitea/blob/v1.26.0/models/actions/task.go#L330-L375>
/// <https://docs.gitea.com/1.26/administration/config-cheat-sheet/#actions-actions>
pub const GITEA_ZOMBIE_TASK_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// Consecutive heartbeat failures tolerated before failing closed.
///
/// Ten attempts are independent of terminal-delivery retries and tolerate
/// 2.5 minutes of immediate rejections. Even if every request consumes the
/// default 30-second HTTP bound, exhaustion occurs within 7.5 minutes, below
/// Gitea 1.26.0's ten-minute stale-task cutoff.
pub const DEFAULT_HEARTBEAT_FAILURE_ATTEMPTS: u32 = 10;

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
    /// Maximum duration of each `git init`, `remote`, `fetch`, or `checkout`
    /// subprocess after a task has been claimed.
    pub git_operation_timeout: Duration,
    /// Interval between nonterminal `UpdateTask` heartbeats while an already
    /// claimed workflow executes. This must remain comfortably below Gitea's
    /// stale-task cutoff; the production default is 15 seconds.
    pub heartbeat_interval: Duration,
    /// Consecutive heartbeat transport failures tolerated before failing
    /// closed. A successful heartbeat resets the count.
    pub heartbeat_failure_attempts: u32,
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
            http_request_timeout: DEFAULT_HTTP_REQUEST_TIMEOUT,
            git_operation_timeout: DEFAULT_GIT_OPERATION_TIMEOUT,
            heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL,
            heartbeat_failure_attempts: DEFAULT_HEARTBEAT_FAILURE_ATTEMPTS,
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
