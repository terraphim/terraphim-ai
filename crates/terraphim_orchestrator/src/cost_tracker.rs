use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

const WARNING_THRESHOLD: f64 = 0.80;
const SUB_CENTS_PER_USD: u64 = 10_000; // hundredths-of-a-cent precision

/// Result of a budget check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetVerdict {
    /// Agent has no budget cap (subscription model).
    Uncapped,
    /// Spend is within normal budget range.
    WithinBudget,
    /// Spend has reached warning threshold (80%).
    NearExhaustion { spent_cents: u64, budget_cents: u64 },
    /// Spend has reached or exceeded 100% of budget.
    Exhausted { spent_cents: u64, budget_cents: u64 },
}

impl BudgetVerdict {
    /// Returns true if the agent should be paused (budget exhausted).
    pub fn should_pause(&self) -> bool {
        matches!(self, BudgetVerdict::Exhausted { .. })
    }

    /// Returns true if a warning should be issued (near exhaustion).
    pub fn should_warn(&self) -> bool {
        matches!(self, BudgetVerdict::NearExhaustion { .. })
    }
}

impl fmt::Display for BudgetVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BudgetVerdict::Uncapped => write!(f, "uncapped"),
            BudgetVerdict::WithinBudget => write!(f, "within budget"),
            BudgetVerdict::NearExhaustion {
                spent_cents,
                budget_cents,
            } => write!(
                f,
                "near exhaustion ({} / {} cents)",
                spent_cents, budget_cents
            ),
            BudgetVerdict::Exhausted {
                spent_cents,
                budget_cents,
            } => write!(f, "exhausted ({} / {} cents)", spent_cents, budget_cents),
        }
    }
}

/// Per-execution metrics for an agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    /// Timestamp when the execution started.
    pub started_at: DateTime<Utc>,
    /// Timestamp when the execution completed.
    pub completed_at: DateTime<Utc>,
    /// Input token count (prompt tokens).
    pub input_tokens: u64,
    /// Output token count (completion tokens).
    pub output_tokens: u64,
    /// Total token count (input + output).
    pub total_tokens: u64,
    /// Latency in milliseconds.
    pub latency_ms: u64,
    /// Estimated cost in USD for this execution.
    pub estimated_cost_usd: f64,
    /// Whether the execution succeeded.
    pub success: bool,
    /// Optional error message if execution failed.
    pub error_message: Option<String>,
    /// Model used for this execution.
    pub model: Option<String>,
    /// Provider used for this execution.
    pub provider: Option<String>,
}

impl ExecutionMetrics {
    /// Create a new execution metrics record.
    pub fn new(started_at: DateTime<Utc>) -> Self {
        Self {
            started_at,
            completed_at: started_at,
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            latency_ms: 0,
            estimated_cost_usd: 0.0,
            success: true,
            error_message: None,
            model: None,
            provider: None,
        }
    }

    /// Mark the execution as completed with metrics.
    pub fn complete(
        mut self,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
        success: bool,
    ) -> Self {
        self.completed_at = Utc::now();
        self.input_tokens = input_tokens;
        self.output_tokens = output_tokens;
        self.total_tokens = input_tokens + output_tokens;
        self.latency_ms = (self.completed_at - self.started_at).num_milliseconds() as u64;
        self.estimated_cost_usd = cost_usd;
        self.success = success;
        self
    }

    /// Mark execution as failed with error message.
    pub fn fail(mut self, error: String) -> Self {
        self.completed_at = Utc::now();
        self.success = false;
        self.error_message = Some(error);
        self.latency_ms = (self.completed_at - self.started_at).num_milliseconds() as u64;
        self
    }

    /// Set model and provider information.
    pub fn with_model(mut self, model: String, provider: String) -> Self {
        self.model = Some(model);
        self.provider = Some(provider);
        self
    }
}

