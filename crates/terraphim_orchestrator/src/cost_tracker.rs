use chrono::{Datelike, Utc};
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

/// Tracks per-agent spend with budget enforcement.
pub struct CostTracker {
    agents: HashMap<String, AgentCost>,
}

impl CostTracker {
    /// Create a new empty CostTracker.
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    /// Register an agent with its monthly budget.
    /// None budget means uncapped (subscription model).
    pub fn register(&mut self, agent_name: &str, budget_monthly_cents: Option<u64>) {
        self.agents
            .insert(agent_name.to_string(), AgentCost::new(budget_monthly_cents));
    }

    /// Record a cost for an agent and return the budget verdict.
    /// Returns Uncapped for unregistered agents.
    pub fn record_cost(&self, agent_name: &str, cost_usd: f64) -> BudgetVerdict {
        match self.agents.get(agent_name) {
            Some(agent_cost) => agent_cost.record_cost(cost_usd),
            None => BudgetVerdict::Uncapped,
        }
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
        assert!(BudgetVerdict::Exhausted {
            spent_cents: 100,
            budget_cents: 100
        }
        .should_pause());

        assert!(!BudgetVerdict::NearExhaustion {
            spent_cents: 80,
            budget_cents: 100
        }
        .should_pause());

        assert!(!BudgetVerdict::WithinBudget.should_pause());
        assert!(!BudgetVerdict::Uncapped.should_pause());
    }

    #[test]
    fn test_should_warn_only_on_near_exhaustion() {
        assert!(BudgetVerdict::NearExhaustion {
            spent_cents: 80,
            budget_cents: 100
        }
        .should_warn());

        assert!(!BudgetVerdict::Exhausted {
            spent_cents: 100,
            budget_cents: 100
        }
        .should_warn());

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
}
