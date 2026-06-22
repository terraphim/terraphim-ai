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
//! - `RUNNER_HTTP_TIMEOUT`  per-request HTTP timeout in seconds (default 30)
//! - `RUNNER_TAXONOMY_DIR`  directory containing `command_policy.md` for the
//!   command allowlist; if unset, the embedded default policy is used

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use terraphim_gitea_runner::client::{GiteaRunnerClient, ReqwestRunnerClient};
use terraphim_gitea_runner::config::{LegacyStatusMirrorConfig, RunnerConfig};
use terraphim_gitea_runner::poller::Poller;
use terraphim_gitea_runner::state::RunnerState;
use terraphim_gitea_runner::taxonomy_policy::TaxonomyPlanner;
use terraphim_gitea_runner::types::{DeclareRequest, RegisterRequest};

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn csv(key: &str, default: &[&str]) -> Vec<String> {
    match std::env::var(key) {
        Ok(v) if !v.trim().is_empty() => v.split(',').map(|s| s.trim().to_string()).collect(),
        _ => default.iter().map(|s| s.to_string()).collect(),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

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

    let http_timeout_secs: u64 = std::env::var("RUNNER_HTTP_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    let http_request_timeout = Duration::from_secs(http_timeout_secs);

    let status_token = std::env::var("RUNNER_STATUS_TOKEN")
        .ok()
        .or_else(|| std::env::var("GITEA_TOKEN").ok());

    let taxonomy_dir = std::env::var("RUNNER_TAXONOMY_DIR").ok().map(PathBuf::from);

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
        http_request_timeout,
        poll_timeout: Duration::from_secs(http_timeout_secs * 2),
        taxonomy_dir,
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
    poller.run_forever(&state).await?;
    Ok(())
}
