//! Token usage and cost tracking for agents

use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{AgentId, MultiAgentError, MultiAgentResult};

/// Cost record for tracking agent expenses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRecord {
    pub timestamp: DateTime<Utc>,
    pub agent_id: AgentId,
    pub operation_type: String,
    pub cost_usd: f64,
    pub metadata: HashMap<String, String>,
}

/// Token usage record for a single request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageRecord {
    /// Unique request ID
    pub request_id: Uuid,
    /// Timestamp of the request
    pub timestamp: DateTime<Utc>,
    /// Agent that made the request
    pub agent_id: AgentId,
    /// Model used
    pub model: String,
    /// Input tokens consumed
    pub input_tokens: u64,
    /// Output tokens generated
    pub output_tokens: u64,
    /// Total tokens (input + output)
    pub total_tokens: u64,
    /// Cost in USD
    pub cost_usd: f64,
    /// Request duration in milliseconds
    pub duration_ms: u64,
    /// Quality score (0.0 - 1.0)
    pub quality_score: Option<f64>,
}

impl TokenUsageRecord {
    pub fn new(
        agent_id: AgentId,
        model: String,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
        duration_ms: u64,
    ) -> Self {
        Self {
            request_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            agent_id,
            model,
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            cost_usd,
            duration_ms,
            quality_score: None,
        }
    }

    pub fn with_quality_score(mut self, score: f64) -> Self {
        self.quality_score = Some(score.clamp(0.0, 1.0));
        self
    }
}

/// Model pricing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Model name
    pub model: String,
    /// Cost per 1000 input tokens in USD
    pub input_cost_per_1k: f64,
    /// Cost per 1000 output tokens in USD
    pub output_cost_per_1k: f64,
    /// Maximum tokens per request
    pub max_tokens: u64,
    /// Context window size
    pub context_window: u64,
}

impl ModelPricing {
    pub fn calculate_cost(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        let input_cost = (input_tokens as f64 / 1000.0) * self.input_cost_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * self.output_cost_per_1k;
        input_cost + output_cost
    }
}

/// Token usage tracker for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageTracker {
    /// Agent ID
    pub agent_id: AgentId,
    /// All usage records
    pub records: Vec<TokenUsageRecord>,
    /// Total input tokens used
    pub total_input_tokens: u64,
    /// Total output tokens generated
    pub total_output_tokens: u64,
    /// Total requests made
    pub total_requests: u64,
    /// Total cost in USD
    pub total_cost_usd: f64,
    /// Average tokens per request
    pub avg_tokens_per_request: f64,
    /// Average cost per request
    pub avg_cost_per_request: f64,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

impl TokenUsageTracker {
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            records: Vec::new(),
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_requests: 0,
            total_cost_usd: 0.0,
            avg_tokens_per_request: 0.0,
            avg_cost_per_request: 0.0,
            last_updated: Utc::now(),
        }
    }

    /// Record a new token usage
    pub fn record_usage(&mut self, record: TokenUsageRecord) {
        self.total_input_tokens += record.input_tokens;
        self.total_output_tokens += record.output_tokens;
        self.total_requests += 1;
        self.total_cost_usd += record.cost_usd;

        self.avg_tokens_per_request = (self.total_input_tokens + self.total_output_tokens) as f64
            / self.total_requests as f64;
        self.avg_cost_per_request = self.total_cost_usd / self.total_requests as f64;

        self.last_updated = Utc::now();
        self.records.push(record);
    }

    /// Add a token usage record
    pub fn add_record(&mut self, record: TokenUsageRecord) -> MultiAgentResult<()> {
        self.record_usage(record);
        Ok(())
    }

    /// Get usage statistics for a time period
    pub fn get_usage_in_period(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> UsageStats {
        let period_records: Vec<&TokenUsageRecord> = self
            .records
            .iter()
            .filter(|r| r.timestamp >= start && r.timestamp <= end)
            .collect();

        let total_tokens: u64 = period_records.iter().map(|r| r.total_tokens).sum();
        let total_cost: f64 = period_records.iter().map(|r| r.cost_usd).sum();
        let request_count = period_records.len() as u64;

        UsageStats {
            period_start: start,
            period_end: end,
            request_count,
            total_input_tokens: period_records.iter().map(|r| r.input_tokens).sum(),
            total_output_tokens: period_records.iter().map(|r| r.output_tokens).sum(),
            total_tokens,
            total_cost_usd: total_cost,
            avg_tokens_per_request: if request_count > 0 {
                total_tokens as f64 / request_count as f64
            } else {
                0.0
            },
            avg_cost_per_request: if request_count > 0 {
                total_cost / request_count as f64
            } else {
                0.0
            },
        }
    }

    /// Get today's usage
    pub fn get_today_usage(&self) -> UsageStats {
        let today = Utc::now().date_naive();
        let start = today.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end = today.and_hms_opt(23, 59, 59).unwrap().and_utc();
        self.get_usage_in_period(start, end)
    }

    /// Get this month's usage
    pub fn get_month_usage(&self) -> UsageStats {
        let now = Utc::now();
        let start = now
            .date_naive()
            .with_day(1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let end = now;
        self.get_usage_in_period(start, end)
    }
}

/// Usage statistics for a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub request_count: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub avg_tokens_per_request: f64,
    pub avg_cost_per_request: f64,
}

