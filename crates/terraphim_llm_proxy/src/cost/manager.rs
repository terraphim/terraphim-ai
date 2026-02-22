//! Budget management and cost tracking

use crate::cost::{CostCalculator, CostConfig, CostEstimate};
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Budget manager for tracking and enforcing cost limits
#[derive(Debug, Clone)]
pub struct BudgetManager {
    config: Arc<RwLock<CostConfig>>,
    #[allow(dead_code)]
    calculator: Arc<CostCalculator>,
    budgets: Arc<RwLock<HashMap<String, BudgetInfo>>>,
    cost_history: Arc<RwLock<Vec<CostRecord>>>,
}

/// Budget information for a user or session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetInfo {
    pub id: String,
    pub limit: f64,
    pub current_spend: f64,
    pub period: BudgetPeriod,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub alerts: Vec<BudgetAlert>,
    pub auto_enforce: bool,
}

/// Budget period types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BudgetPeriod {
    Daily,
    Weekly,
    Monthly,
    Yearly,
    OneTime,
    Custom(Duration),
}

/// Budget alert information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetAlert {
    pub level: AlertLevel,
    pub threshold: f64,
    pub triggered_at: DateTime<Utc>,
    pub message: String,
}

/// Alert levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

/// Cost record for tracking expenses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRecord {
    pub id: String,
    pub budget_id: String,
    pub timestamp: DateTime<Utc>,
    pub provider: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost: f64,
    pub currency: String,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
}

