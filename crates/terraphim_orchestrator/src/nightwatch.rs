use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use terraphim_spawner::health::HealthStatus;
use terraphim_spawner::output::OutputEvent;
use tokio::sync::mpsc;

use crate::config::NightwatchConfig;

/// A claim within a reasoning certificate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    /// The claim statement.
    pub claim: String,
    /// Evidence supporting the claim.
    pub evidence: String,
    /// Optional dimension or category for the claim.
    pub dimension: Option<String>,
}

/// Semi-formal reasoning certificate (arXiv:2603.01896 Phase 4).
/// Produced by the CTO Executive System judge for audit trail integration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReasoningCertificate {
    /// Premises or assumptions for the reasoning.
    pub premises: Vec<String>,
    /// Claims derived from the premises.
    pub claims: Vec<Claim>,
    /// Edge cases or boundary conditions considered.
    pub edge_cases: Vec<String>,
    /// The formal conclusion of the reasoning.
    pub formal_conclusion: String,
    /// Confidence score (0.0-1.0).
    pub confidence: f64,
}

/// Validates a reasoning certificate according to minimum quality criteria.
///
/// Returns true if:
/// - At least 2 premises are provided
/// - At least 1 claim is present
/// - The formal conclusion is non-empty
/// - Confidence is greater than 0.0
pub fn validate_certificate(cert: &ReasoningCertificate) -> bool {
    cert.premises.len() >= 2
        && !cert.claims.is_empty()
        && !cert.formal_conclusion.is_empty()
        && cert.confidence > 0.0
}

/// Dual-panel evaluation result for detecting drift through independent assessments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualPanelResult {
    /// Automated metric (e.g., certificate completeness).
    pub panel_a_score: f64,
    /// Independent metric (e.g., output length/structure).
    pub panel_b_score: f64,
    /// How closely panels agree (0.0-1.0).
    pub agreement: f64,
    /// True if panels disagree significantly.
    pub drift_detected: bool,
    /// Human-readable summary.
    pub details: String,
}

/// Evaluate agent output using dual-panel assessment.
///
/// Panel A evaluates certificate quality (if present) by checking:
/// - Presence of sufficient premises (>= 2)
/// - Presence of claims
/// - Non-empty conclusion
/// - Positive confidence score
///
/// Panel B evaluates output structure by checking:
/// - Presence of section headers (## or similar)
/// - Presence of evidence markers ("evidence:", "because", etc.)
/// - Presence of conclusion markers ("conclusion:", "therefore", etc.)
///
/// Returns a `DualPanelResult` with agreement score and drift detection flag.
/// Drift is detected when panel agreement is below 0.5.
pub fn dual_panel_evaluate(
    output: &str,
    certificate: Option<&ReasoningCertificate>,
) -> DualPanelResult {
    // Panel A: Certificate quality score (0.0-1.0)
    let panel_a_score = if let Some(cert) = certificate {
        calculate_certificate_score(cert)
    } else {
        0.0
    };

    // Panel B: Output structure score (0.0-1.0)
    let panel_b_score = calculate_structure_score(output);

    // Calculate agreement: 1.0 - absolute difference
    let agreement = 1.0 - (panel_a_score - panel_b_score).abs();

    // Drift detected if agreement is below threshold
    let drift_detected = agreement < 0.5;

    // Build human-readable details
    let details = format!(
        "Panel A (certificate): {:.2}, Panel B (structure): {:.2}, Agreement: {:.2} - {}",
        panel_a_score,
        panel_b_score,
        agreement,
        if drift_detected {
            "DRIFT DETECTED: panels disagree significantly"
        } else {
            "No drift: panels agree"
        }
    );

    DualPanelResult {
        panel_a_score,
        panel_b_score,
        agreement,
        drift_detected,
        details,
    }
}

/// Calculate certificate quality score (0.0-1.0).
///
/// Base score of 0.5 if certificate passes validation.
/// Additional points for:
/// - Multiple premises (> 2)
/// - Multiple claims
/// - Edge cases considered
/// - High confidence (> 0.8)
fn calculate_certificate_score(cert: &ReasoningCertificate) -> f64 {
    if !validate_certificate(cert) {
        return 0.0;
    }

    let mut score: f64 = 0.5; // Base score for passing validation

    // Bonus for extra premises
    if cert.premises.len() > 2 {
        score += 0.1;
    }

    // Bonus for multiple claims
    if cert.claims.len() > 1 {
        score += 0.1;
    }

    // Bonus for edge cases
    if !cert.edge_cases.is_empty() {
        score += 0.1;
    }

    // Bonus for high confidence
    if cert.confidence > 0.8 {
        score += 0.2;
    }

    score.min(1.0)
}

