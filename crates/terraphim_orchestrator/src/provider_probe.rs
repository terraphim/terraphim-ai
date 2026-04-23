//! Provider availability probing with per-provider circuit breakers.
//!
//! Reuses [`terraphim_spawner::health::CircuitBreaker`] for tracking provider
//! health state (Closed/Open/HalfOpen). The probe executes `action::` templates
//! from KG routing rules via CLI tools to test the full stack.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use terraphim_spawner::health::{CircuitBreaker, CircuitBreakerConfig, CircuitState, HealthStatus};
use tracing::{debug, info, warn};

use crate::kg_router::KgRouter;

/// Result of probing a single provider+model combination.
#[derive(Debug, Clone, serde::Serialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
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
                failure_threshold: 2,
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
            seen.insert(key.clone(), true);

            // Skip probing models whose circuit breaker is already Open.
            // Probing a known-unhealthy provider wastes tokens and spawns
            // processes that will likely timeout and leak.
            if let Some(breaker) = self.breakers.get(&key) {
                if matches!(breaker.state(), CircuitState::Open) {
                    debug!(
                        provider = %rule.provider,
                        model = %rule.model,
                        "probe skipped: circuit breaker open"
                    );
                    continue;
                }
            }

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

        // Update circuit breakers from probe results (keyed by provider:model)
        for result in &results {
            let key = format!("{}:{}", result.provider, result.model);
            let breaker = self
                .breakers
                .entry(key)
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

    /// Get health status for a specific provider+model combination.
    ///
    /// Uses **probe results first** at the model level, then falls back to
    /// circuit breaker. This is per-model, not per-provider aggregate.
    pub fn model_health(&self, provider: &str, model: &str) -> HealthStatus {
        let key = format!("{provider}:{model}");

        // Check latest probe result for this exact model
        if let Some(result) = self
            .results
            .iter()
            .find(|r| r.provider == provider && r.model == model)
        {
            return match result.status {
                ProbeStatus::Success => HealthStatus::Healthy,
                ProbeStatus::Error => HealthStatus::Unhealthy,
                ProbeStatus::Timeout => HealthStatus::Unhealthy,
            };
        }

        // Fall back to circuit breaker (keyed by provider:model)
        match self.breakers.get(&key) {
            Some(breaker) => match breaker.state() {
                CircuitState::Closed => HealthStatus::Healthy,
                CircuitState::HalfOpen => HealthStatus::Degraded,
                CircuitState::Open => HealthStatus::Unhealthy,
            },
            None => HealthStatus::Healthy,
        }
    }

    /// Get aggregate health status for a provider (any model healthy = provider healthy).
    pub fn provider_health(&self, provider: &str) -> HealthStatus {
        // Check probe results at model level
        let provider_results: Vec<_> = self
            .results
            .iter()
            .filter(|r| r.provider == provider)
            .collect();

        if !provider_results.is_empty() {
            if provider_results
                .iter()
                .any(|r| r.status == ProbeStatus::Success)
            {
                return HealthStatus::Healthy;
            }
            return HealthStatus::Unhealthy;
        }

        // Fall back to circuit breakers for this provider
        let provider_breakers: Vec<_> = self
            .breakers
            .iter()
            .filter(|(k, _)| k.starts_with(&format!("{provider}:")))
            .collect();

        if provider_breakers.is_empty() {
            return HealthStatus::Healthy; // Unknown
        }

        if provider_breakers.iter().any(|(_, b)| b.should_allow()) {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy
        }
    }

    /// Check if a provider is healthy enough to dispatch to.
    pub fn is_healthy(&self, provider: &str) -> bool {
        matches!(
            self.provider_health(provider),
            HealthStatus::Healthy | HealthStatus::Degraded
        )
    }

    /// Check if a specific model is healthy.
    pub fn is_model_healthy(&self, provider: &str, model: &str) -> bool {
        matches!(
            self.model_health(provider, model),
            HealthStatus::Healthy | HealthStatus::Degraded
        )
    }

    /// List all unhealthy provider names.
    ///
    /// A provider is unhealthy only if ALL its models are unhealthy.
    pub fn unhealthy_providers(&self) -> Vec<String> {
        let mut providers: HashMap<String, (usize, usize)> = HashMap::new(); // (total, healthy)

        for result in &self.results {
            let entry = providers.entry(result.provider.clone()).or_insert((0, 0));
            entry.0 += 1;
            if result.status == ProbeStatus::Success {
                entry.1 += 1;
            }
        }

        let mut unhealthy: Vec<String> = providers
            .into_iter()
            .filter(|(_, (total, healthy))| *total > 0 && *healthy == 0)
            .map(|(name, _)| name)
            .collect();

        // Also check circuit breakers for providers not in probe results
        let mut cb_providers: HashMap<String, (usize, usize)> = HashMap::new();
        for (key, breaker) in &self.breakers {
            if let Some(provider) = key.split(':').next() {
                if !unhealthy.contains(&provider.to_string()) {
                    let entry = cb_providers.entry(provider.to_string()).or_insert((0, 0));
                    entry.0 += 1;
                    if breaker.should_allow() {
                        entry.1 += 1;
                    }
                }
            }
        }

        for (name, (total, healthy)) in cb_providers {
            if total > 0 && healthy == 0 && !unhealthy.contains(&name) {
                unhealthy.push(name);
            }
        }

        unhealthy
    }

    /// Record a success for a provider (e.g., from ExitClassifier).
    /// Record a success for a provider+model (e.g., from ExitClassifier).
    pub fn record_success(&mut self, provider: &str) {
        // Update all circuit breakers for this provider
        let prefix = format!("{provider}:");
        for (key, breaker) in &mut self.breakers {
            if key.starts_with(&prefix) {
                breaker.record_success();
            }
        }
    }

    /// Record a success for a specific model.
    pub fn record_model_success(&mut self, provider: &str, model: &str) {
        let key = format!("{provider}:{model}");
        if let Some(breaker) = self.breakers.get_mut(&key) {
            breaker.record_success();
        }
    }

    /// Record a failure for a provider (affects all models for that provider).
    pub fn record_failure(&mut self, provider: &str) {
        let prefix = format!("{provider}:");
        let keys: Vec<String> = self
            .breakers
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .cloned()
            .collect();

        for key in keys {
            let breaker = self.breakers.get_mut(&key).unwrap();
            breaker.record_failure();
        }

        // If no breakers exist for this provider, create one at provider level
        if !self.breakers.keys().any(|k| k.starts_with(&prefix)) {
            let key = format!("{provider}:*");
            let breaker = self
                .breakers
                .entry(key)
                .or_insert_with(|| CircuitBreaker::new(self.cb_config.clone()));
            breaker.record_failure();
        }

        warn!(
            provider = provider,
            "provider failure recorded (all models)"
        );
    }

    /// Record a failure for a specific model.
    pub fn record_model_failure(&mut self, provider: &str, model: &str) {
        let key = format!("{provider}:{model}");
        let breaker = self
            .breakers
            .entry(key)
            .or_insert_with(|| CircuitBreaker::new(self.cb_config.clone()));
        breaker.record_failure();
        warn!(
            provider = provider,
            model = model,
            state = %breaker.state(),
            "model failure recorded"
        );
    }

    /// Get the latest probe results.
    pub fn results(&self) -> &[ProbeResult] {
        &self.results
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
///
/// If the model string does not pass the C1 allow-list checked by
/// [`crate::config::is_allowed_provider`], the probe is skipped immediately
/// and returns a sentinel result so banned providers never reach the
/// CLI execution path.
async fn probe_single(provider: &str, model: &str, action_template: Option<&str>) -> ProbeResult {
    let timestamp = chrono::Utc::now().to_rfc3339();

    // C1 gate: skip probe for any provider not in the subscription allow-list.
    if !crate::config::is_allowed_provider(model) {
        warn!(
            model = %model,
            provider = %provider,
            "probe skipped: provider not in C1 allow-list"
        );
        return ProbeResult {
            provider: provider.to_string(),
            model: model.to_string(),
            cli_tool: String::new(),
            status: ProbeStatus::Error,
            latency_ms: None,
            error: Some("probe skipped: provider not in C1 allow-list".to_string()),
            timestamp,
        };
    }

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
    let timeout = Duration::from_secs(60);

    debug!(provider, model, action = %action, "running probe command");

    // Prepend common tool directories to PATH so CLI tools (opencode, claude,
    // cargo, gtr) are found without sourcing .profile (which may have errors).
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/alex".to_string());
    let path_prefix =
        format!("{home}/.local/bin:{home}/.bun/bin:{home}/bin:{home}/.cargo/bin:{home}/go/bin",);

    // Spawn the child process BEFORE the timeout so we can kill it explicitly
    // if the probe hangs.  Previously the timeout dropped the future without
    // terminating the underlying bash/opencode process, causing task
    // accumulation in systemd (observed: 1 244 leaked tasks).
    let child = match tokio::process::Command::new("bash")
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
    {
        Ok(c) => c,
        Err(e) => {
            return ProbeResult {
                provider: provider.to_string(),
                model: model.to_string(),
                cli_tool,
                status: ProbeStatus::Error,
                latency_ms: None,
                error: Some(format!("spawn failed: {e}")),
                timestamp,
            };
        }
    };

    let pid = child.id();
    let result = tokio::time::timeout(timeout, child.wait_with_output()).await;

    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(Ok(output)) => {
            if output.status.success() {
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
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let err = format!(
                    "exit {}: {}",
                    output.status,
                    stderr.chars().take(200).collect::<String>()
                );
                warn!(provider, model, error = %err, "probe failed");
                ProbeResult {
                    provider: provider.to_string(),
                    model: model.to_string(),
                    cli_tool,
                    status: ProbeStatus::Error,
                    latency_ms: Some(latency_ms),
                    error: Some(err),
                    timestamp,
                }
            }
        }
        Ok(Err(e)) => {
            let err = format!("wait failed: {e}");
            warn!(provider, model, error = %err, "probe failed");
            ProbeResult {
                provider: provider.to_string(),
                model: model.to_string(),
                cli_tool,
                status: ProbeStatus::Error,
                latency_ms: Some(latency_ms),
                error: Some(err),
                timestamp,
            }
        }
        Err(_) => {
            // Timeout: the bash/opencode process is still running.  Explicitly
            // SIGKILL it to prevent process leaks.
            if let Some(pid) = pid {
                let _ = std::process::Command::new("kill")
                    .arg("-9")
                    .arg(pid.to_string())
                    .spawn();
            }
            warn!(provider, model, "probe timed out after 60s");
            ProbeResult {
                provider: provider.to_string(),
                model: model.to_string(),
                cli_tool,
                status: ProbeStatus::Timeout,
                latency_ms: Some(latency_ms),
                error: Some("timeout after 60s".to_string()),
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
        // But the timed-out model is individually unhealthy
        assert!(!map.is_model_healthy("minimax", "opencode-go/minimax-m2.5"));
        assert!(map.is_model_healthy("minimax", "minimax-coding-plan/MiniMax-M2.5"));
    }

    #[test]
    fn per_model_failure_does_not_affect_other_models() {
        let mut map = ProviderHealthMap::new(Duration::from_secs(300));
        // Opus fails but sonnet works
        map.results = vec![
            ProbeResult {
                provider: "anthropic".to_string(),
                model: "claude-opus-4-6".to_string(),
                cli_tool: "claude".to_string(),
                status: ProbeStatus::Error,
                latency_ms: Some(5000),
                error: Some("rate limited".to_string()),
                timestamp: String::new(),
            },
            ProbeResult {
                provider: "anthropic".to_string(),
                model: "claude-sonnet-4-6".to_string(),
                cli_tool: "claude".to_string(),
                status: ProbeStatus::Success,
                latency_ms: Some(8000),
                error: None,
                timestamp: String::new(),
            },
        ];
        // Provider healthy (sonnet works)
        assert!(map.is_healthy("anthropic"));
        // But opus individually unhealthy
        assert!(!map.is_model_healthy("anthropic", "claude-opus-4-6"));
        assert!(map.is_model_healthy("anthropic", "claude-sonnet-4-6"));
        // Provider NOT in unhealthy list (sonnet keeps it alive)
        assert!(!map.unhealthy_providers().contains(&"anthropic".to_string()));
    }
}
