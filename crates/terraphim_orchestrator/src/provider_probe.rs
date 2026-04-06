//! Provider availability probing with per-provider circuit breakers.
//!
//! Reuses [`terraphim_spawner::health::CircuitBreaker`] for tracking provider
//! health state (Closed/Open/HalfOpen). The probe executes `action::` templates
//! from KG routing rules via CLI tools to test the full stack.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use terraphim_spawner::health::{CircuitBreaker, CircuitBreakerConfig, CircuitState, HealthStatus};
use tracing::{info, warn};

use crate::kg_router::KgRouter;

/// Result of probing a single provider+model combination.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProbeResult {
    pub provider: String,
    pub model: String,
    pub cli_tool: String,
    pub status: ProbeStatus,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
    pub timestamp: String,
}

/// Status of a probe attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeStatus {
    Success,
    Error,
    Timeout,
}

/// Cached provider availability map with TTL-based refresh.
pub struct ProviderHealthMap {
    /// Per-provider circuit breakers.
    breakers: HashMap<String, CircuitBreaker>,
    /// Latest probe results.
    results: Vec<ProbeResult>,
    /// When the last probe ran.
    probed_at: Option<Instant>,
    /// How long probe results are valid.
    ttl: Duration,
    /// Circuit breaker configuration.
    cb_config: CircuitBreakerConfig,
}

impl std::fmt::Debug for ProviderHealthMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderHealthMap")
            .field("providers", &self.breakers.len())
            .field("results", &self.results.len())
            .field("stale", &self.is_stale())
            .finish()
    }
}

impl ProviderHealthMap {
    /// Create a new health map with the given TTL.
    pub fn new(ttl: Duration) -> Self {
        Self {
            breakers: HashMap::new(),
            results: Vec::new(),
            probed_at: None,
            ttl,
            cb_config: CircuitBreakerConfig {
                failure_threshold: 5,
                cooldown: Duration::from_secs(60),
                success_threshold: 1,
            },
        }
    }

    /// Check if cached probe results have expired.
    pub fn is_stale(&self) -> bool {
        self.probed_at
            .map(|t| t.elapsed() >= self.ttl)
            .unwrap_or(true)
    }

    /// Run probes for all providers found in KG routing rules.
    ///
    /// Extracts unique `(provider, model, action)` triples from the router's
    /// rules, executes each action template with a test prompt via
    /// `tokio::process::Command`, and records results.
    pub async fn probe_all(&mut self, kg_router: &KgRouter) {
        let mut seen = HashMap::new();
        let mut tasks = Vec::new();

        // Collect unique provider+model combos from all KG routing rules
        for rule in kg_router.all_routes() {
            let key = format!("{}:{}", rule.provider, rule.model);
            if seen.contains_key(&key) {
                continue;
            }
            seen.insert(key, true);

            let provider = rule.provider.clone();
            let model = rule.model.clone();
            let action = rule.action.clone();

            tasks.push(tokio::spawn(async move {
                probe_single(&provider, &model, action.as_deref()).await
            }));
        }

        let mut results = Vec::new();
        for task in tasks {
            match task.await {
                Ok(result) => results.push(result),
                Err(e) => warn!(error = %e, "probe task panicked"),
            }
        }

        // Update circuit breakers from probe results
        for result in &results {
            let breaker = self
                .breakers
                .entry(result.provider.clone())
                .or_insert_with(|| CircuitBreaker::new(self.cb_config.clone()));

            match result.status {
                ProbeStatus::Success => breaker.record_success(),
                ProbeStatus::Error | ProbeStatus::Timeout => breaker.record_failure(),
            }
        }

        info!(
            providers_probed = results.len(),
            healthy = results
                .iter()
                .filter(|r| r.status == ProbeStatus::Success)
                .count(),
            "provider probe complete"
        );

        self.results = results;
        self.probed_at = Some(Instant::now());
    }