/// Aggregated metrics for an agent over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    /// Agent name.
    pub agent_name: String,
    /// Total number of executions.
    pub total_executions: u64,
    /// Number of successful executions.
    pub successful_executions: u64,
    /// Number of failed executions.
    pub failed_executions: u64,
    /// Total input tokens across all executions.
    pub total_input_tokens: u64,
    /// Total output tokens across all executions.
    pub total_output_tokens: u64,
    /// Total tokens across all executions.
    pub total_tokens: u64,
    /// Total latency in milliseconds across all executions.
    pub total_latency_ms: u64,
    /// Total estimated cost in USD across all executions.
    pub total_cost_usd: f64,
    /// Average tokens per execution.
    pub avg_tokens_per_execution: f64,
    /// Average latency per execution in milliseconds.
    pub avg_latency_ms: f64,
    /// Average cost per execution in USD.
    pub avg_cost_usd: f64,
    /// Success rate (0.0 - 1.0).
    pub success_rate: f64,
    /// Timestamp of first execution.
    pub first_execution_at: Option<DateTime<Utc>>,
    /// Timestamp of most recent execution.
    pub last_execution_at: Option<DateTime<Utc>>,
    /// Recent execution history (last 100 executions).
    pub recent_executions: Vec<ExecutionMetrics>,
}

impl AgentMetrics {
    /// Create new agent metrics for the given agent.
    pub fn new(agent_name: String) -> Self {
        Self {
            agent_name,
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_tokens: 0,
            total_latency_ms: 0,
            total_cost_usd: 0.0,
            avg_tokens_per_execution: 0.0,
            avg_latency_ms: 0.0,
            avg_cost_usd: 0.0,
            success_rate: 0.0,
            first_execution_at: None,
            last_execution_at: None,
            recent_executions: Vec::with_capacity(100),
        }
    }

    /// Record a new execution and update aggregated metrics.
    pub fn record_execution(&mut self, execution: ExecutionMetrics) {
        self.total_executions += 1;

        if execution.success {
            self.successful_executions += 1;
            self.total_input_tokens += execution.input_tokens;
            self.total_output_tokens += execution.output_tokens;
            self.total_tokens += execution.total_tokens;
            self.total_cost_usd += execution.estimated_cost_usd;
        } else {
            self.failed_executions += 1;
        }

        self.total_latency_ms += execution.latency_ms;

        // Update timestamps
        if self.first_execution_at.is_none() {
            self.first_execution_at = Some(execution.started_at);
        }
        self.last_execution_at = Some(execution.completed_at);

        // Update averages
        self.avg_tokens_per_execution = self.total_tokens as f64 / self.total_executions as f64;
        self.avg_latency_ms = self.total_latency_ms as f64 / self.total_executions as f64;
        self.avg_cost_usd = self.total_cost_usd / self.total_executions as f64;
        self.success_rate = self.successful_executions as f64 / self.total_executions as f64;

        // Add to recent executions, keeping only last 100
        self.recent_executions.push(execution);
        if self.recent_executions.len() > 100 {
            self.recent_executions.remove(0);
        }
    }

    /// Get cost efficiency metric (tokens per dollar).
    pub fn tokens_per_dollar(&self) -> f64 {
        if self.total_cost_usd > 0.0 {
            self.total_tokens as f64 / self.total_cost_usd
        } else {
            0.0
        }
    }
}

/// Internal cost tracking for a single agent.
struct AgentCost {
    /// Spend in hundredths-of-a-cent (1 USD = 10_000 sub-cents).
    spend_sub_cents: AtomicU64,
    /// Monthly budget in cents (None = unlimited).
    budget_cents: Option<u64>,
    /// Month number (1-12) when this agent's budget resets.
    reset_month: u8,
    /// Year when this agent's budget resets.
    reset_year: i32,
}

impl AgentCost {
    fn new(budget_cents: Option<u64>) -> Self {
        let now = Utc::now();
        Self {
            spend_sub_cents: AtomicU64::new(0),
            budget_cents,
            reset_month: now.month() as u8,
            reset_year: now.year(),
        }
    }

    /// Record a cost in USD and return the current budget verdict.
    fn record_cost(&self, cost_usd: f64) -> BudgetVerdict {
        let sub_cents = (cost_usd * SUB_CENTS_PER_USD as f64).round() as u64;
        self.spend_sub_cents.fetch_add(sub_cents, Ordering::Relaxed);
        self.check()
    }

    /// Check current budget status without recording new spend.
    fn check(&self) -> BudgetVerdict {
        let budget_cents = match self.budget_cents {
            Some(b) => b,
            None => return BudgetVerdict::Uncapped,
        };

        let spent_sub_cents = self.spend_sub_cents.load(Ordering::Relaxed);
        let spent_cents = spent_sub_cents / 100; // Convert sub-cents to cents

        if spent_cents >= budget_cents {
            BudgetVerdict::Exhausted {
                spent_cents,
                budget_cents,
            }
        } else if spent_cents as f64 >= budget_cents as f64 * WARNING_THRESHOLD {
            BudgetVerdict::NearExhaustion {
                spent_cents,
                budget_cents,
            }
        } else {
            BudgetVerdict::WithinBudget
        }
    }