/// Budget alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetAlert {
    /// Alert ID
    pub id: Uuid,
    /// Agent ID (None for global alerts)
    pub agent_id: Option<AgentId>,
    /// Alert threshold in USD
    pub threshold_usd: f64,
    /// Time window for the alert
    pub window: AlertWindow,
    /// Whether the alert is enabled
    pub enabled: bool,
    /// Alert actions to take
    pub actions: Vec<AlertAction>,
}

/// Time window for budget alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertWindow {
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

/// Actions to take when budget alert is triggered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertAction {
    Log,
    Email(String),
    Webhook(String),
    DisableAgent,
    RateLimit(u64), // requests per minute
}

/// Cost tracker with budget monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostTracker {
    /// Model pricing information
    pub model_pricing: HashMap<String, ModelPricing>,
    /// Budget alerts
    pub alerts: Vec<BudgetAlert>,
    /// Daily spending by agent
    pub daily_spending: HashMap<String, HashMap<AgentId, f64>>, // date -> agent_id -> cost
    /// Monthly budget in USD
    pub monthly_budget_usd: Option<f64>,
    /// Daily budget in USD
    pub daily_budget_usd: Option<f64>,
    /// Current month spending
    pub current_month_spending: f64,
    /// Current day spending
    pub current_day_spending: f64,
    /// Last updated
    pub last_updated: DateTime<Utc>,
}

impl CostTracker {
    pub fn new() -> Self {
        Self {
            model_pricing: Self::default_model_pricing(),
            alerts: Vec::new(),
            daily_spending: HashMap::new(),
            monthly_budget_usd: None,
            daily_budget_usd: None,
            current_month_spending: 0.0,
            current_day_spending: 0.0,
            last_updated: Utc::now(),
        }
    }

    /// Default model pricing (can be updated from config)
    fn default_model_pricing() -> HashMap<String, ModelPricing> {
        let mut pricing = HashMap::new();

        // OpenAI GPT models
        pricing.insert(
            "gpt-4".to_string(),
            ModelPricing {
                model: "gpt-4".to_string(),
                input_cost_per_1k: 0.03,
                output_cost_per_1k: 0.06,
                max_tokens: 4096,
                context_window: 8192,
            },
        );

        pricing.insert(
            "gpt-3.5-turbo".to_string(),
            ModelPricing {
                model: "gpt-3.5-turbo".to_string(),
                input_cost_per_1k: 0.001,
                output_cost_per_1k: 0.002,
                max_tokens: 4096,
                context_window: 16384,
            },
        );

        // Anthropic Claude models
        pricing.insert(
            "claude-3-opus".to_string(),
            ModelPricing {
                model: "claude-3-opus".to_string(),
                input_cost_per_1k: 0.015,
                output_cost_per_1k: 0.075,
                max_tokens: 4096,
                context_window: 200000,
            },
        );

        pricing
    }

    /// Calculate cost for a request
    pub fn calculate_cost(
        &self,
        model: &str,
        input_tokens: u64,
        output_tokens: u64,
    ) -> MultiAgentResult<f64> {
        let pricing = self.model_pricing.get(model).ok_or_else(|| {
            MultiAgentError::ConfigError(format!("No pricing data for model: {}", model))
        })?;

        Ok(pricing.calculate_cost(input_tokens, output_tokens))
    }

    /// Record spending for an agent
    pub fn record_spending(&mut self, agent_id: AgentId, cost_usd: f64) {
        let today = Utc::now().format("%Y-%m-%d").to_string();

        self.daily_spending
            .entry(today)
            .or_insert_with(HashMap::new)
            .entry(agent_id)
            .and_modify(|e| *e += cost_usd)
            .or_insert(cost_usd);

        self.current_day_spending += cost_usd;
        self.current_month_spending += cost_usd;
        self.last_updated = Utc::now();
    }

