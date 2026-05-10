//! Provider availability probing with per-provider circuit breakers.
//!
//! Reuses [`terraphim_spawner::health::CircuitBreaker`] for tracking provider
//! health state (Closed/Open/HalfOpen). The probe executes `action::` templates
//! from KG routing rules via CLI tools to test the full stack.

use std::collections::{HashMap, HashSet};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeStatus {
    Success,
    Error,
    Timeout,
    RateLimited,
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
    /// Providers currently blocked by rate limits.
    rate_limited: HashSet<String>,
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
                cooldown: Duration::from_secs(300),
                success_threshold: 1,
            },
            rate_limited: HashSet::new(),
        }
    }

    /// Check if a provider is currently rate-limited.
    pub fn is_rate_limited(&self, provider: &str) -> bool {
        self.rate_limited.contains(provider)
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
    pub async fn probe_all(&mut self, kg_router: &KgRouter, is_blocked: &dyn Fn(&str) -> bool) {
        let mut seen = HashMap::new();
        let mut tasks = Vec::new();
        let mut results = Vec::new();

        // Clear rate-limited set; we will rebuild it for this tick.
        self.rate_limited.clear();

        // Collect unique provider+model combos from all KG routing rules
        for rule in kg_router.all_routes() {
            let key = format!("{}:{}", rule.provider, rule.model);
            if seen.contains_key(&key) {
                continue;
            }
            seen.insert(key.clone(), true);

            // Skip probing providers that are currently rate-limited.
            if is_blocked(&rule.provider) {
                debug!(
                    provider = %rule.provider,
                    model = %rule.model,
                    "probe skipped: provider rate-limited"
                );
                self.rate_limited.insert(rule.provider.clone());
                results.push(ProbeResult {
                    provider: rule.provider.clone(),
                    model: rule.model.clone(),
                    cli_tool: String::new(),
                    status: ProbeStatus::RateLimited,
                    latency_ms: None,
                    error: Some("probe skipped: provider rate-limited".to_string()),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                });
                continue;
            }

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

        for task in tasks {
            match task.await {
                Ok(result) => results.push(result),
                Err(e) => warn!(error = %e, "probe task panicked"),
            }
        }

        // Update circuit breakers from probe results (keyed by provider:model).
        // Skip updating the breaker when the probe failed because the CLI tool
        // is missing from PATH — that is an environment/configuration issue,
        // not a provider health issue, and should not open the circuit.
        for result in &results {
            let key = format!("{}:{}", result.provider, result.model);
            let breaker = self
                .breakers
                .entry(key)
                .or_insert_with(|| CircuitBreaker::new(self.cb_config.clone()));

            match result.status {
                ProbeStatus::Success => breaker.record_success(),
                ProbeStatus::Error => {
                    // Do not count CLI-not-found as a provider failure.
                    if result
                        .error
                        .as_ref()
                        .is_some_and(|e| e.contains("CLI tool") && e.contains("not found on PATH"))
                    {
                        debug!(
                            provider = %result.provider,
                            model = %result.model,
                            "skipping circuit-breaker update: CLI tool missing"
                        );
                    } else {
                        breaker.record_failure();
                    }
                }
                ProbeStatus::Timeout => breaker.record_failure(),
                ProbeStatus::RateLimited => {
                    // Rate-limited probes are skipped; do not update breaker.
                    debug!(
                        provider = %result.provider,
                        model = %result.model,
                        "skipping circuit-breaker update: provider rate-limited"
                    );
                }
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
                ProbeStatus::RateLimited => HealthStatus::Degraded,
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
        // Rate-limited providers report as Degraded regardless of other state.
        if self.is_rate_limited(provider) {
            return HealthStatus::Degraded;
        }

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
    /// Returned names are canonicalised via [`canonical_quota_key`].
    pub fn unhealthy_providers(&self) -> Vec<String> {
        let mut providers: HashMap<String, (usize, usize)> = HashMap::new(); // (total, healthy)

        for result in &self.results {
            let canonical = crate::provider_budget::canonical_quota_key(&result.provider);
            let entry = providers.entry(canonical.to_string()).or_insert((0, 0));
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
                let canonical = crate::provider_budget::canonical_quota_key(provider);
                if !unhealthy.contains(&canonical.to_string()) {
                    let entry = cb_providers.entry(canonical.to_string()).or_insert((0, 0));
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

        // Exclude rate-limited providers; they are not truly "unhealthy".
        unhealthy.retain(|p| !self.is_rate_limited(p));

        unhealthy
    }

    /// Record a success for a provider (e.g., from ExitClassifier).
    /// Record a success for a provider+model (e.g., from ExitClassifier).
    pub fn record_success(&mut self, provider: &str) {
        let canonical = crate::provider_budget::canonical_quota_key(provider);
        // Update all circuit breakers for this provider
        let prefix = format!("{canonical}:");
        for (key, breaker) in &mut self.breakers {
            if key.starts_with(&prefix) {
                breaker.record_success();
            }
        }
    }

    /// Record a success for a specific model.
    pub fn record_model_success(&mut self, provider: &str, model: &str) {
        let canonical = crate::provider_budget::canonical_quota_key(provider);
        let key = format!("{canonical}:{model}");
        if let Some(breaker) = self.breakers.get_mut(&key) {
            breaker.record_success();
        }
    }

    /// Record a failure for a provider (affects all models for that provider).
    pub fn record_failure(&mut self, provider: &str) {
        let canonical = crate::provider_budget::canonical_quota_key(provider);
        let prefix = format!("{canonical}:");
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
            let key = format!("{canonical}:*");
            let breaker = self
                .breakers
                .entry(key)
                .or_insert_with(|| CircuitBreaker::new(self.cb_config.clone()));
            breaker.record_failure();
        }

        warn!(
            provider = canonical,
            "provider failure recorded (all models)"
        );
    }

    /// Record a failure for a specific model.
    pub fn record_model_failure(&mut self, provider: &str, model: &str) {
        let canonical = crate::provider_budget::canonical_quota_key(provider);
        let key = format!("{canonical}:{model}");
        let breaker = self
            .breakers
            .entry(key)
            .or_insert_with(|| CircuitBreaker::new(self.cb_config.clone()));
        breaker.record_failure();
        warn!(
            provider = canonical,
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

    /// Send probe results to Quickwit for cost-aware routing analytics.
    pub async fn send_to_quickwit(
        &self,
        sink: &crate::quickwit::QuickwitFleetSink,
        project_id: &str,
    ) {
        for result in &self.results {
            let doc = crate::quickwit::LogDocument {
                timestamp: result.timestamp.clone(),
                project_id: project_id.to_string(),
                level: match result.status {
                    ProbeStatus::Success => "INFO".to_string(),
                    _ => "WARN".to_string(),
                },
                agent_name: "probe".to_string(),
                layer: "Core".to_string(),
                source: "probe".to_string(),
                message: result.error.clone().unwrap_or_else(|| format!("probe {:?}", result.status)),
                model: Some(result.model.clone()),
                cost_usd: Some(0.0), // Probes are zero-cost test calls
                latency_ms: result.latency_ms,
                exit_class: Some(format!("{:?}", result.status).to_lowercase()),
                is_free: true, // Probes don't incur cost
                ..Default::default()
            };
            if let Err(e) = sink.send(doc).await {
                warn!(error = %e, "failed to send probe result to Quickwit");
            }
        }
        info!(results = self.results.len(), "probe results sent to Quickwit");
    }
}

/// Check whether a CLI tool is available on PATH.
///
/// Runs `which <tool>` and returns `true` only when the tool exists and is
/// executable. This lets us distinguish "provider API down" from "CLI tool
/// missing / not on PATH" without executing the actual probe.
fn cli_tool_on_path(tool: &str) -> bool {
    std::process::Command::new("which")
        .arg(tool)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Probe a single provider+model by executing its action template.
///
/// If the model string does not pass the C1 allow-list checked by
/// [`crate::config::is_allowed_provider`], the probe is skipped immediately
/// and returns a sentinel result so banned providers never reach the
/// CLI execution path.
///
/// If the CLI tool referenced by the action template is not on PATH, the
/// probe returns an error but the circuit breaker is **not** updated, so
/// a missing tool does not permanently mark the provider as unhealthy.
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

    // Validate CLI tool is on PATH before attempting probe.
    // A missing tool is an environment/configuration issue, not a provider
    // health issue, so we must not open the circuit breaker for it.
    if !cli_tool.is_empty() && !cli_tool_on_path(&cli_tool) {
        warn!(
            provider = %provider,
            model = %model,
            cli_tool = %cli_tool,
            "probe skipped: CLI tool not found on PATH"
        );
        return ProbeResult {
            provider: provider.to_string(),
            model: model.to_string(),
            cli_tool: cli_tool.clone(),
            status: ProbeStatus::Error,
            latency_ms: None,
            error: Some(format!(
                "probe skipped: CLI tool '{}' not found on PATH",
                cli_tool
            )),
            timestamp,
        };
    }

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

    #[test]
    fn cli_tool_on_path_finds_existing_tools() {
        // `sh` is guaranteed to exist on any POSIX system
        assert!(cli_tool_on_path("sh"));
        assert!(cli_tool_on_path("bash"));
    }

    #[test]
    fn cli_tool_on_path_rejects_missing_tools() {
        // A tool name that almost certainly does not exist
        assert!(!cli_tool_on_path("definitely_not_a_real_tool_12345"));
    }

    #[test]
    fn missing_cli_probe_does_not_open_breaker() {
        let mut map = ProviderHealthMap::new(Duration::from_secs(300));

        // Simulate the exact update logic from `probe_all` with a CLI-not-found
        // result. We create the breaker entry and then run the same match logic
        // that `probe_all` uses.
        let result = ProbeResult {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            cli_tool: "nonexistent-cli".to_string(),
            status: ProbeStatus::Error,
            latency_ms: None,
            error: Some("probe skipped: CLI tool 'nonexistent-cli' not found on PATH".to_string()),
            timestamp: String::new(),
        };

        let key = format!("{}:{}", result.provider, result.model);
        let breaker = map
            .breakers
            .entry(key)
            .or_insert_with(|| CircuitBreaker::new(map.cb_config.clone()));

        // Replicate the logic from `probe_all`.
        match result.status {
            ProbeStatus::Success => breaker.record_success(),
            ProbeStatus::Error => {
                if result
                    .error
                    .as_ref()
                    .is_some_and(|e| e.contains("CLI tool") && e.contains("not found on PATH"))
                {
                    // Skipped — this is what we want to verify.
                } else {
                    breaker.record_failure();
                }
            }
            ProbeStatus::Timeout => breaker.record_failure(),
            ProbeStatus::RateLimited => {
                // Skipped — same as CLI-not-found.
            }
        }

        // The breaker must still be Closed because the CLI-not-found error
        // was skipped.
        assert!(matches!(
            breaker.state(),
            terraphim_spawner::health::CircuitState::Closed
        ));

        // When there are no probe results, the provider falls back to the
        // circuit breaker state, which is Closed (healthy).
        assert!(map.is_healthy("openai"));
    }

    #[test]
    fn probe_status_rate_limited_serialisation() {
        let status = ProbeStatus::RateLimited;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"rate_limited\"");
        let deserialized: ProbeStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ProbeStatus::RateLimited);
    }

    #[tokio::test]
    async fn probe_skips_rate_limited_provider() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("test.md"),
            r#"# Test
route:: test-provider, test-model
action:: test-cli-tool --model {{ model }}
"#,
        )
        .unwrap();

        let kg_router = crate::kg_router::KgRouter::load(dir.path()).unwrap();
        let mut map = ProviderHealthMap::new(Duration::from_secs(300));

        // Block the test provider.
        map.probe_all(&kg_router, &|provider: &str| provider == "test-provider")
            .await;

        // Should have recorded a RateLimited result.
        let result = map
            .results
            .iter()
            .find(|r| r.provider == "test-provider" && r.model == "test-model")
            .expect("RateLimited result should be recorded");
        assert_eq!(result.status, ProbeStatus::RateLimited);
        assert_eq!(
            result.error,
            Some("probe skipped: provider rate-limited".to_string())
        );

        // Provider should be tracked as rate-limited.
        assert!(map.is_rate_limited("test-provider"));

        // Health should report Degraded.
        assert_eq!(
            map.model_health("test-provider", "test-model"),
            HealthStatus::Degraded
        );
        assert_eq!(map.provider_health("test-provider"), HealthStatus::Degraded);
    }

    #[tokio::test]
    async fn rate_limit_expiry_triggers_reprobe() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("test.md"),
            r#"# Test
route:: test-provider, test-model
action:: definitely-not-a-real-cli-12345 --model {{ model }}
"#,
        )
        .unwrap();

        let kg_router = crate::kg_router::KgRouter::load(dir.path()).unwrap();
        let mut map = ProviderHealthMap::new(Duration::from_secs(300));

        // First call: provider is blocked -> RateLimited.
        map.probe_all(&kg_router, &|_: &str| true).await;
        let first_result = map
            .results
            .iter()
            .find(|r| r.provider == "test-provider")
            .expect("Result should exist");
        assert_eq!(first_result.status, ProbeStatus::RateLimited);

        // Second call: block has "expired" (is_blocked returns false).
        // The probe will spawn the CLI, which does not exist, yielding Error.
        map.probe_all(&kg_router, &|_: &str| false).await;
        let second_result = map
            .results
            .iter()
            .find(|r| r.provider == "test-provider")
            .expect("Result should exist");

        // After expiry the provider is re-probed (not skipped due to rate limit).
        // It may still fail for other reasons (C1 allow-list, CLI not found, etc.),
        // but it must NOT be RateLimited.
        assert_ne!(
            second_result.status,
            ProbeStatus::RateLimited,
            "Provider should be re-probed after rate limit expiry, not skipped. Got: {:?}",
            second_result
        );

        // Provider is no longer tracked as rate-limited.
        assert!(!map.is_rate_limited("test-provider"));
    }

    #[test]
    fn rate_limited_does_not_open_circuit_breaker() {
        let mut map = ProviderHealthMap::new(Duration::from_secs(300));

        // Record 4 failures for kimi (threshold is 5)
        for _ in 0..4 {
            map.record_failure("kimi");
        }

        // Simulate the exact update logic from `probe_all` with a RateLimited
        // result. We create the breaker entry and then run the same match logic.
        let result = ProbeResult {
            provider: "kimi".to_string(),
            model: "kimi-for-coding/k2p5".to_string(),
            cli_tool: "opencode".to_string(),
            status: ProbeStatus::RateLimited,
            latency_ms: None,
            error: Some("probe skipped: provider rate-limited".to_string()),
            timestamp: String::new(),
        };

        let key = format!("{}:{}", result.provider, result.model);
        let breaker = map
            .breakers
            .entry(key)
            .or_insert_with(|| CircuitBreaker::new(map.cb_config.clone()));

        // Replicate the logic from `probe_all`.
        match result.status {
            ProbeStatus::Success => breaker.record_success(),
            ProbeStatus::Error => {
                if result
                    .error
                    .as_ref()
                    .is_some_and(|e| e.contains("CLI tool") && e.contains("not found on PATH"))
                {
                    // Skipped
                } else {
                    breaker.record_failure();
                }
            }
            ProbeStatus::Timeout => breaker.record_failure(),
            ProbeStatus::RateLimited => {
                // Skipped — same as CLI-not-found.
            }
        }

        // The breaker must still be Closed because the RateLimited error
        // was skipped (failure count remains at 4, not 5).
        assert!(matches!(
            breaker.state(),
            terraphim_spawner::health::CircuitState::Closed
        ));

        // Provider is still healthy because the circuit breaker is Closed.
        assert!(map.is_healthy("kimi"));
    }

    #[test]
    fn model_health_returns_degraded_for_rate_limited() {
        let mut map = ProviderHealthMap::new(Duration::from_secs(300));
        map.rate_limited.insert("anthropic".to_string());
        map.results = vec![ProbeResult {
            provider: "anthropic".to_string(),
            model: "claude-sonnet".to_string(),
            cli_tool: "claude".to_string(),
            status: ProbeStatus::RateLimited,
            latency_ms: None,
            error: Some("probe skipped: provider rate-limited".to_string()),
            timestamp: String::new(),
        }];

        assert_eq!(
            map.model_health("anthropic", "claude-sonnet"),
            HealthStatus::Degraded
        );
    }

    #[test]
    fn provider_health_returns_degraded_for_rate_limited() {
        let mut map = ProviderHealthMap::new(Duration::from_secs(300));
        map.rate_limited.insert("anthropic".to_string());

        // Even with no probe results, a rate-limited provider reports Degraded.
        assert_eq!(map.provider_health("anthropic"), HealthStatus::Degraded);
    }

    #[test]
    fn unhealthy_providers_excludes_rate_limited() {
        let mut map = ProviderHealthMap::new(Duration::from_secs(300));

        // Simulate a provider with all models failing.
        map.results = vec![
            ProbeResult {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                cli_tool: "opencode".to_string(),
                status: ProbeStatus::Error,
                latency_ms: Some(5000),
                error: Some("API key invalid".to_string()),
                timestamp: String::new(),
            },
            ProbeResult {
                provider: "anthropic".to_string(),
                model: "claude-sonnet".to_string(),
                cli_tool: "claude".to_string(),
                status: ProbeStatus::RateLimited,
                latency_ms: None,
                error: Some("probe skipped: provider rate-limited".to_string()),
                timestamp: String::new(),
            },
        ];

        // Mark anthropic as rate-limited.
        map.rate_limited.insert("anthropic".to_string());

        let unhealthy = map.unhealthy_providers();
        assert!(unhealthy.contains(&"openai".to_string()));
        assert!(!unhealthy.contains(&"anthropic".to_string()));
    }
}