    /// Reset spend if we've rolled into a new month.
    fn reset_if_due(&mut self) {
        let now = Utc::now();
        let current_month = now.month() as u8;
        let current_year = now.year();

        if current_month != self.reset_month || current_year != self.reset_year {
            self.spend_sub_cents.store(0, Ordering::Relaxed);
            self.reset_month = current_month;
            self.reset_year = current_year;
        }
    }

    /// Get total spend in USD.
    fn spent_usd(&self) -> f64 {
        let sub_cents = self.spend_sub_cents.load(Ordering::Relaxed);
        sub_cents as f64 / SUB_CENTS_PER_USD as f64
    }
}

/// Snapshot of an agent's cost status (for serialization).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSnapshot {
    pub agent_name: String,
    pub spent_usd: f64,
    pub budget_cents: Option<u64>,
    pub verdict: String,
}

/// Tracks per-agent spend with budget enforcement and detailed metrics.
pub struct CostTracker {
    agents: HashMap<String, AgentCost>,
    /// Per-agent detailed metrics.
    metrics: HashMap<String, AgentMetrics>,
}

impl CostTracker {
    /// Create a new empty CostTracker.
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            metrics: HashMap::new(),
        }
    }

    /// Register an agent with its monthly budget.
    /// None budget means uncapped (subscription model).
    pub fn register(&mut self, agent_name: &str, budget_monthly_cents: Option<u64>) {
        self.agents
            .insert(agent_name.to_string(), AgentCost::new(budget_monthly_cents));
        self.metrics.insert(
            agent_name.to_string(),
            AgentMetrics::new(agent_name.to_string()),
        );
    }

    /// Record a cost for an agent and return the budget verdict.
    /// Returns Uncapped for unregistered agents.
    pub fn record_cost(&self, agent_name: &str, cost_usd: f64) -> BudgetVerdict {
        match self.agents.get(agent_name) {
            Some(agent_cost) => agent_cost.record_cost(cost_usd),
            None => BudgetVerdict::Uncapped,
        }
    }

    /// Record execution metrics for an agent.
    pub fn record_execution(&mut self, agent_name: &str, execution: ExecutionMetrics) {
        // Record the cost
        let _ = self.record_cost(agent_name, execution.estimated_cost_usd);

        // Record the detailed metrics
        if let Some(metrics) = self.metrics.get_mut(agent_name) {
            metrics.record_execution(execution);
        }
    }

    /// Get metrics for a specific agent.
    pub fn get_metrics(&self, agent_name: &str) -> Option<&AgentMetrics> {
        self.metrics.get(agent_name)
    }

    /// Get mutable metrics for a specific agent.
    pub fn get_metrics_mut(&mut self, agent_name: &str) -> Option<&mut AgentMetrics> {
        self.metrics.get_mut(agent_name)
    }

    /// Get all agent metrics.
    pub fn all_metrics(&self) -> &HashMap<String, AgentMetrics> {
        &self.metrics
    }

    /// Check budget status for a specific agent.
    /// Returns Uncapped for unregistered agents.
    pub fn check(&self, agent_name: &str) -> BudgetVerdict {
        match self.agents.get(agent_name) {
            Some(agent_cost) => agent_cost.check(),
            None => BudgetVerdict::Uncapped,
        }
    }

    /// Check budget status for all registered agents.
    /// Returns only actionable verdicts (NearExhaustion or Exhausted).
    pub fn check_all(&self) -> Vec<(String, BudgetVerdict)> {
        self.agents
            .iter()
            .filter_map(|(name, agent_cost)| {
                let verdict = agent_cost.check();
                match verdict {
                    BudgetVerdict::NearExhaustion { .. } | BudgetVerdict::Exhausted { .. } => {
                        Some((name.clone(), verdict))
                    }
                    _ => None,
                }
            })
            .collect()
    }

    /// Reset budgets for all agents if we've entered a new month.
    pub fn monthly_reset_if_due(&mut self) {
        for agent_cost in self.agents.values_mut() {
            agent_cost.reset_if_due();
        }
    }

    /// Get snapshots of all registered agents.
    pub fn snapshots(&self) -> Vec<CostSnapshot> {
        self.agents
            .iter()
            .map(|(name, agent_cost)| {
                let verdict = agent_cost.check();
                CostSnapshot {
                    agent_name: name.clone(),
                    spent_usd: agent_cost.spent_usd(),
                    budget_cents: agent_cost.budget_cents,
                    verdict: format!("{}", verdict),
                }
            })
            .collect()
    }

    /// Get total fleet spend across all agents in USD.
    pub fn total_fleet_spend_usd(&self) -> f64 {
        self.agents
            .values()
            .map(|agent_cost| agent_cost.spent_usd())
            .sum()
    }

    /// Get total fleet metrics.
    pub fn fleet_metrics(&self) -> AgentMetrics {
        let mut fleet = AgentMetrics::new("fleet".to_string());

        for metrics in self.metrics.values() {
            fleet.total_executions += metrics.total_executions;
            fleet.successful_executions += metrics.successful_executions;
            fleet.failed_executions += metrics.failed_executions;
            fleet.total_input_tokens += metrics.total_input_tokens;
            fleet.total_output_tokens += metrics.total_output_tokens;
            fleet.total_tokens += metrics.total_tokens;
            fleet.total_latency_ms += metrics.total_latency_ms;
            fleet.total_cost_usd += metrics.total_cost_usd;
        }

        if fleet.total_executions > 0 {
            fleet.avg_tokens_per_execution =
                fleet.total_tokens as f64 / fleet.total_executions as f64;
            fleet.avg_latency_ms = fleet.total_latency_ms as f64 / fleet.total_executions as f64;
            fleet.avg_cost_usd = fleet.total_cost_usd / fleet.total_executions as f64;
            fleet.success_rate = fleet.successful_executions as f64 / fleet.total_executions as f64;
        }

        fleet
    }
}