/// Calculate output structure score (0.0-1.0).
///
/// Checks for:
/// - Section headers (##, ###)
/// - Evidence markers
/// - Conclusion markers
/// - Minimum length
fn calculate_structure_score(output: &str) -> f64 {
    let lower = output.to_lowercase();
    let mut score: f64 = 0.0;

    // Check for section headers
    if lower.contains("##") || lower.contains("###") {
        score += 0.3;
    }

    // Check for evidence markers
    if lower.contains("evidence:")
        || lower.contains("because")
        || lower.contains("since")
        || lower.contains("given that")
    {
        score += 0.3;
    }

    // Check for conclusion markers
    if lower.contains("conclusion:")
        || lower.contains("therefore")
        || lower.contains("thus")
        || lower.contains("in conclusion")
    {
        score += 0.3;
    }

    // Minimum length check (at least 100 chars for meaningful content)
    if output.len() >= 100 {
        score += 0.1;
    }

    score.min(1.0)
}

/// Behavioral drift metrics for a single agent.
#[derive(Debug, Clone, Default)]
pub struct DriftMetrics {
    /// Errors / total output lines.
    pub error_rate: f64,
    /// Non-error commands / total commands.
    pub command_success_rate: f64,
    /// Process health from HealthStatus observations.
    pub health_score: f64,
    /// Cost efficiency (tokens per dollar, 0.0 if no cost data).
    pub cost_efficiency: f64,
    /// Budget exhaustion rate (spent / budget, 1.0 if uncapped).
    pub budget_exhaustion_rate: f64,
    /// Number of samples in the evaluation window.
    pub sample_count: u64,
}

/// Drift score combining all metrics into a single 0.0-1.0 value.
#[derive(Debug, Clone)]
pub struct DriftScore {
    pub agent_name: String,
    pub score: f64,
    pub metrics: DriftMetrics,
    pub level: CorrectionLevel,
}

/// Correction level based on drift severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CorrectionLevel {
    /// No drift detected.
    Normal,
    /// 10-20% drift: log warning, refresh context.
    Minor,
    /// 20-40% drift: reload config.
    Moderate,
    /// 40-70% drift: restart agent.
    Severe,
    /// >70% drift: pause agent, escalate to human.
    Critical,
}

/// Alert emitted by NightwatchMonitor when drift exceeds threshold.
#[derive(Debug, Clone)]
pub struct DriftAlert {
    pub agent_name: String,
    pub drift_score: DriftScore,
    pub recommended_action: CorrectionAction,
}

/// Action the orchestrator should take in response to drift.
#[derive(Debug, Clone)]
pub enum CorrectionAction {
    /// Log and continue.
    LogWarning(String),
    /// Restart the agent.
    RestartAgent,
    /// Pause agent and notify human.
    PauseAndEscalate(String),
}

/// Per-agent metric accumulator for the evaluation window.
#[derive(Debug, Default)]
struct AgentMetricAccumulator {
    total_lines: u64,
    error_lines: u64,
    health_checks: u64,
    healthy_checks: u64,
    /// Total cost observed for this agent (USD).
    total_cost_usd: f64,
    /// Total tokens observed for this agent.
    total_tokens: u64,
    /// Budget cap for this agent (None = uncapped).
    budget_cents: Option<u64>,
}

impl AgentMetricAccumulator {
    fn drift_metrics(&self) -> DriftMetrics {
        if self.total_lines == 0 && self.health_checks == 0 {
            return DriftMetrics {
                error_rate: 0.0,
                command_success_rate: 1.0,
                health_score: 1.0,
                cost_efficiency: 0.0,
                budget_exhaustion_rate: 0.0, // No budget exhaustion when no samples
                sample_count: 0,
            };
        }

        let error_rate = if self.total_lines > 0 {
            self.error_lines as f64 / self.total_lines as f64
        } else {
            0.0
        };

        let command_success_rate = if self.total_lines > 0 {
            1.0 - error_rate
        } else {
            1.0
        };

        let health_score = if self.health_checks > 0 {
            self.healthy_checks as f64 / self.health_checks as f64
        } else {
            1.0
        };

        // Calculate cost efficiency (tokens per dollar)
        let cost_efficiency = if self.total_cost_usd > 0.0 {
            self.total_tokens as f64 / self.total_cost_usd
        } else {
            0.0
        };

        // Calculate budget exhaustion rate
        // 0.0 = no spend or uncapped, 1.0 = 100% of budget spent
        let budget_exhaustion_rate = match self.budget_cents {
            Some(cents) if cents > 0 => {
                let spent_cents = (self.total_cost_usd * 100.0) as u64;
                spent_cents as f64 / cents as f64
            }
            _ => 0.0, // Uncapped agents have 0 exhaustion (no limit to hit)
        };

        DriftMetrics {
            error_rate,
            command_success_rate,
            health_score,
            cost_efficiency,
            budget_exhaustion_rate,
            sample_count: self.total_lines + self.health_checks,
        }
    }

