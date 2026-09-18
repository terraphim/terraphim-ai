//! Native Gitea runner daemon.
//!
//! Registers (once) against the configured org, declares labels, then polls for
//! tasks. Configuration via environment:
//!
//! - `GITEA_URL`            (default `https://git.terraphim.cloud`)
//! - `GITEA_ORG`            (default `terraphim`)
//! - `RUNNER_TOKEN`         registration token (from `op`; first run only)
//! - `RUNNER_STATE_FILE`    default `.runner`
//! - `RUNNER_LABELS`        comma-separated, default `terraphim-native`
//! - `RUNNER_ACTIVE_REPOS`  comma-separated repo allowlist (required unless
//!   `RUNNER_ACCEPT_ALL=1`, which opts into all org jobs)
//! - `RUNNER_ACCEPT_ALL`    set `1` to accept every terraphim-native job (no allowlist)
//! - `RUNNER_LEGACY_TOKEN`  enable the legacy commit-status mirror with this API token
//! - `RUNNER_LEGACY_CONTEXT` legacy mirror context, default `adf/build`
//! - `RUNNER_STATUS_TOKEN`  API token for native commit-status posts (preferred over
//!   per-job `github.token`, which often returns HTTP 401 on private repos)
//! - `GITEA_TOKEN`          fallback for `RUNNER_STATUS_TOKEN` when unset
//! - `RUNNER_CHECKOUT_DIR`  checkout root; per-repo trees at `<root>/<owner>/<repo>` (default `.`)
//! - `RUNNER_HTTP_TIMEOUT`  per-request HTTP timeout in seconds (default 30; must be > 0)
//! - `RUNNER_GIT_TIMEOUT`   per-Git-operation checkout timeout in seconds (default 300)
//! - `RUNNER_HEARTBEAT_INTERVAL` interval in seconds between nonterminal
//!   `UpdateTask` heartbeats for a claimed workflow (default 15; must be > 0)
//! - `RUNNER_HEARTBEAT_FAILURE_ATTEMPTS` consecutive heartbeat failures before
//!   fail-closed terminalization (default 10; must be > 0; resets on success)
//!   The configured lease envelope `(interval + HTTP timeout) x failure attempts`
//!   must stay below Gitea's tested 600-second stale-task cutoff.
//! - `RUNNER_POLL_TIMEOUT`  belt-and-suspenders timeout wrapping only the
//!   pre-claim `FetchTask` request (default `2 x RUNNER_HTTP_TIMEOUT`). It never
//!   bounds an already-claimed workflow; TaskWorker owns that lifecycle. Must be > 0.
//! - `RUNNER_TAXONOMY_DIR`  directory containing `command_policy.md` for the
//!   command allowlist; if unset, the embedded default policy is used
//!
//! ## Service shutdown contract
//! Host steps run in dedicated process groups. A systemd unit must use
//! `KillMode=control-group` (not `process`) so a forced stop reaches every
//! descendant. SIGTERM/SIGINT initiates graceful shutdown: the daemon stops
//! after any already-claimed task has terminalized. `TimeoutStopSec` therefore
//! needs to cover the maximum accepted workflow duration; after that grace
//! period systemd's control-group kill is the final containment boundary.
//! `WatchdogSec` must be unset or exceed the maximum claimed-task duration
//! unless watchdog notifications move into the owned-task heartbeat; the
//! current poller cannot notify systemd while it awaits a claimed task.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use terraphim_gitea_runner::client::{GiteaRunnerClient, ReqwestRunnerClient};
use terraphim_gitea_runner::config::{
    DEFAULT_GIT_OPERATION_TIMEOUT, DEFAULT_HEARTBEAT_FAILURE_ATTEMPTS, DEFAULT_HEARTBEAT_INTERVAL,
    DEFAULT_HTTP_REQUEST_TIMEOUT, GITEA_ZOMBIE_TASK_TIMEOUT, LegacyStatusMirrorConfig,
    RunnerConfig, VmMode,
};
use terraphim_gitea_runner::poller::Poller;
use terraphim_gitea_runner::state::RunnerState;
use terraphim_gitea_runner::taxonomy_policy::TaxonomyPlanner;
use terraphim_gitea_runner::types::{DeclareRequest, RegisterRequest};

#[cfg(test)]
#[path = "../../build.rs"]
mod build_script;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn csv(key: &str, default: &[&str]) -> Vec<String> {
    match std::env::var(key) {
        Ok(v) if !v.trim().is_empty() => v.split(',').map(|s| s.trim().to_string()).collect(),
        _ => default.iter().map(|s| s.to_string()).collect(),
    }
}