impl Default for CostTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uncapped_agent_always_allowed() {
        let mut tracker = CostTracker::new();
        tracker.register("test-agent", None);

        let verdict = tracker.record_cost("test-agent", 100.0);
        assert_eq!(verdict, BudgetVerdict::Uncapped);

        // Even with more spend, still uncapped
        let verdict = tracker.record_cost("test-agent", 1000.0);
        assert_eq!(verdict, BudgetVerdict::Uncapped);
    }

    #[test]
    fn test_within_budget() {
        let mut tracker = CostTracker::new();
        tracker.register("test-agent", Some(5000)); // $50.00 budget

        // Spend $20.00 = 2000 cents, which is 40% of budget
        let verdict = tracker.record_cost("test-agent", 20.0);
        assert_eq!(verdict, BudgetVerdict::WithinBudget);
    }

    #[test]
    fn test_near_exhaustion_at_80_pct() {
        let mut tracker = CostTracker::new();
        tracker.register("test-agent", Some(10000)); // $100.00 budget

        // Spend $81.00 = 8100 cents, which is 81% of budget
        let verdict = tracker.record_cost("test-agent", 81.0);
        assert!(
            matches!(
                verdict,
                BudgetVerdict::NearExhaustion {
                    spent_cents: 8100,
                    budget_cents: 10000
                }
            ),
            "Expected NearExhaustion at 81%, got {:?}",
            verdict
        );
    }

    #[test]
    fn test_exhausted_at_100_pct() {
        let mut tracker = CostTracker::new();
        tracker.register("test-agent", Some(5000)); // $50.00 budget

        // Spend $51.00 = 5100 cents, which exceeds 100% of budget
        let verdict = tracker.record_cost("test-agent", 51.0);
        assert!(
            matches!(
                verdict,
                BudgetVerdict::Exhausted {
                    spent_cents: 5100,
                    budget_cents: 5000
                }
            ),
            "Expected Exhausted at >100%, got {:?}",
            verdict
        );
    }

    #[test]
    fn test_should_pause_only_on_exhausted() {
        assert!(
            BudgetVerdict::Exhausted {
                spent_cents: 100,
                budget_cents: 100
            }
            .should_pause()
        );

        assert!(
            !BudgetVerdict::NearExhaustion {
                spent_cents: 80,
                budget_cents: 100
            }
            .should_pause()
        );

        assert!(!BudgetVerdict::WithinBudget.should_pause());
        assert!(!BudgetVerdict::Uncapped.should_pause());
    }

    #[test]
    fn test_should_warn_only_on_near_exhaustion() {
        assert!(
            BudgetVerdict::NearExhaustion {
                spent_cents: 80,
                budget_cents: 100
            }
            .should_warn()
        );

        assert!(
            !BudgetVerdict::Exhausted {
                spent_cents: 100,
                budget_cents: 100
            }
            .should_warn()
        );

        assert!(!BudgetVerdict::WithinBudget.should_warn());
        assert!(!BudgetVerdict::Uncapped.should_warn());
    }

    #[test]
    fn test_check_all_returns_only_actionable() {
        let mut tracker = CostTracker::new();
        tracker.register("uncapped-agent", None);
        tracker.register("within-budget", Some(10000));
        tracker.register("near-limit", Some(10000));
        tracker.register("exhausted", Some(10000));

        // Spend to trigger different states
        tracker.record_cost("within-budget", 50.0); // 50%
        tracker.record_cost("near-limit", 85.0); // 85%
        tracker.record_cost("exhausted", 100.0); // 100%

        let actionable = tracker.check_all();
        assert_eq!(actionable.len(), 2);

        // Verify the right agents are returned
        let names: Vec<_> = actionable.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"near-limit"));
        assert!(names.contains(&"exhausted"));
        assert!(!names.contains(&"uncapped-agent"));
        assert!(!names.contains(&"within-budget"));
    }

    #[test]
    fn test_monthly_reset() {
        let mut tracker = CostTracker::new();
        tracker.register("test-agent", Some(10000));

        // Spend some amount
        tracker.record_cost("test-agent", 50.0);
        assert_eq!(tracker.check("test-agent"), BudgetVerdict::WithinBudget);

        // Simulate a reset by manually manipulating the reset date
        // In a real scenario, we'd need to mock time
        if let Some(agent) = tracker.agents.get_mut("test-agent") {
            // Set reset month to previous month to force reset
            let now = Utc::now();
            if now.month() == 1 {
                agent.reset_month = 12;
                agent.reset_year = now.year() - 1;
            } else {
                agent.reset_month = (now.month() - 1) as u8;
                agent.reset_year = now.year();
            }
        }

        // Now the reset should occur
        tracker.monthly_reset_if_due();

        // After reset, should be back to within budget (spend cleared)
        assert_eq!(tracker.check("test-agent"), BudgetVerdict::WithinBudget);
        assert_eq!(tracker.total_fleet_spend_usd(), 0.0);
    }

    #[test]
    fn test_record_cost_returns_verdict() {
        let mut tracker = CostTracker::new();
        tracker.register("test-agent", Some(10000));

        let verdict = tracker.record_cost("test-agent", 85.0);
        assert!(
            matches!(verdict, BudgetVerdict::NearExhaustion { .. }),
            "Expected NearExhaustion, got {:?}",
            verdict
        );
    }

    #[test]
    fn test_unregistered_agent_treated_as_uncapped() {
        let tracker = CostTracker::new();
        // Don't register the agent

        let verdict = tracker.record_cost("unknown-agent", 1000.0);
        assert_eq!(verdict, BudgetVerdict::Uncapped);

        let check_result = tracker.check("unknown-agent");
        assert_eq!(check_result, BudgetVerdict::Uncapped);
    }

    #[test]
    fn test_total_fleet_spend() {
        let mut tracker = CostTracker::new();
        tracker.register("agent-1", Some(10000));
        tracker.register("agent-2", Some(10000));
        tracker.register("agent-3", None);

        tracker.record_cost("agent-1", 10.0);
        tracker.record_cost("agent-2", 20.0);
        tracker.record_cost("agent-3", 30.0);

        assert_eq!(tracker.total_fleet_spend_usd(), 60.0);
    }

    #[test]
    fn test_snapshots() {
        let mut tracker = CostTracker::new();
        tracker.register("agent-1", Some(10000));
        tracker.register("agent-2", None);

        tracker.record_cost("agent-1", 85.0); // NearExhaustion
        tracker.record_cost("agent-2", 100.0); // Uncapped

        let snapshots = tracker.snapshots();
        assert_eq!(snapshots.len(), 2);

        let snapshot_1 = snapshots
            .iter()
            .find(|s| s.agent_name == "agent-1")
            .unwrap();
        assert_eq!(snapshot_1.spent_usd, 85.0);
        assert_eq!(snapshot_1.budget_cents, Some(10000));
        assert!(snapshot_1.verdict.contains("near exhaustion"));

        let snapshot_2 = snapshots
            .iter()
            .find(|s| s.agent_name == "agent-2")
            .unwrap();
        assert_eq!(snapshot_2.spent_usd, 100.0);
        assert_eq!(snapshot_2.budget_cents, None);
        assert!(snapshot_2.verdict.contains("uncapped"));
    }

    #[test]
    fn test_sub_cent_precision() {
        let mut tracker = CostTracker::new();
        tracker.register("test-agent", Some(20000)); // $200.00 budget

        // Spend $0.0001 x 10000 = $1.00
        for _ in 0..10000 {
            tracker.record_cost("test-agent", 0.0001);
        }

        let snapshot = tracker
            .snapshots()
            .into_iter()
            .find(|s| s.agent_name == "test-agent")
            .unwrap();
        assert!(
            (snapshot.spent_usd - 1.0).abs() < 0.001,
            "Expected ~$1.00, got ${}",
            snapshot.spent_usd
        );
    }

    #[test]
    fn test_execution_metrics_recording() {
        let mut tracker = CostTracker::new();
        tracker.register("test-agent", Some(10000));

        let execution = ExecutionMetrics::new(Utc::now())
            .complete(100, 50, 0.005, true)
            .with_model("gpt-4".to_string(), "openai".to_string());

        tracker.record_execution("test-agent", execution);

        let metrics = tracker.get_metrics("test-agent").unwrap();
        assert_eq!(metrics.total_executions, 1);
        assert_eq!(metrics.successful_executions, 1);
        assert_eq!(metrics.total_input_tokens, 100);
        assert_eq!(metrics.total_output_tokens, 50);
        assert_eq!(metrics.total_tokens, 150);
        assert!((metrics.total_cost_usd - 0.005).abs() < 0.0001);
        assert_eq!(metrics.success_rate, 1.0);
        assert_eq!(metrics.recent_executions.len(), 1);
    }

    #[test]
    fn test_agent_metrics_aggregation() {
        let mut tracker = CostTracker::new();
        tracker.register("test-agent", Some(10000));

        // Record multiple executions
        for i in 0..5 {
            let execution = ExecutionMetrics::new(Utc::now()).complete(
                100 + i as u64 * 10,
                50 + i as u64 * 5,
                0.005,
                true,
            );
            tracker.record_execution("test-agent", execution);
        }

        let metrics = tracker.get_metrics("test-agent").unwrap();
        assert_eq!(metrics.total_executions, 5);
        assert_eq!(metrics.successful_executions, 5);
        assert_eq!(metrics.total_input_tokens, 100 + 110 + 120 + 130 + 140);
        assert_eq!(metrics.total_output_tokens, 50 + 55 + 60 + 65 + 70);
        assert!(metrics.avg_tokens_per_execution > 0.0);
        assert!(metrics.avg_cost_usd > 0.0);
        assert_eq!(metrics.success_rate, 1.0);
    }

    #[test]
    fn test_failed_execution_recording() {
        let mut tracker = CostTracker::new();
        tracker.register("test-agent", Some(10000));

        let execution = ExecutionMetrics::new(Utc::now()).fail("API timeout".to_string());

        tracker.record_execution("test-agent", execution);

        let metrics = tracker.get_metrics("test-agent").unwrap();
        assert_eq!(metrics.total_executions, 1);
        assert_eq!(metrics.successful_executions, 0);
        assert_eq!(metrics.failed_executions, 1);
        assert_eq!(metrics.success_rate, 0.0);
    }

    #[test]
    fn test_fleet_metrics() {
        let mut tracker = CostTracker::new();
        tracker.register("agent-1", Some(10000));
        tracker.register("agent-2", Some(10000));

        let execution1 = ExecutionMetrics::new(Utc::now()).complete(100, 50, 0.01, true);
        let execution2 = ExecutionMetrics::new(Utc::now()).complete(200, 100, 0.02, true);

        tracker.record_execution("agent-1", execution1);
        tracker.record_execution("agent-2", execution2);

        let fleet = tracker.fleet_metrics();
        assert_eq!(fleet.total_executions, 2);
        assert_eq!(fleet.total_input_tokens, 300);
        assert_eq!(fleet.total_output_tokens, 150);
        assert!((fleet.total_cost_usd - 0.03).abs() < 0.001);
    }

    #[test]
    fn test_tokens_per_dollar() {
        let mut metrics = AgentMetrics::new("test".to_string());
        metrics.total_tokens = 1000;
        metrics.total_cost_usd = 0.01;

        assert_eq!(metrics.tokens_per_dollar(), 100000.0);

        // Test zero cost case
        metrics.total_cost_usd = 0.0;
        assert_eq!(metrics.tokens_per_dollar(), 0.0);
    }
}