impl BudgetManager {
    /// Create a new budget manager
    pub fn new(config: CostConfig, calculator: Arc<CostCalculator>) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            calculator,
            budgets: Arc::new(RwLock::new(HashMap::new())),
            cost_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a new budget
    pub async fn create_budget(
        &self,
        id: String,
        limit: f64,
        period: BudgetPeriod,
        auto_enforce: bool,
    ) -> Result<(), BudgetError> {
        let mut budgets = self.budgets.write().await;

        if budgets.contains_key(&id) {
            return Err(BudgetError::BudgetExists(id));
        }

        let (start_time, end_time) = match &period {
            BudgetPeriod::Daily => {
                let start = Utc::now()
                    .date_naive()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc();
                let end = start + Duration::days(1);
                (start, Some(end))
            }
            BudgetPeriod::Weekly => {
                let start = Utc::now()
                    .date_naive()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
                    - Duration::days(chrono::Weekday::Mon.num_days_from_monday() as i64);
                let end = start + Duration::weeks(1);
                (start, Some(end))
            }
            BudgetPeriod::Monthly => {
                let now = Utc::now();
                let start = now
                    .date_naive()
                    .with_day(1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc();
                let end = if now.month() == 12 {
                    (now.year() + 1, 1, 1)
                } else {
                    (now.year(), now.month() + 1, 1)
                };
                let end = Utc.with_ymd_and_hms(end.0, end.1, end.2, 0, 0, 0).unwrap();
                (start, Some(end))
            }
            BudgetPeriod::Yearly => {
                let now = Utc::now();
                let start = Utc.with_ymd_and_hms(now.year(), 1, 1, 0, 0, 0).unwrap();
                let end = Utc.with_ymd_and_hms(now.year() + 1, 1, 1, 0, 0, 0).unwrap();
                (start, Some(end))
            }
            BudgetPeriod::OneTime => (Utc::now(), None),
            BudgetPeriod::Custom(duration) => {
                let start = Utc::now();
                let end = start + *duration;
                (start, Some(end))
            }
        };

        let budget = BudgetInfo {
            id: id.clone(),
            limit,
            current_spend: 0.0,
            period,
            start_time,
            end_time,
            alerts: Vec::new(),
            auto_enforce,
        };
        info!("Created budget: {} with limit ${:.2}", id, limit);

        budgets.insert(id, budget);

        Ok(())
    }

    /// Check if a request is within budget
    pub async fn check_request_cost(
        &self,
        budget_id: &str,
        estimate: &CostEstimate,
    ) -> Result<BudgetCheckResult, BudgetError> {
        let budgets = self.budgets.read().await;
        let budget = budgets
            .get(budget_id)
            .ok_or(BudgetError::BudgetNotFound(budget_id.to_string()))?;

        // Check if budget is expired
        if let Some(end_time) = budget.end_time {
            if Utc::now() > end_time {
                return Err(BudgetError::BudgetExpired(budget_id.to_string()));
            }
        }

        let projected_spend = budget.current_spend + estimate.estimated_cost;
        let usage_percentage = projected_spend / budget.limit;

        // Check if request would exceed budget
        if projected_spend > budget.limit && budget.auto_enforce {
            return Ok(BudgetCheckResult {
                allowed: false,
                reason: Some(format!(
                    "Request would exceed budget limit of ${:.2}",
                    budget.limit
                )),
                projected_spend,
                remaining_budget: budget.limit - budget.current_spend,
                alerts: self.generate_alerts(budget, usage_percentage).await,
            });
        }

        // Generate alerts if needed
        let alerts = self.generate_alerts(budget, usage_percentage).await;

        Ok(BudgetCheckResult {
            allowed: true,
            reason: None,
            projected_spend,
            remaining_budget: budget.limit - budget.current_spend,
            alerts,
        })
    }

    /// Record actual cost after request completion
    pub async fn record_cost(&self, record: CostRecord) -> Result<(), BudgetError> {
        let mut budgets = self.budgets.write().await;
        let budget = budgets
            .get_mut(&record.budget_id)
            .ok_or(BudgetError::BudgetNotFound(record.budget_id.clone()))?;

        budget.current_spend += record.cost;

        // Update alerts
        let usage_percentage = budget.current_spend / budget.limit;
        let new_alerts = self.generate_alerts(budget, usage_percentage).await;
        for alert in new_alerts {
            if !budget.alerts.iter().any(|a| a.level == alert.level) {
                budget.alerts.push(alert);
            }
        }

        // Store cost record
        let mut history = self.cost_history.write().await;
        debug!(
            "Recorded cost: ${:.6} for budget: {}",
            record.cost, record.budget_id
        );
        history.push(record);

        Ok(())
    }

    /// Get budget information
    pub async fn get_budget(&self, id: &str) -> Option<BudgetInfo> {
        let budgets = self.budgets.read().await;
        budgets.get(id).cloned()
    }

    /// Get all budgets
    pub async fn get_all_budgets(&self) -> Vec<BudgetInfo> {
        let budgets = self.budgets.read().await;
        budgets.values().cloned().collect()
    }

    /// Get cost history for a budget
    pub async fn get_cost_history(&self, budget_id: &str, limit: Option<usize>) -> Vec<CostRecord> {
        let history = self.cost_history.read().await;
        history
            .iter()
            .filter(|record| record.budget_id == budget_id)
            .rev()
            .take(limit.unwrap_or(100))
            .cloned()
            .collect()
    }

    /// Get cost analytics
    pub async fn get_cost_analytics(&self, budget_id: &str) -> Option<CostAnalytics> {
        let budgets = self.budgets.read().await;
        let budget = budgets.get(budget_id)?;

        let history = self.cost_history.read().await;
        let budget_history: Vec<_> = history
            .iter()
            .filter(|record| record.budget_id == budget_id)
            .collect();

        let total_requests = budget_history.len();
        let total_cost: f64 = budget_history.iter().map(|r| r.cost).sum();
        let total_input_tokens: u32 = budget_history.iter().map(|r| r.input_tokens).sum();
        let total_output_tokens: u32 = budget_history.iter().map(|r| r.output_tokens).sum();

        let mut provider_costs: HashMap<String, f64> = HashMap::new();
        let mut model_costs: HashMap<String, f64> = HashMap::new();

        for record in budget_history.iter() {
            *provider_costs.entry(record.provider.clone()).or_insert(0.0) += record.cost;
            *model_costs.entry(record.model.clone()).or_insert(0.0) += record.cost;
        }

        Some(CostAnalytics {
            budget_id: budget_id.to_string(),
            total_requests,
            total_cost,
            remaining_budget: budget.limit - budget.current_spend,
            budget_limit: budget.limit,
            usage_percentage: (budget.current_spend / budget.limit) * 100.0,
            total_input_tokens,
            total_output_tokens,
            average_cost_per_request: if total_requests > 0 {
                total_cost / total_requests as f64
            } else {
                0.0
            },
            provider_costs,
            model_costs,
            period_start: budget.start_time,
            period_end: budget.end_time,
        })
    }

    /// Reset budget (for periodic budgets)
    pub async fn reset_budget(&self, id: &str) -> Result<(), BudgetError> {
        let mut budgets = self.budgets.write().await;
        let budget = budgets
            .get_mut(id)
            .ok_or(BudgetError::BudgetNotFound(id.to_string()))?;

        budget.current_spend = 0.0;
        budget.alerts.clear();

        // Update time for periodic budgets
        match &budget.period {
            BudgetPeriod::Daily => {
                let start = Utc::now()
                    .date_naive()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc();
                let end = start + Duration::days(1);
                budget.start_time = start;
                budget.end_time = Some(end);
            }
            BudgetPeriod::Weekly => {
                let start = Utc::now()
                    .date_naive()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
                    - Duration::days(chrono::Weekday::Mon.num_days_from_monday() as i64);
                let end = start + Duration::weeks(1);
                budget.start_time = start;
                budget.end_time = Some(end);
            }
            BudgetPeriod::Monthly => {
                let now = Utc::now();
                let start = now
                    .date_naive()
                    .with_day(1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc();
                let end = if now.month() == 12 {
                    (now.year() + 1, 1, 1)
                } else {
                    (now.year(), now.month() + 1, 1)
                };
                let end = Utc.with_ymd_and_hms(end.0, end.1, end.2, 0, 0, 0).unwrap();
                budget.start_time = start;
                budget.end_time = Some(end);
            }
            _ => {} // Other periods don't auto-reset
        }

        info!("Reset budget: {}", id);
        Ok(())
    }

    /// Generate alerts based on usage percentage
    async fn generate_alerts(
        &self,
        _budget: &BudgetInfo,
        usage_percentage: f64,
    ) -> Vec<BudgetAlert> {
        let config = self.config.read().await;
        let mut alerts = Vec::new();

        if usage_percentage >= config.budget_critical_threshold {
            alerts.push(BudgetAlert {
                level: AlertLevel::Critical,
                threshold: config.budget_critical_threshold,
                triggered_at: Utc::now(),
                message: format!(
                    "Budget usage at {:.1}% (critical threshold {:.1}%)",
                    usage_percentage * 100.0,
                    config.budget_critical_threshold * 100.0
                ),
            });
        } else if usage_percentage >= config.budget_warning_threshold {
            alerts.push(BudgetAlert {
                level: AlertLevel::Warning,
                threshold: config.budget_warning_threshold,
                triggered_at: Utc::now(),
                message: format!(
                    "Budget usage at {:.1}% (warning threshold {:.1}%)",
                    usage_percentage * 100.0,
                    config.budget_warning_threshold * 100.0
                ),
            });
        }

        alerts
    }
}

/// Result of budget check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetCheckResult {
    pub allowed: bool,
    pub reason: Option<String>,
    pub projected_spend: f64,
    pub remaining_budget: f64,
    pub alerts: Vec<BudgetAlert>,
}

/// Cost analytics information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostAnalytics {
    pub budget_id: String,
    pub total_requests: usize,
    pub total_cost: f64,
    pub remaining_budget: f64,
    pub budget_limit: f64,
    pub usage_percentage: f64,
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
    pub average_cost_per_request: f64,
    pub provider_costs: HashMap<String, f64>,
    pub model_costs: HashMap<String, f64>,
    pub period_start: DateTime<Utc>,
    pub period_end: Option<DateTime<Utc>>,
}