fn parse_positive_u64(key: &str, value: Option<&str>, default: u64) -> anyhow::Result<u64> {
    let Some(value) = value else {
        return Ok(default);
    };
    let parsed = value
        .parse::<u64>()
        .map_err(|error| anyhow::anyhow!("{key} must be a positive integer: {error}"))?;
    if parsed == 0 {
        anyhow::bail!("{key} must be greater than zero");
    }
    Ok(parsed)
}

fn parse_positive_u32(key: &str, value: Option<&str>, default: u32) -> anyhow::Result<u32> {
    let parsed = parse_positive_u64(key, value, u64::from(default))?;
    u32::try_from(parsed).map_err(|_| anyhow::anyhow!("{key} must fit in a 32-bit integer"))
}

#[derive(Debug, PartialEq, Eq)]
struct RunnerTiming {
    http_request_timeout: Duration,
    heartbeat_interval: Duration,
    heartbeat_failure_attempts: u32,
    git_operation_timeout: Duration,
    poll_timeout: Duration,
}

fn resolve_runner_timing(
    http_value: Option<&str>,
    heartbeat_value: Option<&str>,
    heartbeat_attempts_value: Option<&str>,
    git_value: Option<&str>,
    poll_value: Option<&str>,
) -> anyhow::Result<RunnerTiming> {
    let http_timeout_secs = parse_positive_u64(
        "RUNNER_HTTP_TIMEOUT",
        http_value,
        DEFAULT_HTTP_REQUEST_TIMEOUT.as_secs(),
    )?;
    let heartbeat_interval_secs = parse_positive_u64(
        "RUNNER_HEARTBEAT_INTERVAL",
        heartbeat_value,
        DEFAULT_HEARTBEAT_INTERVAL.as_secs(),
    )?;
    let heartbeat_failure_attempts = parse_positive_u32(
        "RUNNER_HEARTBEAT_FAILURE_ATTEMPTS",
        heartbeat_attempts_value,
        DEFAULT_HEARTBEAT_FAILURE_ATTEMPTS,
    )?;
    let git_timeout_secs = parse_positive_u64(
        "RUNNER_GIT_TIMEOUT",
        git_value,
        DEFAULT_GIT_OPERATION_TIMEOUT.as_secs(),
    )?;
    let http_request_timeout = Duration::from_secs(http_timeout_secs);
    let heartbeat_interval = Duration::from_secs(heartbeat_interval_secs);
    let per_attempt = heartbeat_interval
        .checked_add(http_request_timeout)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "heartbeat failure envelope overflows while adding \
                 RUNNER_HEARTBEAT_INTERVAL and RUNNER_HTTP_TIMEOUT"
            )
        })?;
    let envelope = per_attempt
        .checked_mul(heartbeat_failure_attempts)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "heartbeat failure envelope overflows while multiplying by \
                 RUNNER_HEARTBEAT_FAILURE_ATTEMPTS"
            )
        })?;
    if envelope >= GITEA_ZOMBIE_TASK_TIMEOUT {
        anyhow::bail!(
            "heartbeat failure envelope ({heartbeat_interval_secs}s + {http_timeout_secs}s) x \
             {heartbeat_failure_attempts} = {}s must be below the Gitea stale-task cutoff of {}s; \
             lower RUNNER_HEARTBEAT_INTERVAL, RUNNER_HTTP_TIMEOUT, or \
             RUNNER_HEARTBEAT_FAILURE_ATTEMPTS",
            envelope.as_secs(),
            GITEA_ZOMBIE_TASK_TIMEOUT.as_secs()
        );
    }

    let default_poll_timeout = http_timeout_secs.checked_mul(2).ok_or_else(|| {
        anyhow::anyhow!(
            "default RUNNER_POLL_TIMEOUT overflows at twice RUNNER_HTTP_TIMEOUT; \
             configure a smaller positive timeout"
        )
    })?;
    let poll_timeout_secs =
        parse_positive_u64("RUNNER_POLL_TIMEOUT", poll_value, default_poll_timeout)?;

    Ok(RunnerTiming {
        http_request_timeout,
        heartbeat_interval,
        heartbeat_failure_attempts,
        git_operation_timeout: Duration::from_secs(git_timeout_secs),
        poll_timeout: Duration::from_secs(poll_timeout_secs),
    })
}

fn build_provenance() -> &'static str {
    concat!(
        "version=",
        env!("CARGO_PKG_VERSION"),
        " commit=",
        env!("TERRAPHIM_RUNNER_BUILD_COMMIT")
    )
}