    fn reset(&mut self) {
        self.total_lines = 0;
        self.error_lines = 0;
        self.health_checks = 0;
        self.healthy_checks = 0;
        self.total_cost_usd = 0.0;
        self.total_tokens = 0;
        // budget_cents is preserved during reset
    }
}

/// Monitors agent behavior and detects drift.
pub struct NightwatchMonitor {
    config: NightwatchConfig,
    agent_metrics: HashMap<String, AgentMetricAccumulator>,
    alert_tx: mpsc::Sender<DriftAlert>,
    alert_rx: mpsc::Receiver<DriftAlert>,
}

impl NightwatchMonitor {
    /// Create a new monitor with the given configuration.
    pub fn new(config: NightwatchConfig) -> Self {
        let (alert_tx, alert_rx) = mpsc::channel(64);
        Self {
            config,
            agent_metrics: HashMap::new(),
            alert_tx,
            alert_rx,
        }
    }

    /// Feed an output event from an agent into the monitor.
    pub fn observe(&mut self, agent_name: &str, event: &OutputEvent) {
        let acc = self
            .agent_metrics
            .entry(agent_name.to_string())
            .or_default();

        match event {
            OutputEvent::Stdout { .. } => {
                acc.total_lines += 1;
            }
            OutputEvent::Stderr { line, .. } => {
                acc.total_lines += 1;
                // Only count as error if line contains error-like keywords.
                // Many CLIs (codex/bun) write normal output to stderr.
                let lower = line.to_lowercase();
                if lower.contains("error")
                    || lower.contains("panic")
                    || lower.contains("fatal")
                    || lower.contains("failed")
                {
                    acc.error_lines += 1;
                }
            }
            OutputEvent::Mention { .. } => {
                acc.total_lines += 1;
            }
            OutputEvent::Completed { .. } => {}
        }
    }

    /// Feed a health status update into the monitor.
    pub fn observe_health(&mut self, agent_name: &str, status: HealthStatus) {
        let acc = self
            .agent_metrics
            .entry(agent_name.to_string())
            .or_default();
        acc.health_checks += 1;
        if status == HealthStatus::Healthy {
            acc.healthy_checks += 1;
        }
    }

    /// Feed cost and token usage data into the monitor.
    /// This enables cost-based drift detection for nightshift decisions.
    pub fn observe_cost(
        &mut self,
        agent_name: &str,
        cost_usd: f64,
        input_tokens: u64,
        output_tokens: u64,
        budget_cents: Option<u64>,
    ) {
        let acc = self
            .agent_metrics
            .entry(agent_name.to_string())
            .or_default();
        acc.total_cost_usd += cost_usd;
        acc.total_tokens += input_tokens + output_tokens;
        // Only update budget if not already set (first call wins)
        if acc.budget_cents.is_none() {
            acc.budget_cents = budget_cents;
        }
    }

    /// Get the next drift alert (async, used in select!).
    pub async fn next_alert(&mut self) -> DriftAlert {
        self.alert_rx
            .recv()
            .await
            .expect("alert channel should never close while monitor exists")
    }

    /// Evaluate drift for all agents and emit alerts for any that exceed thresholds.
    pub fn evaluate(&mut self) {
        let mut alerts = Vec::new();
        for (name, acc) in &self.agent_metrics {
            let metrics = acc.drift_metrics();
            let score = self.calculate_drift(&metrics);
            let level = self.classify_drift(score);
            if level > CorrectionLevel::Normal {
                let action = Self::recommended_action(level, name);
                alerts.push(DriftAlert {
                    agent_name: name.clone(),
                    drift_score: DriftScore {
                        agent_name: name.clone(),
                        score,
                        metrics,
                        level,
                    },
                    recommended_action: action,
                });
            }
        }
        for alert in alerts {
            let _ = self.alert_tx.try_send(alert);
        }
    }

    /// Get current drift score for an agent (synchronous query).
    pub fn drift_score(&self, agent_name: &str) -> Option<DriftScore> {
        self.agent_metrics.get(agent_name).map(|acc| {
            let metrics = acc.drift_metrics();
            let score = self.calculate_drift(&metrics);
            let level = self.classify_drift(score);
            DriftScore {
                agent_name: agent_name.to_string(),
                score,
                metrics,
                level,
            }
        })
    }

    /// Get all current drift scores.
    pub fn all_drift_scores(&self) -> Vec<DriftScore> {
        self.agent_metrics
            .iter()
            .map(|(name, acc)| {
                let metrics = acc.drift_metrics();
                let score = self.calculate_drift(&metrics);
                let level = self.classify_drift(score);
                DriftScore {
                    agent_name: name.clone(),
                    score,
                    metrics,
                    level,
                }
            })
            .collect()
    }

    /// Reset metrics for an agent (after restart).
    pub fn reset(&mut self, agent_name: &str) {
        if let Some(acc) = self.agent_metrics.get_mut(agent_name) {
            acc.reset();
        }
    }