/// Budget management errors
#[derive(Debug, thiserror::Error)]
pub enum BudgetError {
    #[error("Budget not found: {0}")]
    BudgetNotFound(String),

    #[error("Budget already exists: {0}")]
    BudgetExists(String),

    #[error("Budget expired: {0}")]
    BudgetExpired(String),

    #[error("Invalid budget configuration: {0}")]
    InvalidConfiguration(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cost::{CostConfig, PricingDatabase};

    fn create_test_budget_manager() -> BudgetManager {
        let config = CostConfig {
            budget_warning_threshold: 0.5,
            budget_critical_threshold: 0.8,
            ..Default::default()
        };

        let pricing_db = Arc::new(PricingDatabase::new(config.clone()));
        let calculator = Arc::new(CostCalculator::new(pricing_db));

        BudgetManager::new(config, calculator)
    }

    #[tokio::test]
    async fn test_create_budget() {
        let manager = create_test_budget_manager();

        manager
            .create_budget("test-budget".to_string(), 100.0, BudgetPeriod::Daily, true)
            .await
            .unwrap();

        let budget = manager.get_budget("test-budget").await;
        assert!(budget.is_some());

        let budget = budget.unwrap();
        assert_eq!(budget.id, "test-budget");
        assert_eq!(budget.limit, 100.0);
        assert_eq!(budget.current_spend, 0.0);
    }

    #[tokio::test]
    async fn test_budget_check() {
        let manager = create_test_budget_manager();

        manager
            .create_budget("test-budget".to_string(), 10.0, BudgetPeriod::Daily, true)
            .await
            .unwrap();

        let estimate = CostEstimate {
            provider: "test-provider".to_string(),
            model: "test-model".to_string(),
            input_tokens: 1000,
            output_tokens: 500,
            estimated_cost: 5.0,
            currency: "USD".to_string(),
            confidence: 0.9,
            cost_breakdown: crate::cost::calculator::CostBreakdown {
                input_cost: 2.0,
                output_cost: 2.5,
                request_cost: 0.5,
                discounts: Vec::new(),
            },
        };

        let result = manager
            .check_request_cost("test-budget", &estimate)
            .await
            .unwrap();
        assert!(result.allowed);
        assert_eq!(result.projected_spend, 5.0);
        assert_eq!(result.remaining_budget, 10.0); // Current spend is 0, so remaining is 10 - 0 = 10
    }

    #[tokio::test]
    async fn test_budget_enforcement() {
        let manager = create_test_budget_manager();

        manager
            .create_budget("test-budget".to_string(), 10.0, BudgetPeriod::Daily, true)
            .await
            .unwrap();

        let estimate = CostEstimate {
            provider: "test-provider".to_string(),
            model: "test-model".to_string(),
            input_tokens: 1000,
            output_tokens: 500,
            estimated_cost: 15.0, // Over budget
            currency: "USD".to_string(),
            confidence: 0.9,
            cost_breakdown: crate::cost::calculator::CostBreakdown {
                input_cost: 6.0,
                output_cost: 7.5,
                request_cost: 1.5,
                discounts: Vec::new(),
            },
        };

        let result = manager
            .check_request_cost("test-budget", &estimate)
            .await
            .unwrap();
        assert!(!result.allowed);
        assert!(result.reason.is_some());
    }
}
