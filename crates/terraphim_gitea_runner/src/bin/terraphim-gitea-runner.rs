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
//! - `RUNNER_ACTIVE_REPOS`  comma-separated allowlist (empty = all offered)
//! - `RUNNER_CHECKOUT_DIR`  default `.`

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use terraphim_gitea_runner::client::{GiteaRunnerClient, ReqwestRunnerClient};
use terraphim_gitea_runner::config::RunnerConfig;
use terraphim_gitea_runner::policy::DeterministicPlanner;
use terraphim_gitea_runner::poller::Poller;
use terraphim_gitea_runner::state::RunnerState;
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

    let config = RunnerConfig {
        instance_url: env_or("GITEA_URL", "https://git.terraphim.cloud"),
        org: env_or("GITEA_ORG", "terraphim"),
        registration_token: std::env::var("RUNNER_TOKEN").ok(),
        state_file: PathBuf::from(env_or("RUNNER_STATE_FILE", ".runner")),
        labels: csv("RUNNER_LABELS", &["terraphim-native"]),
        poll_interval: Duration::from_secs(3),
        active_repos: csv("RUNNER_ACTIVE_REPOS", &[]),
    };
    let checkout_dir = env_or("RUNNER_CHECKOUT_DIR", ".");
    let version = env!("CARGO_PKG_VERSION").to_string();

    let client = Arc::new(ReqwestRunnerClient::new(config.instance_url.clone()));

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

    let poller = Poller::new(client, Arc::new(DeterministicPlanner), config, checkout_dir);
    poller.run_forever(&state).await?;
    Ok(())
}
