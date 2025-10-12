//! Cost tracking for TruthForge LLM agents
//!
//! Tracks token usage and costs per agent, session, and overall analysis.
//! Provides budget limits and cost breakdowns for transparency.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{Result, TruthForgeError};

/// Pricing for OpenRouter Claude models (as of October 2024)
#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub model: String,
    pub input_cost_per_million: f64,
    pub output_cost_per_million: f64,
}

impl ModelPricing {
    pub fn claude_sonnet() -> Self {
        Self {
            model: "anthropic/claude-3.5-sonnet".to_string(),
            input_cost_per_million: 3.0,
            output_cost_per_million: 15.0,
        }
    }

    pub fn claude_haiku() -> Self {
        Self {
            model: "anthropic/claude-3.5-haiku".to_string(),
            input_cost_per_million: 0.25,
            output_cost_per_million: 1.25,
        }
    }

    pub fn calculate_cost(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.input_cost_per_million;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output_cost_per_million;
        input_cost + output_cost
    }
}

/// Token usage and cost for a single agent call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCost {
    pub agent_name: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub cost_usd: f64,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}

impl AgentCost {
    pub fn new(
        agent_name: String,
        model: String,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
        duration_ms: u64,
    ) -> Self {
        Self {
            agent_name,
            model,
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            cost_usd,
            duration_ms,
            timestamp: Utc::now(),
        }
    }
}

/// Cost breakdown by workflow stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageCosts {
    pub stage_name: String,
    pub agent_costs: Vec<AgentCost>,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub duration_ms: u64,
}

impl StageCosts {
    pub fn new(stage_name: String) -> Self {
        Self {
            stage_name,
            agent_costs: Vec::new(),
            total_tokens: 0,
            total_cost_usd: 0.0,
            duration_ms: 0,
        }
    }

    pub fn add_agent_cost(&mut self, cost: AgentCost) {
        self.total_tokens += cost.total_tokens;
        self.total_cost_usd += cost.cost_usd;
        self.duration_ms += cost.duration_ms;
        self.agent_costs.push(cost);
    }
}