    /// Calculate drift as weighted average of metric deviations.
    /// Returns 0.0 when no samples have been collected.
    /// Includes budget exhaustion in calculation - agents near budget limits
    /// are flagged for potential replacement (nightshift consideration).
    /// Weights are configurable via NightwatchConfig.
    fn calculate_drift(&self, metrics: &DriftMetrics) -> f64 {
        if metrics.sample_count == 0 {
            return 0.0;
        }

        // Use configurable weights from config
        let error_weight = self.config.error_weight;
        let success_weight = self.config.success_weight;
        let health_weight = self.config.health_weight;
        let budget_weight = self.config.budget_weight;

        let error_drift = metrics.error_rate;
        let success_drift = 1.0 - metrics.command_success_rate;
        let health_drift = 1.0 - metrics.health_score;

        // Budget exhaustion contributes to drift when near/exceeding limits
        // >80% budget usage starts contributing significantly
        // Uncapped agents have budget_exhaustion_rate = 0.0 (no limit to exhaust)
        let budget_drift = if metrics.budget_exhaustion_rate > 0.8 {
            (metrics.budget_exhaustion_rate - 0.8) * 5.0 // Scale 0.8-1.0 to 0.0-1.0
        } else {
            0.0
        };

        error_weight * error_drift
            + success_weight * success_drift
            + health_weight * health_drift
            + budget_weight * budget_drift
    }

    /// Classify a drift score into a correction level.
    fn classify_drift(&self, score: f64) -> CorrectionLevel {
        if score >= self.config.critical_threshold {
            CorrectionLevel::Critical
        } else if score >= self.config.severe_threshold {
            CorrectionLevel::Severe
        } else if score >= self.config.moderate_threshold {
            CorrectionLevel::Moderate
        } else if score >= self.config.minor_threshold {
            CorrectionLevel::Minor
        } else {
            CorrectionLevel::Normal
        }
    }

    /// Map correction level to recommended action.
    fn recommended_action(level: CorrectionLevel, agent_name: &str) -> CorrectionAction {
        match level {
            CorrectionLevel::Normal => CorrectionAction::LogWarning("no action needed".to_string()),
            CorrectionLevel::Minor => {
                CorrectionAction::LogWarning(format!("minor drift detected for {}", agent_name))
            }
            CorrectionLevel::Moderate => CorrectionAction::RestartAgent,
            CorrectionLevel::Severe => CorrectionAction::RestartAgent,
            CorrectionLevel::Critical => CorrectionAction::PauseAndEscalate(format!(
                "critical drift for {}, human intervention required",
                agent_name
            )),
        }
    }
}

/// Tracks API rate limits per agent per provider.
#[derive(Debug, Clone, Default)]
pub struct RateLimitTracker {
    /// Calls made per (agent_name, provider_id) in current window.
    pub calls: HashMap<(String, String), RateLimitWindow>,
}

/// Sliding window for rate limit tracking.
#[derive(Debug, Clone)]
pub struct RateLimitWindow {
    /// Calls in current hour.
    pub calls_this_hour: u32,
    /// Provider-reported limit (from HTTP headers, if available).
    pub hourly_limit: Option<u32>,
    /// Window start.
    pub window_start: chrono::DateTime<chrono::Utc>,
}

impl RateLimitTracker {
    /// Record an API call for an agent+provider pair.
    pub fn record_call(&mut self, agent_name: &str, provider_id: &str) {
        let key = (agent_name.to_string(), provider_id.to_string());
        let window = self.calls.entry(key).or_insert_with(|| RateLimitWindow {
            calls_this_hour: 0,
            hourly_limit: None,
            window_start: chrono::Utc::now(),
        });

        // Reset window if more than 1 hour has passed
        let elapsed = chrono::Utc::now() - window.window_start;
        if elapsed.num_seconds() >= 3600 {
            window.calls_this_hour = 0;
            window.window_start = chrono::Utc::now();
        }

        window.calls_this_hour += 1;
    }

    /// Check if an agent can make more calls to a provider.
    pub fn can_call(&self, agent_name: &str, provider_id: &str) -> bool {
        let key = (agent_name.to_string(), provider_id.to_string());
        match self.calls.get(&key) {
            Some(window) => match window.hourly_limit {
                Some(limit) => window.calls_this_hour < limit,
                None => true,
            },
            None => true,
        }
    }

    /// Update limit from provider response headers.
    pub fn update_limit(&mut self, agent_name: &str, provider_id: &str, limit: u32) {
        let key = (agent_name.to_string(), provider_id.to_string());
        if let Some(window) = self.calls.get_mut(&key) {
            window.hourly_limit = Some(limit);
        }
    }