    /// Add a cost record
    pub fn add_record(&mut self, record: CostRecord) -> MultiAgentResult<()> {
        self.record_spending(record.agent_id, record.cost_usd);
        Ok(())
    }

    /// Check if any budget limits are exceeded
    pub fn check_budget_limits(
        &self,
        _agent_id: AgentId,
        additional_cost: f64,
    ) -> MultiAgentResult<()> {
        // Check daily budget
        if let Some(daily_budget) = self.daily_budget_usd {
            if self.current_day_spending + additional_cost > daily_budget {
                return Err(MultiAgentError::BudgetLimitExceeded {
                    current: self.current_day_spending + additional_cost,
                    limit: daily_budget,
                });
            }
        }

        // Check monthly budget
        if let Some(monthly_budget) = self.monthly_budget_usd {
            if self.current_month_spending + additional_cost > monthly_budget {
                return Err(MultiAgentError::BudgetLimitExceeded {
                    current: self.current_month_spending + additional_cost,
                    limit: monthly_budget,
                });
            }
        }

        Ok(())
    }

    /// Add a budget alert
    pub fn add_alert(&mut self, alert: BudgetAlert) {
        self.alerts.push(alert);
    }

    /// Check and trigger alerts
    pub fn check_alerts(&self, agent_id: AgentId, current_spending: f64) -> Vec<&BudgetAlert> {
        self.alerts
            .iter()
            .filter(|alert| {
                alert.enabled
                    && (alert.agent_id.is_none() || alert.agent_id == Some(agent_id))
                    && current_spending >= alert.threshold_usd
            })
            .collect()
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
    fn test_token_usage_record() {
        let agent_id = AgentId::new_v4();
        let record = TokenUsageRecord::new(agent_id, "gpt-4".to_string(), 100, 50, 0.01, 1000);

        assert_eq!(record.agent_id, agent_id);
        assert_eq!(record.model, "gpt-4");
        assert_eq!(record.input_tokens, 100);
        assert_eq!(record.output_tokens, 50);
        assert_eq!(record.total_tokens, 150);
        assert_eq!(record.cost_usd, 0.01);
        assert_eq!(record.duration_ms, 1000);
    }

    #[test]
    fn test_token_usage_tracker() {
        let agent_id = AgentId::new_v4();
        let mut tracker = TokenUsageTracker::new(agent_id);

        let record = TokenUsageRecord::new(agent_id, "gpt-4".to_string(), 100, 50, 0.01, 1000);

        tracker.record_usage(record);

        assert_eq!(tracker.total_input_tokens, 100);
        assert_eq!(tracker.total_output_tokens, 50);
        assert_eq!(tracker.total_requests, 1);
        assert_eq!(tracker.total_cost_usd, 0.01);
        assert_eq!(tracker.avg_tokens_per_request, 150.0);
        assert_eq!(tracker.avg_cost_per_request, 0.01);
    }

    #[test]
    fn test_model_pricing() {
        let pricing = ModelPricing {
            model: "gpt-4".to_string(),
            input_cost_per_1k: 0.03,
            output_cost_per_1k: 0.06,
            max_tokens: 4096,
            context_window: 8192,
        };

        let cost = pricing.calculate_cost(1000, 500);
        assert_eq!(cost, 0.03 + 0.03); // 1000 input + 500 output
    }

    #[test]
    fn test_cost_tracker() {
        let mut tracker = CostTracker::new();
        let agent_id = AgentId::new_v4();

        // Test cost calculation
        let cost = tracker.calculate_cost("gpt-4", 1000, 500).unwrap();
        assert_eq!(cost, 0.06); // 0.03 + 0.03

        // Test spending recording
        tracker.record_spending(agent_id, cost);
        assert_eq!(tracker.current_day_spending, cost);
        assert_eq!(tracker.current_month_spending, cost);
    }

    #[test]
    fn test_budget_limits() {
        let mut tracker = CostTracker::new();
        tracker.daily_budget_usd = Some(1.0);
        tracker.current_day_spending = 0.8;

        let agent_id = AgentId::new_v4();

        // Should pass - under budget
        assert!(tracker.check_budget_limits(agent_id, 0.1).is_ok());

        // Should fail - over budget
        assert!(tracker.check_budget_limits(agent_id, 0.3).is_err());
    }
}
