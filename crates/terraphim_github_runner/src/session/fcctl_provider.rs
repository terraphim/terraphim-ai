//! Real fcctl-web VM session provider.
//!
//! Implements the [`VmProvider`] trait against a live fcctl-web HTTP service,
//! replacing the simulated `HostVmProvider` when `RUNNER_VM_MODE=firecracker`.

use std::time::{Duration, Instant};

use async_trait::async_trait;
use log::{info, warn};
use serde::Deserialize;

use crate::error::{GitHubRunnerError, Result};
use crate::session::manager::VmProvider;

/// Default time to wait for a VM to reach `running` status.
const DEFAULT_READY_TIMEOUT: Duration = Duration::from_secs(30);

/// Poll interval when waiting for VM to become running.
const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Per the design doc's TARGET_BOOT_TIME_MS, warn if boot exceeds this.
const WARN_BOOT_THRESHOLD: Duration = Duration::from_secs(2);

/// Real provider backed by the live fcctl-web HTTP service.
pub struct FcctlWebProvider {
    base_url: String,
    auth_token: Option<String>,
    client: reqwest::Client,
    ready_timeout: Duration,
}

#[derive(Debug, Deserialize)]
struct VmResponse {
    id: String,
    status: Option<String>,
}

impl FcctlWebProvider {
    /// Create a new provider.
    ///
    /// `auth_token` is sent as `Authorization: Bearer <token>`. If `None`,
    /// requests are unauthenticated (fcctl-web in dev mode).
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        Self {
            base_url: base_url.into(),
            auth_token,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            ready_timeout: DEFAULT_READY_TIMEOUT,
        }
    }

    fn vms_url(&self) -> String {
        format!("{}/api/vms", self.base_url)
    }

    fn vm_url(&self, vm_id: &str) -> String {
        format!("{}/api/vms/{}", self.base_url, vm_id)
    }

    fn add_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(ref token) = self.auth_token {
            req.bearer_auth(token)
        } else {
            req
        }
    }
}

#[async_trait]
impl VmProvider for FcctlWebProvider {
    async fn allocate(&self, vm_type: &str) -> Result<(String, Duration)> {
        let start = Instant::now();

        let payload = serde_json::json!({ "vm_type": vm_type });
        let req = self
            .client
            .post(self.vms_url())
            .header("Content-Type", "application/json")
            .json(&payload);
        let req = self.add_auth(req);

        let resp = req
            .send()
            .await
            .map_err(|e| GitHubRunnerError::VmAllocation(format!("POST /api/vms failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(GitHubRunnerError::VmAllocation(format!(
                "POST /api/vms returned {status}: {body}"
            )));
        }

        let vm: VmResponse = resp
            .json()
            .await
            .map_err(|e| GitHubRunnerError::VmAllocation(format!("parse VM response: {e}")))?;

        let vm_id = vm.id;
        info!("fcctl-web: created VM {vm_id} (type {vm_type}), waiting for running");

        let deadline = start + self.ready_timeout;
        loop {
            if Instant::now() > deadline {
                return Err(GitHubRunnerError::VmAllocation(format!(
                    "VM {vm_id} did not reach running within {:?}",
                    self.ready_timeout
                )));
            }

            let req = self.client.get(self.vm_url(&vm_id));
            let req = self.add_auth(req);
            let resp = req.send().await.map_err(|e| {
                GitHubRunnerError::VmAllocation(format!("GET /api/vms/{vm_id} failed: {e}"))
            })?;

            if resp.status().is_success() {
                let vm: VmResponse = resp.json().await.map_err(|e| {
                    GitHubRunnerError::VmAllocation(format!("parse poll response: {e}"))
                })?;

                if vm.status.as_deref() == Some("running") {
                    let elapsed = start.elapsed();
                    if elapsed > WARN_BOOT_THRESHOLD {
                        warn!(
                            "fcctl-web: VM {vm_id} boot took {:?} (>{:?})",
                            elapsed, WARN_BOOT_THRESHOLD
                        );
                    } else {
                        info!("fcctl-web: VM {vm_id} running in {:?}", elapsed);
                    }
                    return Ok((vm_id, elapsed));
                }
            }

            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    async fn release(&self, vm_id: &str) -> Result<()> {
        let req = self.client.delete(self.vm_url(vm_id));
        let req = self.add_auth(req);

        let resp = req.send().await.map_err(|e| {
            GitHubRunnerError::VmAllocation(format!("DELETE /api/vms/{vm_id} failed: {e}"))
        })?;

        let status = resp.status();
        if status.is_success() || status.as_u16() == 404 {
            info!("fcctl-web: released VM {vm_id} (status {status})");
            Ok(())
        } else {
            let body = resp.text().await.unwrap_or_default();
            warn!("fcctl-web: DELETE /api/vms/{vm_id} returned {status}: {body}");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_urls() {
        let p = FcctlWebProvider::new("http://127.0.0.1:8080", None);
        assert_eq!(p.vms_url(), "http://127.0.0.1:8080/api/vms");
        assert_eq!(p.vm_url("vm-123"), "http://127.0.0.1:8080/api/vms/vm-123");
    }

    #[test]
    fn handles_auth_token() {
        let p = FcctlWebProvider::new("http://localhost:8080", Some("secret".into()));
        assert_eq!(p.auth_token.as_deref(), Some("secret"));
    }

    #[test]
    fn handles_no_auth_token() {
        let p = FcctlWebProvider::new("http://localhost:8080", None);
        assert!(p.auth_token.is_none());
    }
}