    /// Get health status for a provider.
    ///
    /// Uses **probe results first**: if the latest probe for this provider
    /// failed or timed out, it's unhealthy regardless of circuit breaker state.
    /// Falls back to circuit breaker for providers not recently probed.
    pub fn provider_health(&self, provider: &str) -> HealthStatus {
        // Check latest probe results (most authoritative)
        if let Some(status) = self.latest_probe_status(provider) {
            return match status {
                ProbeStatus::Success => HealthStatus::Healthy,
                ProbeStatus::Error => HealthStatus::Unhealthy,
                ProbeStatus::Timeout => HealthStatus::Unhealthy,
            };
        }

        // Fall back to circuit breaker for unprobed providers
        match self.breakers.get(provider) {
            Some(breaker) => match breaker.state() {
                CircuitState::Closed => HealthStatus::Healthy,
                CircuitState::HalfOpen => HealthStatus::Degraded,
                CircuitState::Open => HealthStatus::Unhealthy,
            },
            None => HealthStatus::Healthy, // Unknown providers assumed healthy
        }
    }

    /// Check if a provider is healthy enough to dispatch to.
    ///
    /// A provider is healthy if its latest probe succeeded OR it wasn't probed
    /// and the circuit breaker allows requests.
    pub fn is_healthy(&self, provider: &str) -> bool {
        matches!(
            self.provider_health(provider),
            HealthStatus::Healthy | HealthStatus::Degraded
        )
    }

    /// List all unhealthy provider names (from probe results + circuit breakers).
    pub fn unhealthy_providers(&self) -> Vec<String> {
        let mut unhealthy: Vec<String> = Vec::new();

        // From probe results: any provider with failed/timeout probe
        for result in &self.results {
            if result.status != ProbeStatus::Success && !unhealthy.contains(&result.provider) {
                unhealthy.push(result.provider.clone());
            }
        }

        // From circuit breakers: any open circuit not already in list
        for (name, breaker) in &self.breakers {
            if !breaker.should_allow() && !unhealthy.contains(name) {
                unhealthy.push(name.clone());
            }
        }

        unhealthy
    }

    /// Get the latest probe status for a provider (best result across all models).
    fn latest_probe_status(&self, provider: &str) -> Option<ProbeStatus> {
        let provider_results: Vec<_> = self
            .results
            .iter()
            .filter(|r| r.provider == provider)
            .collect();

        if provider_results.is_empty() {
            return None;
        }

        // If ANY model for this provider succeeded, provider is healthy
        if provider_results
            .iter()
            .any(|r| r.status == ProbeStatus::Success)
        {
            Some(ProbeStatus::Success)
        } else {
            // All models failed -- use the "least bad" status
            Some(provider_results[0].status)
        }
    }

    /// Record a success for a provider (e.g., from ExitClassifier).
    pub fn record_success(&mut self, provider: &str) {
        if let Some(breaker) = self.breakers.get_mut(provider) {
            breaker.record_success();
        }
    }

    /// Record a failure for a provider (e.g., from ExitClassifier ModelError).
    pub fn record_failure(&mut self, provider: &str) {
        let breaker = self
            .breakers
            .entry(provider.to_string())
            .or_insert_with(|| CircuitBreaker::new(self.cb_config.clone()));
        breaker.record_failure();
        warn!(
            provider = provider,
            state = %breaker.state(),
            "provider failure recorded"
        );
    }

    /// Get the latest probe results.
    pub fn results(&self) -> &[ProbeResult] {
        &self.results
    }