    /// Get remaining calls for an agent+provider.
    pub fn remaining(&self, agent_name: &str, provider_id: &str) -> Option<u32> {
        let key = (agent_name.to_string(), provider_id.to_string());
        self.calls.get(&key).and_then(|window| {
            window
                .hourly_limit
                .map(|limit| limit.saturating_sub(window.calls_this_hour))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::capability::ProcessId;

    fn make_stdout(line: &str) -> OutputEvent {
        OutputEvent::Stdout {
            process_id: ProcessId::new(),
            line: line.to_string(),
        }
    }

    fn make_stderr(line: &str) -> OutputEvent {
        OutputEvent::Stderr {
            process_id: ProcessId::new(),
            line: line.to_string(),
        }
    }

    #[test]
    fn test_drift_metrics_zero() {
        let monitor = NightwatchMonitor::new(NightwatchConfig::default());
        assert!(monitor.drift_score("nonexistent").is_none());
    }

    #[test]
    fn test_drift_metrics_normal() {
        let mut monitor = NightwatchMonitor::new(NightwatchConfig::default());
        // All stdout, no errors
        for _ in 0..100 {
            monitor.observe("agent-a", &make_stdout("ok"));
        }
        monitor.observe_health("agent-a", HealthStatus::Healthy);

        let ds = monitor.drift_score("agent-a").unwrap();
        assert_eq!(ds.level, CorrectionLevel::Normal);
        assert!(ds.score < 0.10);
    }

    #[test]
    fn test_drift_metrics_minor() {
        let mut monitor = NightwatchMonitor::new(NightwatchConfig::default());
        // ~15% error rate -> minor drift
        for _ in 0..85 {
            monitor.observe("agent-b", &make_stdout("ok"));
        }
        for _ in 0..15 {
            monitor.observe("agent-b", &make_stderr("error"));
        }
        monitor.observe_health("agent-b", HealthStatus::Healthy);

        let ds = monitor.drift_score("agent-b").unwrap();
        // error_rate=0.15, success=0.85, health=1.0, budget=0.0
        // drift = 0.35*0.15 + 0.25*0.15 + 0.20*0.0 + 0.20*0.0 = 0.09
        // With new weights, 0.09 is just below minor threshold (0.10)
        assert_eq!(ds.level, CorrectionLevel::Normal);
    }

    #[test]
    fn test_drift_metrics_moderate() {
        let mut monitor = NightwatchMonitor::new(NightwatchConfig::default());
        // ~30% error rate -> moderate drift
        for _ in 0..70 {
            monitor.observe("agent-c", &make_stdout("ok"));
        }
        for _ in 0..30 {
            monitor.observe("agent-c", &make_stderr("error"));
        }
        monitor.observe_health("agent-c", HealthStatus::Healthy);

        let ds = monitor.drift_score("agent-c").unwrap();
        // error_rate=0.30, success=0.70, health=1.0, budget=0.0
        // drift = 0.35*0.30 + 0.25*0.30 + 0.20*0.0 + 0.20*0.0 = 0.18
        // 0.18 is now Minor (below moderate threshold of 0.20)
        assert_eq!(ds.level, CorrectionLevel::Minor);
    }

    #[test]
    fn test_drift_metrics_severe() {
        let mut monitor = NightwatchMonitor::new(NightwatchConfig::default());
        // ~60% error rate + degraded health
        for _ in 0..40 {
            monitor.observe("agent-d", &make_stdout("ok"));
        }
        for _ in 0..60 {
            monitor.observe("agent-d", &make_stderr("error"));
        }
        for _ in 0..5 {
            monitor.observe_health("agent-d", HealthStatus::Healthy);
        }
        for _ in 0..5 {
            monitor.observe_health("agent-d", HealthStatus::Degraded);
        }

        let ds = monitor.drift_score("agent-d").unwrap();
        // error_rate=0.60, success=0.40, health=0.50
        // drift = 0.4*0.60 + 0.3*0.60 + 0.3*0.50 = 0.24+0.18+0.15 = 0.57
        assert_eq!(ds.level, CorrectionLevel::Severe);
    }

    #[test]
    fn test_drift_metrics_critical() {
        let mut monitor = NightwatchMonitor::new(NightwatchConfig::default());
        // ~90% error rate + mostly unhealthy
        for _ in 0..10 {
            monitor.observe("agent-e", &make_stdout("ok"));
        }
        for _ in 0..90 {
            monitor.observe("agent-e", &make_stderr("error"));
        }
        for _ in 0..8 {
            monitor.observe_health("agent-e", HealthStatus::Unhealthy);
        }
        for _ in 0..2 {
            monitor.observe_health("agent-e", HealthStatus::Healthy);
        }

        let ds = monitor.drift_score("agent-e").unwrap();
        // error_rate=0.90, health=0.20
        // drift = 0.4*0.90 + 0.3*0.90 + 0.3*0.80 = 0.36+0.27+0.24 = 0.87
        assert_eq!(ds.level, CorrectionLevel::Critical);
    }

    #[test]
    fn test_drift_reset() {
        let mut monitor = NightwatchMonitor::new(NightwatchConfig::default());
        for _ in 0..50 {
            monitor.observe("agent-f", &make_stderr("error"));
        }
        let ds = monitor.drift_score("agent-f").unwrap();
        assert!(ds.score > 0.5);

        monitor.reset("agent-f");
        let ds = monitor.drift_score("agent-f").unwrap();
        assert!(ds.score < f64::EPSILON);
        assert_eq!(ds.metrics.sample_count, 0);
    }

    #[test]
    fn test_stderr_without_error_keywords_not_counted() {
        let mut monitor = NightwatchMonitor::new(NightwatchConfig::default());
        // Stderr lines without error keywords (like bun/codex init output)
        for _ in 0..100 {
            monitor.observe("agent-bun", &make_stderr("bun install v1.2.3"));
        }
        monitor.observe_health("agent-bun", HealthStatus::Healthy);

        let ds = monitor.drift_score("agent-bun").unwrap();
        // All 100 lines are stderr but none contain error keywords
        assert_eq!(ds.metrics.error_rate, 0.0);
        assert_eq!(ds.level, CorrectionLevel::Normal);
    }

    #[test]
    fn test_stderr_with_error_keywords_counted() {
        let mut monitor = NightwatchMonitor::new(NightwatchConfig::default());
        for _ in 0..50 {
            monitor.observe("agent-err", &make_stdout("ok"));
        }
        for _ in 0..50 {
            monitor.observe("agent-err", &make_stderr("fatal: connection refused"));
        }
        monitor.observe_health("agent-err", HealthStatus::Healthy);

        let ds = monitor.drift_score("agent-err").unwrap();
        assert_eq!(ds.metrics.error_rate, 0.5);
        assert!(ds.level >= CorrectionLevel::Moderate);
    }

    #[test]
    fn test_correction_level_ordering() {
        assert!(CorrectionLevel::Normal < CorrectionLevel::Minor);
        assert!(CorrectionLevel::Minor < CorrectionLevel::Moderate);
        assert!(CorrectionLevel::Moderate < CorrectionLevel::Severe);
        assert!(CorrectionLevel::Severe < CorrectionLevel::Critical);
    }

    #[test]
    fn test_rate_limit_tracker_basic() {
        let mut tracker = RateLimitTracker::default();

        assert!(tracker.can_call("agent-a", "openai"));
        assert!(tracker.remaining("agent-a", "openai").is_none());

        tracker.record_call("agent-a", "openai");
        tracker.update_limit("agent-a", "openai", 100);

        assert!(tracker.can_call("agent-a", "openai"));
        assert_eq!(tracker.remaining("agent-a", "openai"), Some(99));
    }

    #[test]
    fn test_rate_limit_tracker_exhausted() {
        let mut tracker = RateLimitTracker::default();

        tracker.record_call("agent-b", "anthropic");
        tracker.update_limit("agent-b", "anthropic", 2);
        tracker.record_call("agent-b", "anthropic");

        assert!(!tracker.can_call("agent-b", "anthropic"));
        assert_eq!(tracker.remaining("agent-b", "anthropic"), Some(0));
    }

    #[test]
    fn test_evaluate_emits_alerts() {
        let mut monitor = NightwatchMonitor::new(NightwatchConfig::default());

        // Create agent with high error rate
        for _ in 0..90 {
            monitor.observe("bad-agent", &make_stderr("error"));
        }
        for _ in 0..10 {
            monitor.observe("bad-agent", &make_stdout("ok"));
        }

        monitor.evaluate();

        // Should have an alert in the channel
        match monitor.alert_rx.try_recv() {
            Ok(alert) => {
                assert_eq!(alert.agent_name, "bad-agent");
                assert!(alert.drift_score.level >= CorrectionLevel::Moderate);
            }
            Err(_) => panic!("expected alert from evaluate"),
        }
    }

    // ========================================================================
    // ReasoningCertificate Tests (Gitea #92)
    // ========================================================================

    #[test]
    fn test_reasoning_certificate_valid() {
        let cert = ReasoningCertificate {
            premises: vec!["premise1".to_string(), "premise2".to_string()],
            claims: vec![Claim {
                claim: "claim1".to_string(),
                evidence: "evidence1".to_string(),
                dimension: Some("test".to_string()),
            }],
            edge_cases: vec![],
            formal_conclusion: "conclusion".to_string(),
            confidence: 0.95,
        };
        assert!(validate_certificate(&cert));
    }

    #[test]
    fn test_reasoning_certificate_insufficient_premises() {
        let cert = ReasoningCertificate {
            premises: vec!["premise1".to_string()],
            claims: vec![Claim {
                claim: "claim1".to_string(),
                evidence: "evidence1".to_string(),
                dimension: None,
            }],
            edge_cases: vec![],
            formal_conclusion: "conclusion".to_string(),
            confidence: 0.95,
        };
        assert!(!validate_certificate(&cert));
    }

    #[test]
    fn test_reasoning_certificate_no_claims() {
        let cert = ReasoningCertificate {
            premises: vec!["premise1".to_string(), "premise2".to_string()],
            claims: vec![],
            edge_cases: vec![],
            formal_conclusion: "conclusion".to_string(),
            confidence: 0.95,
        };
        assert!(!validate_certificate(&cert));
    }

    #[test]
    fn test_reasoning_certificate_empty_conclusion() {
        let cert = ReasoningCertificate {
            premises: vec!["premise1".to_string(), "premise2".to_string()],
            claims: vec![Claim {
                claim: "claim1".to_string(),
                evidence: "evidence1".to_string(),
                dimension: None,
            }],
            edge_cases: vec![],
            formal_conclusion: "".to_string(),
            confidence: 0.95,
        };
        assert!(!validate_certificate(&cert));
    }

    #[test]
    fn test_reasoning_certificate_zero_confidence() {
        let cert = ReasoningCertificate {
            premises: vec!["premise1".to_string(), "premise2".to_string()],
            claims: vec![Claim {
                claim: "claim1".to_string(),
                evidence: "evidence1".to_string(),
                dimension: None,
            }],
            edge_cases: vec![],
            formal_conclusion: "conclusion".to_string(),
            confidence: 0.0,
        };
        assert!(!validate_certificate(&cert));
    }

    #[test]
    fn test_reasoning_certificate_default_invalid() {
        let cert = ReasoningCertificate::default();
        assert!(!validate_certificate(&cert));
    }

    #[test]
    fn test_reasoning_certificate_with_edge_cases() {
        let cert = ReasoningCertificate {
            premises: vec!["premise1".to_string(), "premise2".to_string()],
            claims: vec![
                Claim {
                    claim: "claim1".to_string(),
                    evidence: "evidence1".to_string(),
                    dimension: Some("dimension1".to_string()),
                },
                Claim {
                    claim: "claim2".to_string(),
                    evidence: "evidence2".to_string(),
                    dimension: Some("dimension2".to_string()),
                },
            ],
            edge_cases: vec!["edge1".to_string(), "edge2".to_string()],
            formal_conclusion: "formal conclusion".to_string(),
            confidence: 0.85,
        };
        assert!(validate_certificate(&cert));
        assert_eq!(cert.premises.len(), 2);
        assert_eq!(cert.claims.len(), 2);
        assert_eq!(cert.edge_cases.len(), 2);
    }

    #[test]
    fn test_claim_without_dimension() {
        let claim = Claim {
            claim: "test claim".to_string(),
            evidence: "test evidence".to_string(),
            dimension: None,
        };
        assert_eq!(claim.claim, "test claim");
        assert_eq!(claim.evidence, "test evidence");
        assert!(claim.dimension.is_none());
    }

    // ========================================================================
    // Dual-Panel Evaluation Tests (Gitea #91)
    // ========================================================================

    #[test]
    fn test_dual_panel_both_agree_no_drift() {
        let output = r#"## Analysis
This is a well-structured output with evidence.

## Evidence
The data shows X because of Y.

## Conclusion
Therefore, we should proceed with Z."#;

        let cert = ReasoningCertificate {
            premises: vec!["premise1".to_string(), "premise2".to_string()],
            claims: vec![
                Claim {
                    claim: "claim1".to_string(),
                    evidence: "evidence1".to_string(),
                    dimension: Some("test".to_string()),
                },
                Claim {
                    claim: "claim2".to_string(),
                    evidence: "evidence2".to_string(),
                    dimension: Some("test2".to_string()),
                },
            ],
            edge_cases: vec!["edge1".to_string()],
            formal_conclusion: "conclusion".to_string(),
            confidence: 0.95,
        };

        let result = dual_panel_evaluate(output, Some(&cert));

        // Both panels should have high scores
        assert!(
            result.panel_a_score > 0.5,
            "Panel A should score high with valid cert"
        );
        assert!(
            result.panel_b_score > 0.5,
            "Panel B should score high with structured output"
        );
        assert!(result.agreement >= 0.5, "Panels should agree");
        assert!(
            !result.drift_detected,
            "No drift should be detected when panels agree"
        );
    }

    #[test]
    fn test_dual_panel_disagree_drift_detected() {
        // Good certificate but poor output structure
        let output = "short"; // No sections, no evidence, no conclusion

        let cert = ReasoningCertificate {
            premises: vec!["premise1".to_string(), "premise2".to_string()],
            claims: vec![Claim {
                claim: "claim1".to_string(),
                evidence: "evidence1".to_string(),
                dimension: None,
            }],
            edge_cases: vec![],
            formal_conclusion: "conclusion".to_string(),
            confidence: 0.95,
        };

        let result = dual_panel_evaluate(output, Some(&cert));

        // Panel A should score high (valid cert), Panel B should score low (poor structure)
        assert!(result.panel_a_score > 0.0, "Panel A should have some score");
        assert!(
            result.panel_b_score < 0.5,
            "Panel B should score low with unstructured output"
        );
        assert!(result.agreement < 0.5, "Panels should disagree");
        assert!(
            result.drift_detected,
            "Drift should be detected when panels disagree"
        );
    }

    #[test]
    fn test_dual_panel_missing_certificate() {
        let output = r#"## Analysis
This output has structure but no certificate.

## Evidence
Because of reasons.

## Conclusion
Therefore, success."#;

        let result = dual_panel_evaluate(output, None);

        // Panel A should be 0 without certificate
        assert_eq!(
            result.panel_a_score, 0.0,
            "Panel A should be 0 when no certificate"
        );
        // Panel B should still evaluate structure
        assert!(
            result.panel_b_score > 0.5,
            "Panel B should score high with structured output"
        );
        // Panels should disagree significantly
        assert!(
            result.drift_detected,
            "Drift should be detected when certificate is missing"
        );
    }

    #[test]
    fn test_dual_panel_both_poor_no_drift() {
        // Poor certificate and poor output (both panels agree on low quality)
        let output = "x";

        let cert = ReasoningCertificate {
            premises: vec!["only_one".to_string()], // Invalid: needs >= 2
            claims: vec![],
            edge_cases: vec![],
            formal_conclusion: "".to_string(),
            confidence: 0.0,
        };

        let result = dual_panel_evaluate(output, Some(&cert));

        // Both panels should score low
        assert_eq!(
            result.panel_a_score, 0.0,
            "Panel A should be 0 with invalid cert"
        );
        assert_eq!(
            result.panel_b_score, 0.0,
            "Panel B should be 0 with no structure"
        );
        // Perfect agreement (both 0)
        assert_eq!(
            result.agreement, 1.0,
            "Agreement should be perfect when both score 0"
        );
        assert!(
            !result.drift_detected,
            "No drift when both panels agree (even if low)"
        );
    }

    #[test]
    fn test_dual_panel_result_serialization() {
        let result = DualPanelResult {
            panel_a_score: 0.9,
            panel_b_score: 0.8,
            agreement: 0.9,
            drift_detected: false,
            details: "Test details".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("panel_a_score"));
        assert!(json.contains("0.9"));
        assert!(json.contains("drift_detected"));

        let deserialized: DualPanelResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.panel_a_score, 0.9);
        assert_eq!(deserialized.panel_b_score, 0.8);
        assert_eq!(deserialized.agreement, 0.9);
        assert!(!deserialized.drift_detected);
    }

    #[test]
    fn test_calculate_certificate_score_components() {
        // Minimal valid certificate (base score only)
        let minimal = ReasoningCertificate {
            premises: vec!["p1".to_string(), "p2".to_string()],
            claims: vec![Claim {
                claim: "c1".to_string(),
                evidence: "e1".to_string(),
                dimension: None,
            }],
            edge_cases: vec![],
            formal_conclusion: "conclusion".to_string(),
            confidence: 0.5,
        };
        assert_eq!(calculate_certificate_score(&minimal), 0.5);

        // Certificate with all bonuses
        let full = ReasoningCertificate {
            premises: vec!["p1".to_string(), "p2".to_string(), "p3".to_string()],
            claims: vec![
                Claim {
                    claim: "c1".to_string(),
                    evidence: "e1".to_string(),
                    dimension: None,
                },
                Claim {
                    claim: "c2".to_string(),
                    evidence: "e2".to_string(),
                    dimension: None,
                },
            ],
            edge_cases: vec!["edge".to_string()],
            formal_conclusion: "conclusion".to_string(),
            confidence: 0.95,
        };
        assert_eq!(calculate_certificate_score(&full), 1.0);
    }

    #[test]
    fn test_calculate_structure_score_components() {
        // Empty output
        assert_eq!(calculate_structure_score(""), 0.0);

        // Just length
        assert_eq!(calculate_structure_score("x".repeat(100).as_str()), 0.1);

        // With sections
        assert!(calculate_structure_score("## Section") >= 0.3);

        // With evidence marker
        assert!(calculate_structure_score("evidence: because") >= 0.3);

        // With conclusion marker
        assert!(calculate_structure_score("conclusion: therefore") >= 0.3);

        // Full structure (with enough length to get the bonus)
        let full = "## Analysis\n\nevidence: X is supported by the data\n\nconclusion: Therefore we should proceed with Y. This is the final conclusion of this analysis.";
        assert!((calculate_structure_score(full) - 1.0).abs() < f64::EPSILON);
    }
}