async fn shutdown_signal() -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {
            signal = tokio::signal::ctrl_c() => signal?,
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    log::info!("runner build provenance: {}", build_provenance());

    let active_repos = csv("RUNNER_ACTIVE_REPOS", &[]);
    // Coexistence guard: an org-scoped runner with an empty allowlist would claim
    // ANY terraphim-native job in the org. Require explicit opt-in to accept-all.
    if active_repos.is_empty() && env_or("RUNNER_ACCEPT_ALL", "0") != "1" {
        anyhow::bail!(
            "RUNNER_ACTIVE_REPOS is empty. Set it to the repos this runner should serve \
             (comma-separated), or set RUNNER_ACCEPT_ALL=1 to deliberately accept every \
             terraphim-native job in the org."
        );
    }

    // Optional legacy adf/build mirror during migration.
    let legacy_status_mirror =
        std::env::var("RUNNER_LEGACY_TOKEN")
            .ok()
            .map(|token| LegacyStatusMirrorConfig {
                token,
                context: env_or("RUNNER_LEGACY_CONTEXT", "adf/build"),
            });

    let http_timeout_value = std::env::var("RUNNER_HTTP_TIMEOUT").ok();
    let heartbeat_interval_value = std::env::var("RUNNER_HEARTBEAT_INTERVAL").ok();
    let heartbeat_attempts_value = std::env::var("RUNNER_HEARTBEAT_FAILURE_ATTEMPTS").ok();
    let git_timeout_value = std::env::var("RUNNER_GIT_TIMEOUT").ok();
    let poll_timeout_value = std::env::var("RUNNER_POLL_TIMEOUT").ok();
    let timing = resolve_runner_timing(
        http_timeout_value.as_deref(),
        heartbeat_interval_value.as_deref(),
        heartbeat_attempts_value.as_deref(),
        git_timeout_value.as_deref(),
        poll_timeout_value.as_deref(),
    )?;

    // #2185 / #3222: this belt-and-suspenders timeout wraps only the pre-claim
    // FetchTask request. Claimed workflow execution is never cancelled by the
    // poll timeout; TaskWorker owns it through terminal publication. Keep this
    // longer than reqwest's request timeout so the client timeout normally fires
    // first. Operators can tune it via env.
    let status_token = std::env::var("RUNNER_STATUS_TOKEN")
        .ok()
        .or_else(|| std::env::var("GITEA_TOKEN").ok());

    let taxonomy_dir = std::env::var("RUNNER_TAXONOMY_DIR").ok().map(PathBuf::from);

    let vm_mode = std::env::var("RUNNER_VM_MODE")
        .map(|s| VmMode::from_env_str(&s))
        .unwrap_or_default();
    let fcctl_url = env_or("FCCTL_URL", "http://127.0.0.1:8080");
    let fcctl_vm_type = env_or("FCCTL_VM_TYPE", "rust-ci");

    let config = RunnerConfig {
        instance_url: env_or("GITEA_URL", "https://git.terraphim.cloud"),
        org: env_or("GITEA_ORG", "terraphim"),
        registration_token: std::env::var("RUNNER_TOKEN").ok(),
        state_file: PathBuf::from(env_or("RUNNER_STATE_FILE", ".runner")),
        labels: csv("RUNNER_LABELS", &["terraphim-native"]),
        poll_interval: Duration::from_secs(3),
        active_repos,
        legacy_status_mirror,
        status_token,
        http_request_timeout: timing.http_request_timeout,
        git_operation_timeout: timing.git_operation_timeout,
        heartbeat_interval: timing.heartbeat_interval,
        heartbeat_failure_attempts: timing.heartbeat_failure_attempts,
        poll_timeout: timing.poll_timeout,
        taxonomy_dir,
        vm_mode,
        fcctl_url,
        fcctl_vm_type,
    };
    let checkout_dir = env_or("RUNNER_CHECKOUT_DIR", ".");
    let version = env!("CARGO_PKG_VERSION").to_string();

    let client = Arc::new(ReqwestRunnerClient::new_with_timeout(
        config.instance_url.clone(),
        config.http_request_timeout,
    ));

    // Register if we have no persisted state.
    let state = match RunnerState::load(&config.state_file)? {
        Some(s) => {
            log::info!("loaded existing runner state: {s:?}");
            s
        }
        None => {
            let token = config.registration_token.clone().ok_or_else(|| {
                anyhow::anyhow!("no runner state and RUNNER_TOKEN unset; cannot register")
            })?;
            let info = client
                .register(RegisterRequest {
                    token,
                    name: format!("terraphim-native-{}", uuid::Uuid::new_v4()),
                    version: version.clone(),
                    labels: config.labels.clone(),
                })
                .await?;
            let s = RunnerState {
                uuid: info.uuid,
                token: info.token,
                name: info.name,
                version: version.clone(),
                labels: if info.labels.is_empty() {
                    config.labels.clone()
                } else {
                    info.labels
                },
                ephemeral: info.ephemeral,
            };
            s.save(&config.state_file)?;
            log::info!("registered new runner: {s:?}");
            s
        }
    };

    // Declare on startup.
    client
        .declare(
            &state,
            DeclareRequest {
                version,
                labels: state.labels.clone(),
            },
        )
        .await?;
    log::info!("declared; polling for tasks (labels={:?})", state.labels);

    // Construct the taxonomy-driven planner. Loads command_policy.md from
    // RUNNER_TAXONOMY_DIR if set, otherwise uses the embedded default.
    let poller = Poller::new(
        client,
        Arc::new(TaxonomyPlanner::new(&config)),
        config,
        checkout_dir,
    );
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        match shutdown_signal().await {
            Ok(()) => {
                log::info!("shutdown requested; waiting for any claimed task to terminalize");
                let _ = shutdown_tx.send(true);
            }
            Err(error) => log::error!("failed to install shutdown signal handler: {error}"),
        }
    });
    poller.run_until_shutdown(&state, shutdown_rx).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn operator_docs_bind_lease_defaults_and_watchdog_contract() {
        let runner_docs = include_str!("terraphim-gitea-runner.rs");
        let attempts_doc = runner_docs
            .lines()
            .find(|line| line.starts_with("//!   fail-closed terminalization (default"))
            .expect("heartbeat attempts must be documented");
        assert!(
            attempts_doc.contains("default 10;"),
            "operator docs must match the tested heartbeat failure default"
        );
        let service_docs: Vec<_> = runner_docs
            .lines()
            .filter(|line| line.starts_with("//!"))
            .collect();
        assert!(
            service_docs
                .iter()
                .any(|line| line.contains("`WatchdogSec` must be unset or exceed")),
            "service contract must prevent watchdog cancellation of claimed tasks"
        );
        let operator_docs = service_docs.join("\n");
        assert!(
            operator_docs.contains(
                "configured lease envelope `(interval + HTTP timeout) x failure attempts`"
            ) && operator_docs.contains("600-second stale-task cutoff"),
            "operator docs must bind configurable timing to the tested lease envelope"
        );

        let poller_docs = include_str!("../poller.rs");
        let poller_docs: Vec<_> = poller_docs
            .lines()
            .filter(|line| line.trim_start().starts_with("///"))
            .collect();
        assert!(
            poller_docs.iter().any(|line| {
                line.contains("must be unset or exceed the maximum claimed-task duration")
            }),
            "poller guidance must describe the once-per-poll watchdog gap"
        );
    }

    #[test]
    fn build_script_tracked_inputs_all_exist() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let tracked = super::build_script::tracked_inputs(manifest_dir)
            .expect("the test checkout must expose tracked inputs");
        assert!(!tracked.is_empty(), "tracked input list must not be empty");
        let missing: Vec<_> = tracked.iter().filter(|path| !path.exists()).collect();
        assert!(
            missing.is_empty(),
            "tracked build inputs do not exist: {missing:?}"
        );
    }

    #[test]
    fn heartbeat_interval_rejects_invalid_or_zero_values() {
        for invalid in ["0", "30s", "abc", "-5"] {
            let error = super::parse_positive_u64("RUNNER_HEARTBEAT_INTERVAL", Some(invalid), 15)
                .expect_err("invalid heartbeat interval must not silently use the default");
            assert!(
                error.to_string().contains("RUNNER_HEARTBEAT_INTERVAL"),
                "{error}"
            );
        }
        assert_eq!(
            super::parse_positive_u64("RUNNER_HEARTBEAT_INTERVAL", None, 15).unwrap(),
            15
        );
        assert_eq!(
            super::parse_positive_u64("RUNNER_HEARTBEAT_INTERVAL", Some("30"), 15).unwrap(),
            30
        );
        assert!(
            super::parse_positive_u32("RUNNER_HEARTBEAT_FAILURE_ATTEMPTS", Some("0"), 10).is_err()
        );
        assert_eq!(
            super::parse_positive_u32("RUNNER_HEARTBEAT_FAILURE_ATTEMPTS", Some("12"), 10).unwrap(),
            12
        );
    }

    #[test]
    fn http_and_poll_timeouts_reject_invalid_or_zero_values() {
        for invalid in ["0", "30s", "abc", "-5"] {
            let http_error = super::resolve_runner_timing(Some(invalid), None, None, None, None)
                .expect_err("invalid HTTP timeout must not silently use the default");
            assert!(http_error.to_string().contains("RUNNER_HTTP_TIMEOUT"));

            let poll_error = super::resolve_runner_timing(None, None, None, None, Some(invalid))
                .expect_err("invalid poll timeout must not silently use the default");
            assert!(poll_error.to_string().contains("RUNNER_POLL_TIMEOUT"));
        }
    }

    #[test]
    fn configured_lease_envelope_must_beat_gitea_cutoff() {
        let safe = super::resolve_runner_timing(Some("30"), Some("29"), Some("10"), None, None)
            .expect("590-second lease envelope must remain below the cutoff");
        assert_eq!(safe.heartbeat_failure_attempts, 10);

        let equal = super::resolve_runner_timing(Some("30"), Some("30"), Some("10"), None, None)
            .expect_err("an envelope equal to the cutoff must be rejected");
        let message = equal.to_string();
        assert!(message.contains("heartbeat failure envelope"), "{message}");
        assert!(message.contains("Gitea stale-task cutoff"), "{message}");

        let overflow = super::resolve_runner_timing(
            Some("18446744073709551615"),
            Some("18446744073709551615"),
            Some("4294967295"),
            None,
            None,
        )
        .expect_err("overflowing lease arithmetic must fail closed");
        assert!(overflow.to_string().contains("overflows"), "{overflow}");
    }

    #[test]
    fn build_provenance_identifies_the_compiled_source() {
        let provenance = super::build_provenance();
        assert!(
            provenance.starts_with(&format!("version={} commit=", env!("CARGO_PKG_VERSION"))),
            "provenance must include both version and a labelled commit identity: {provenance}"
        );

        let commit = provenance
            .split_once(" commit=")
            .expect("provenance must label its commit identity")
            .1;
        if let Some(git_identity) = commit.strip_prefix("git:") {
            let (git_commit, stamped_dirty) = git_identity
                .strip_suffix("-dirty")
                .map_or((git_identity, false), |sha| (sha, true));
            assert_eq!(git_commit.len(), 40, "Git commit must be a full SHA-1");
            assert!(
                git_commit.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "Git commit must contain only hexadecimal digits: {git_commit}"
            );
            let expected = std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .output()
                .expect("git must be available for a Git-stamped test build");
            assert!(expected.status.success());
            assert_eq!(git_commit, String::from_utf8_lossy(&expected.stdout).trim());
            let status = std::process::Command::new("git")
                .args(["status", "--porcelain", "--untracked-files=no"])
                .output()
                .expect("git status must be available for a Git-stamped test build");
            assert!(status.status.success());
            assert_eq!(
                stamped_dirty,
                !status.stdout.is_empty(),
                "compiled provenance must truthfully distinguish clean and dirty tracked trees"
            );
        } else if let Some(archive_commit) = commit.strip_prefix("source-archive:") {
            assert!(
                archive_commit == "unknown"
                    || (archive_commit.len() == 40
                        && archive_commit.bytes().all(|byte| byte.is_ascii_hexdigit())),
                "source archives need a full commit SHA or an explicit unknown fallback: {commit}"
            );
        } else {
            panic!("unsupported commit provenance: {commit}");
        }
    }

    #[test]
    fn source_identity_distinguishes_clean_and_dirty_real_git_trees() {
        let temp = tempfile::tempdir().unwrap();
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .arg("-C")
                .arg(temp.path())
                .args(args)
                .env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
                .output()
                .unwrap()
        };
        assert!(git(&["init", "-q"]).status.success());
        std::fs::write(temp.path().join("tracked.txt"), "clean\n").unwrap();
        assert!(git(&["add", "tracked.txt"]).status.success());
        assert!(
            git(&["-c", "commit.gpgsign=false", "commit", "-q", "-m", "seed"])
                .status
                .success()
        );

        let clean = super::build_script::source_identity(temp.path(), None);
        assert!(clean.starts_with("git:"), "{clean}");
        assert!(!clean.ends_with("-dirty"), "{clean}");

        std::fs::write(temp.path().join("tracked.txt"), "dirty\n").unwrap();
        let dirty = super::build_script::source_identity(temp.path(), None);
        assert_eq!(dirty, format!("{clean}-dirty"));
    }
}