    /// Load probe results from a JSON file (pi-benchmark compatible format).
    ///
    /// Used by the meta-coordinator to bootstrap health state from a previous
    /// benchmark run stored at `dir/latest.json`. Circuit breakers are updated
    /// from the loaded results so routing decisions are meaningful before the
    /// first live probe completes.
    pub fn load_from_file(&mut self, dir: &std::path::Path) {
        let path = dir.join("latest.json");
        match std::fs::read_to_string(&path) {
            Ok(json) => match serde_json::from_str::<Vec<ProbeResult>>(&json) {
                Ok(results) => {
                    info!(
                        path = %path.display(),
                        results = results.len(),
                        "meta-coordinator: loaded prior benchmark results"
                    );
                    // Update circuit breakers from loaded results
                    for result in &results {
                        let breaker = self
                            .breakers
                            .entry(result.provider.clone())
                            .or_insert_with(|| CircuitBreaker::new(self.cb_config.clone()));
                        match result.status {
                            ProbeStatus::Success => breaker.record_success(),
                            ProbeStatus::Error | ProbeStatus::Timeout => breaker.record_failure(),
                        }
                    }
                    self.results = results;
                    // Mark as loaded (not stale) so we don't immediately re-probe
                    self.probed_at = Some(Instant::now());
                }
                Err(e) => {
                    warn!(path = %path.display(), error = %e, "meta-coordinator: failed to parse benchmark JSON");
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                info!(path = %path.display(), "meta-coordinator: no prior benchmark results (first run)");
            }
            Err(e) => {
                warn!(path = %path.display(), error = %e, "meta-coordinator: failed to read benchmark file");
            }
        }
    }

    /// Generate a routing summary table from current probe results.
    ///
    /// Returns a human-readable Markdown table sorted by latency (fastest first).
    /// Providers with no probe results are omitted. Used by the meta-coordinator
    /// for periodic routing oversight logging.
    pub fn routing_summary(&self) -> String {
        if self.results.is_empty() {
            return "No probe results available.".to_string();
        }

        let mut lines = vec![
            "| Provider | Model | Status | Latency (ms) |".to_string(),
            "|----------|-------|--------|--------------|".to_string(),
        ];

        let mut sorted = self.results.clone();
        // Sort: success first, then by latency ascending; timeouts/errors last
        sorted.sort_by(|a, b| {
            let a_ok = a.status == ProbeStatus::Success;
            let b_ok = b.status == ProbeStatus::Success;
            match (a_ok, b_ok) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a
                    .latency_ms
                    .unwrap_or(u64::MAX)
                    .cmp(&b.latency_ms.unwrap_or(u64::MAX)),
            }
        });

        for r in &sorted {
            let status = match r.status {
                ProbeStatus::Success => "ok",
                ProbeStatus::Error => "err",
                ProbeStatus::Timeout => "timeout",
            };
            let latency = r
                .latency_ms
                .map(|ms| ms.to_string())
                .unwrap_or_else(|| "-".to_string());
            lines.push(format!(
                "| {} | {} | {} | {} |",
                r.provider, r.model, status, latency
            ));
        }

        lines.join("\n")
    }

    /// Save probe results to a JSON file (pi-benchmark compatible format).
    pub async fn save_results(&self, dir: &std::path::Path) -> std::io::Result<()> {
        tokio::fs::create_dir_all(dir).await?;

        let json = serde_json::to_string_pretty(&self.results).map_err(std::io::Error::other)?;

        let timestamp = chrono::Utc::now().format("%Y-%m-%d-%H%M%S");
        let timestamped = dir.join(format!("{timestamp}.json"));
        let latest = dir.join("latest.json");

        tokio::fs::write(&timestamped, &json).await?;
        tokio::fs::write(&latest, &json).await?;

        info!(
            path = %timestamped.display(),
            results = self.results.len(),
            "probe results saved"
        );
        Ok(())
    }
}