/// Complete cost tracking for a TruthForge analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisCostTracker {
    pub session_id: Uuid,
    pub pass_one_costs: StageCosts,
    pub pass_one_debate_costs: StageCosts,
    pub pass_two_costs: StageCosts,
    pub response_generator_costs: StageCosts,
    pub total_cost_usd: f64,
    pub total_tokens: u64,
    pub total_duration_ms: u64,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl AnalysisCostTracker {
    pub fn new(session_id: Uuid) -> Self {
        Self {
            session_id,
            pass_one_costs: StageCosts::new("Pass One Analysis".to_string()),
            pass_one_debate_costs: StageCosts::new("Pass One Debate".to_string()),
            pass_two_costs: StageCosts::new("Pass Two Exploitation".to_string()),
            response_generator_costs: StageCosts::new("Response Generation".to_string()),
            total_cost_usd: 0.0,
            total_tokens: 0,
            total_duration_ms: 0,
            started_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn add_pass_one_cost(&mut self, cost: AgentCost) {
        self.pass_one_costs.add_agent_cost(cost);
        self.recalculate_totals();
    }

    pub fn add_pass_one_debate_cost(&mut self, cost: AgentCost) {
        self.pass_one_debate_costs.add_agent_cost(cost);
        self.recalculate_totals();
    }

    pub fn add_pass_two_cost(&mut self, cost: AgentCost) {
        self.pass_two_costs.add_agent_cost(cost);
        self.recalculate_totals();
    }

    pub fn add_response_generator_cost(&mut self, cost: AgentCost) {
        self.response_generator_costs.add_agent_cost(cost);
        self.recalculate_totals();
    }

    fn recalculate_totals(&mut self) {
        self.total_cost_usd = self.pass_one_costs.total_cost_usd
            + self.pass_one_debate_costs.total_cost_usd
            + self.pass_two_costs.total_cost_usd
            + self.response_generator_costs.total_cost_usd;

        self.total_tokens = self.pass_one_costs.total_tokens
            + self.pass_one_debate_costs.total_tokens
            + self.pass_two_costs.total_tokens
            + self.response_generator_costs.total_tokens;

        self.total_duration_ms = self.pass_one_costs.duration_ms
            + self.pass_one_debate_costs.duration_ms
            + self.pass_two_costs.duration_ms
            + self.response_generator_costs.duration_ms;
    }

    pub fn complete(&mut self) {
        self.completed_at = Some(Utc::now());
    }

    pub fn check_budget(&self, budget_usd: f64) -> Result<()> {
        if self.total_cost_usd > budget_usd {
            return Err(TruthForgeError::ConfigError(format!(
                "Budget exceeded: ${:.4} > ${:.4}",
                self.total_cost_usd, budget_usd
            )));
        }
        Ok(())
    }

    pub fn cost_breakdown(&self) -> HashMap<String, f64> {
        let mut breakdown = HashMap::new();
        breakdown.insert("pass_one".to_string(), self.pass_one_costs.total_cost_usd);
        breakdown.insert(
            "pass_one_debate".to_string(),
            self.pass_one_debate_costs.total_cost_usd,
        );
        breakdown.insert("pass_two".to_string(), self.pass_two_costs.total_cost_usd);
        breakdown.insert(
            "response_generator".to_string(),
            self.response_generator_costs.total_cost_usd,
        );
        breakdown.insert("total".to_string(), self.total_cost_usd);
        breakdown
    }

    pub fn format_summary(&self) -> String {
        format!(
            "TruthForge Analysis Cost Summary (Session: {})\n\
            ==========================================\n\
            Pass One Analysis:      ${:.4} ({} tokens, {}ms)\n\
            Pass One Debate:        ${:.4} ({} tokens, {}ms)\n\
            Pass Two Exploitation:  ${:.4} ({} tokens, {}ms)\n\
            Response Generation:    ${:.4} ({} tokens, {}ms)\n\
            ------------------------------------------\n\
            TOTAL:                  ${:.4} ({} tokens, {}ms)\n\
            \n\
            Started:  {}\n\
            Completed: {}",
            self.session_id,
            self.pass_one_costs.total_cost_usd,
            self.pass_one_costs.total_tokens,
            self.pass_one_costs.duration_ms,
            self.pass_one_debate_costs.total_cost_usd,
            self.pass_one_debate_costs.total_tokens,
            self.pass_one_debate_costs.duration_ms,
            self.pass_two_costs.total_cost_usd,
            self.pass_two_costs.total_tokens,
            self.pass_two_costs.duration_ms,
            self.response_generator_costs.total_cost_usd,
            self.response_generator_costs.total_tokens,
            self.response_generator_costs.duration_ms,
            self.total_cost_usd,
            self.total_tokens,
            self.total_duration_ms,
            self.started_at.format("%Y-%m-%d %H:%M:%S UTC"),
            self.completed_at
                .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| "In Progress".to_string())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_pricing_sonnet() {
        let pricing = ModelPricing::claude_sonnet();
        let cost = pricing.calculate_cost(1_000_000, 1_000_000);
        assert!((cost - 18.0).abs() < 0.01);
    }

    #[test]
    fn test_model_pricing_haiku() {
        let pricing = ModelPricing::claude_haiku();
        let cost = pricing.calculate_cost(1_000_000, 1_000_000);
        assert!((cost - 1.5).abs() < 0.01);
    }

    #[test]
    fn test_analysis_cost_tracker() {
        let session_id = Uuid::new_v4();
        let mut tracker = AnalysisCostTracker::new(session_id);

        let cost1 = AgentCost::new(
            "OmissionDetector".to_string(),
            "anthropic/claude-3.5-sonnet".to_string(),
            1000,
            500,
            0.015,
            1500,
        );
        tracker.add_pass_one_cost(cost1);

        assert_eq!(tracker.total_tokens, 1500);
        assert!((tracker.total_cost_usd - 0.015).abs() < 0.001);
        assert_eq!(tracker.total_duration_ms, 1500);
    }

    #[test]
    fn test_budget_check() {
        let session_id = Uuid::new_v4();
        let mut tracker = AnalysisCostTracker::new(session_id);

        let cost = AgentCost::new(
            "TestAgent".to_string(),
            "anthropic/claude-3.5-sonnet".to_string(),
            1000,
            500,
            3.0,
            1000,
        );
        tracker.add_pass_one_cost(cost);

        assert!(tracker.check_budget(5.0).is_ok());
        assert!(tracker.check_budget(2.0).is_err());
    }

    #[test]
    fn test_cost_breakdown() {
        let session_id = Uuid::new_v4();
        let mut tracker = AnalysisCostTracker::new(session_id);

        tracker.add_pass_one_cost(AgentCost::new(
            "Agent1".to_string(),
            "sonnet".to_string(),
            1000,
            500,
            0.01,
            1000,
        ));
        tracker.add_pass_two_cost(AgentCost::new(
            "Agent2".to_string(),
            "sonnet".to_string(),
            2000,
            1000,
            0.02,
            2000,
        ));

        let breakdown = tracker.cost_breakdown();
        assert!((breakdown["pass_one"] - 0.01).abs() < 0.001);
        assert!((breakdown["pass_two"] - 0.02).abs() < 0.001);
        assert!((breakdown["total"] - 0.03).abs() < 0.001);
    }
}