/// Probe a single provider+model by executing its action template.
async fn probe_single(provider: &str, model: &str, action_template: Option<&str>) -> ProbeResult {
    let timestamp = chrono::Utc::now().to_rfc3339();
    let test_prompt = "echo hello";

    let action = match action_template {
        Some(tmpl) => tmpl
            .replace("{{ model }}", model)
            .replace("{{model}}", model)
            .replace("{{ prompt }}", test_prompt)
            .replace("{{prompt}}", test_prompt),
        None => {
            return ProbeResult {
                provider: provider.to_string(),
                model: model.to_string(),
                cli_tool: String::new(),
                status: ProbeStatus::Error,
                latency_ms: None,
                error: Some("no action:: template defined".to_string()),
                timestamp,
            };
        }
    };

    // Extract CLI tool name (first word of action)
    let cli_tool = action
        .split_whitespace()
        .next()
        .unwrap_or("")
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_string();

    let start = Instant::now();
    let timeout = Duration::from_secs(30);

    // Prepend common tool directories to PATH so CLI tools (opencode, claude,
    // cargo, gtr) are found without sourcing .profile (which may have errors).
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/alex".to_string());
    let path_prefix =
        format!("{home}/.local/bin:{home}/.bun/bin:{home}/bin:{home}/.cargo/bin:{home}/go/bin",);
    let result = tokio::time::timeout(timeout, async {
        let output = tokio::process::Command::new("bash")
            .arg("-c")
            .arg(&action)
            .env(
                "PATH",
                format!(
                    "{path_prefix}:{}",
                    std::env::var("PATH").unwrap_or_default()
                ),
            )
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("spawn failed: {e}"))?
            .wait_with_output()
            .await
            .map_err(|e| format!("wait failed: {e}"))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!(
                "exit {}: {}",
                output.status,
                stderr.chars().take(200).collect::<String>()
            ))
        }
    })
    .await;

    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(Ok(())) => {
            info!(provider, model, latency_ms, "probe success");
            ProbeResult {
                provider: provider.to_string(),
                model: model.to_string(),
                cli_tool,
                status: ProbeStatus::Success,
                latency_ms: Some(latency_ms),
                error: None,
                timestamp,
            }
        }
        Ok(Err(e)) => {
            warn!(provider, model, error = %e, "probe failed");
            ProbeResult {
                provider: provider.to_string(),
                model: model.to_string(),
                cli_tool,
                status: ProbeStatus::Error,
                latency_ms: Some(latency_ms),
                error: Some(e),
                timestamp,
            }
        }
        Err(_) => {
            warn!(provider, model, "probe timed out after 30s");
            ProbeResult {
                provider: provider.to_string(),
                model: model.to_string(),
                cli_tool,
                status: ProbeStatus::Timeout,
                latency_ms: Some(latency_ms),
                error: Some("timeout after 30s".to_string()),
                timestamp,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_health_map_is_stale() {
        let map = ProviderHealthMap::new(Duration::from_secs(300));
        assert!(map.is_stale());
    }

    #[test]
    fn unknown_provider_is_healthy() {
        let map = ProviderHealthMap::new(Duration::from_secs(300));
        assert!(map.is_healthy("nonexistent"));
        assert_eq!(map.provider_health("nonexistent"), HealthStatus::Healthy);
    }

    #[test]
    fn record_failures_opens_circuit() {
        let mut map = ProviderHealthMap::new(Duration::from_secs(300));
        for _ in 0..5 {
            map.record_failure("kimi");
        }
        assert!(!map.is_healthy("kimi"));
        assert_eq!(map.provider_health("kimi"), HealthStatus::Unhealthy);
        assert_eq!(map.unhealthy_providers(), vec!["kimi".to_string()]);
    }

    #[test]
    fn record_success_keeps_healthy() {
        let mut map = ProviderHealthMap::new(Duration::from_secs(300));
        map.record_failure("kimi");
        map.record_success("kimi");
        // No probe results, so falls back to circuit breaker
        assert!(map.is_healthy("kimi"));
    }

    #[test]
    fn probe_timeout_marks_unhealthy_immediately() {
        let mut map = ProviderHealthMap::new(Duration::from_secs(300));
        // Simulate a probe that timed out
        map.results = vec![ProbeResult {
            provider: "kimi".to_string(),
            model: "kimi-for-coding/k2p5".to_string(),
            cli_tool: "opencode".to_string(),
            status: ProbeStatus::Timeout,
            latency_ms: Some(30000),
            error: Some("timeout".to_string()),
            timestamp: String::new(),
        }];
        // Should be unhealthy even though circuit breaker has 0 failures
        assert!(!map.is_healthy("kimi"));
        assert_eq!(map.provider_health("kimi"), HealthStatus::Unhealthy);
        assert!(map.unhealthy_providers().contains(&"kimi".to_string()));
    }

    #[test]
    fn probe_success_overrides_circuit_breaker_failures() {
        let mut map = ProviderHealthMap::new(Duration::from_secs(300));
        // Circuit breaker has failures but probe succeeded
        for _ in 0..3 {
            map.record_failure("kimi");
        }
        map.results = vec![ProbeResult {
            provider: "kimi".to_string(),
            model: "kimi-for-coding/k2p5".to_string(),
            cli_tool: "opencode".to_string(),
            status: ProbeStatus::Success,
            latency_ms: Some(5000),
            error: None,
            timestamp: String::new(),
        }];
        // Probe success is authoritative
        assert!(map.is_healthy("kimi"));
    }

    #[test]
    fn mixed_model_results_any_success_means_healthy() {
        let mut map = ProviderHealthMap::new(Duration::from_secs(300));
        map.results = vec![
            ProbeResult {
                provider: "minimax".to_string(),
                model: "opencode-go/minimax-m2.5".to_string(),
                cli_tool: "opencode".to_string(),
                status: ProbeStatus::Timeout,
                latency_ms: Some(30000),
                error: Some("timeout".to_string()),
                timestamp: String::new(),
            },
            ProbeResult {
                provider: "minimax".to_string(),
                model: "minimax-coding-plan/MiniMax-M2.5".to_string(),
                cli_tool: "opencode".to_string(),
                status: ProbeStatus::Success,
                latency_ms: Some(10000),
                error: None,
                timestamp: String::new(),
            },
        ];
        // One model succeeded -> provider is healthy
        assert!(map.is_healthy("minimax"));
    }

    #[test]
    fn routing_summary_empty_returns_message() {
        let map = ProviderHealthMap::new(Duration::from_secs(300));
        assert_eq!(map.routing_summary(), "No probe results available.");
    }

    #[test]
    fn routing_summary_contains_providers() {
        let mut map = ProviderHealthMap::new(Duration::from_secs(300));
        map.results = vec![
            ProbeResult {
                provider: "anthropic".to_string(),
                model: "claude-sonnet-4-6".to_string(),
                cli_tool: "claude".to_string(),
                status: ProbeStatus::Success,
                latency_ms: Some(12876),
                error: None,
                timestamp: "2026-04-06T19:00:00Z".to_string(),
            },
            ProbeResult {
                provider: "kimi".to_string(),
                model: "kimi-for-coding/k2p5".to_string(),
                cli_tool: "opencode".to_string(),
                status: ProbeStatus::Timeout,
                latency_ms: Some(30001),
                error: Some("timeout after 30s".to_string()),
                timestamp: "2026-04-06T19:00:00Z".to_string(),
            },
        ];
        let summary = map.routing_summary();
        assert!(
            summary.contains("anthropic"),
            "summary should contain anthropic"
        );
        assert!(summary.contains("kimi"), "summary should contain kimi");
        assert!(summary.contains("ok"), "summary should contain ok status");
        assert!(
            summary.contains("timeout"),
            "summary should contain timeout status"
        );
        // Success should sort before timeout
        let anthropic_pos = summary.find("anthropic").unwrap();
        let kimi_pos = summary.find("kimi").unwrap();
        assert!(
            anthropic_pos < kimi_pos,
            "successful providers should sort before timeouts"
        );
    }

    #[test]
    fn load_from_file_nonexistent_dir_is_noop() {
        let mut map = ProviderHealthMap::new(Duration::from_secs(300));
        map.load_from_file(std::path::Path::new(
            "/nonexistent/path/that/does/not/exist",
        ));
        // Should remain empty, not panic
        assert!(map.results.is_empty());
        assert!(map.is_stale());
    }

    #[test]
    fn load_from_file_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let results = vec![ProbeResult {
            provider: "anthropic".to_string(),
            model: "claude-sonnet-4-6".to_string(),
            cli_tool: "claude".to_string(),
            status: ProbeStatus::Success,
            latency_ms: Some(12876),
            error: None,
            timestamp: "2026-04-06T19:00:00Z".to_string(),
        }];
        let json = serde_json::to_string_pretty(&results).unwrap();
        std::fs::write(dir.path().join("latest.json"), &json).unwrap();

        let mut map = ProviderHealthMap::new(Duration::from_secs(300));
        map.load_from_file(dir.path());

        assert_eq!(map.results.len(), 1);
        assert_eq!(map.results[0].provider, "anthropic");
        assert!(
            map.is_healthy("anthropic"),
            "loaded success should mark provider healthy"
        );
        // Should not be stale after loading (no need for immediate re-probe)
        assert!(!map.is_stale(), "freshly loaded map should not be stale");
    }
}
