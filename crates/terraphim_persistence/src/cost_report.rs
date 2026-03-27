//! Cost reporting persistence for agent budget tracking.
//!
//! Provides storage and retrieval of cost snapshots for monthly budget reporting.

use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

/// Cost snapshot for a single agent at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentCostSnapshot {
    pub agent_name: String,
    pub spent_usd: f64,
    pub budget_cents: Option<u64>,
    pub verdict: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Monthly cost report aggregating all agent spending.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyCostReport {
    pub year: i32,
    pub month: u32,
    pub agents: Vec<AgentCostSnapshot>,
    pub total_fleet_spend_usd: f64,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

impl MonthlyCostReport {
    /// Create a new monthly cost report from agent snapshots.
    pub fn new(year: i32, month: u32, agents: Vec<AgentCostSnapshot>) -> Self {
        let total_fleet_spend_usd = agents.iter().map(|a| a.spent_usd).sum();
        Self {
            year,
            month,
            agents,
            total_fleet_spend_usd,
            generated_at: chrono::Utc::now(),
        }
    }

    /// Get the budget status summary for the report.
    pub fn budget_summary(&self) -> BudgetSummary {
        let mut summary = BudgetSummary::default();

        for agent in &self.agents {
            if let Some(budget_cents) = agent.budget_cents {
                summary.total_budgeted_agents += 1;
                let budget_usd = budget_cents as f64 / 100.0;
                summary.total_budget_usd += budget_usd;

                if agent.verdict.contains("exhausted") {
                    summary.exhausted_agents.push(agent.agent_name.clone());
                } else if agent.verdict.contains("near") {
                    summary.near_limit_agents.push(agent.agent_name.clone());
                }
            } else {
                summary.uncapped_agents.push(agent.agent_name.clone());
            }
        }

        summary.spent_usd = self.total_fleet_spend_usd;
        summary.remaining_usd = (summary.total_budget_usd - summary.spent_usd).max(0.0);

        summary
    }
}

/// Summary of budget status across all agents.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BudgetSummary {
    pub total_budgeted_agents: usize,
    pub total_budget_usd: f64,
    pub spent_usd: f64,
    pub remaining_usd: f64,
    pub exhausted_agents: Vec<String>,
    pub near_limit_agents: Vec<String>,
    pub uncapped_agents: Vec<String>,
}

/// Cost report persistence trait.
#[async_trait::async_trait]
pub trait CostReportPersistence: Send + Sync {
    /// Save a monthly cost report.
    async fn save_monthly_report(&self, report: &MonthlyCostReport) -> crate::Result<()>;

    /// Load a monthly cost report for a specific year and month.
    async fn load_monthly_report(
        &self,
        year: i32,
        month: u32,
    ) -> crate::Result<Option<MonthlyCostReport>>;

    /// List all available monthly reports.
    async fn list_reports(&self) -> crate::Result<Vec<(i32, u32)>>;

    /// Save the current cost snapshot (non-monthly, for real-time tracking).
    async fn save_current_snapshot(
        &self,
        agent_name: &str,
        snapshot: &AgentCostSnapshot,
    ) -> crate::Result<()>;

    /// Load the current cost snapshot for an agent.
    async fn load_current_snapshot(
        &self,
        agent_name: &str,
    ) -> crate::Result<Option<AgentCostSnapshot>>;

    /// Load all current snapshots.
    async fn load_all_current_snapshots(&self) -> crate::Result<Vec<AgentCostSnapshot>>;
}

/// OpenDAL-based implementation of cost report persistence.
pub struct OpenDALCostReportPersistence {
    operator: opendal::Operator,
    prefix: String,
}

impl OpenDALCostReportPersistence {
    /// Create a new persistence instance with the given operator.
    pub fn new(operator: opendal::Operator) -> Self {
        Self {
            operator,
            prefix: "cost-reports".to_string(),
        }
    }

    /// Create a new persistence instance with a custom prefix.
    pub fn with_prefix(operator: opendal::Operator, prefix: impl Into<String>) -> Self {
        Self {
            operator,
            prefix: prefix.into(),
        }
    }

    fn monthly_report_path(&self, year: i32, month: u32) -> String {
        format!("{}/monthly/{}-{:02}.json", self.prefix, year, month)
    }

    fn current_snapshot_path(&self, agent_name: &str) -> String {
        format!("{}/current/{}.json", self.prefix, agent_name)
    }
}

#[async_trait::async_trait]
impl CostReportPersistence for OpenDALCostReportPersistence {
    async fn save_monthly_report(&self, report: &MonthlyCostReport) -> crate::Result<()> {
        let path = self.monthly_report_path(report.year, report.month);
        let json =
            serde_json::to_string_pretty(report).map_err(|e| crate::Error::Serde(e.to_string()))?;

        self.operator
            .write(&path, json)
            .await
            .map_err(|e| crate::Error::Storage(format!("Failed to save report: {}", e)))?;

        info!(path = %path, "saved monthly cost report");
        Ok(())
    }

    async fn load_monthly_report(
        &self,
        year: i32,
        month: u32,
    ) -> crate::Result<Option<MonthlyCostReport>> {
        let path = self.monthly_report_path(year, month);

        match self.operator.read(&path).await {
            Ok(data) => {
                let json = String::from_utf8(data.to_vec())
                    .map_err(|e| crate::Error::Serde(format!("Invalid UTF-8: {}", e)))?;
                let report: MonthlyCostReport = serde_json::from_str(&json)
                    .map_err(|e| crate::Error::Serde(format!("Failed to parse report: {}", e)))?;
                Ok(Some(report))
            }
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(crate::Error::Storage(format!(
                "Failed to load report: {}",
                e
            ))),
        }
    }

    async fn list_reports(&self) -> crate::Result<Vec<(i32, u32)>> {
        let prefix = format!("{}/monthly/", self.prefix);
        let mut reports = Vec::new();

        match self.operator.list(&prefix).await {
            Ok(entries) => {
                for entry in entries {
                    let name = entry.name();
                    if name.ends_with(".json") {
                        // Parse filename like "2026-03.json"
                        let stem = name.trim_end_matches(".json");
                        let parts: Vec<&str> = stem.split('-').collect();
                        if parts.len() == 2 {
                            if let (Ok(year), Ok(month)) =
                                (parts[0].parse::<i32>(), parts[1].parse::<u32>())
                            {
                                reports.push((year, month));
                            }
                        }
                    }
                }
                reports.sort_by(|a, b| b.cmp(a)); // Most recent first
                Ok(reports)
            }
            Err(e) => {
                error!(error = %e, "failed to list reports");
                Err(crate::Error::Storage(format!(
                    "Failed to list reports: {}",
                    e
                )))
            }
        }
    }

    async fn save_current_snapshot(
        &self,
        agent_name: &str,
        snapshot: &AgentCostSnapshot,
    ) -> crate::Result<()> {
        let path = self.current_snapshot_path(agent_name);
        let json = serde_json::to_string_pretty(snapshot)
            .map_err(|e| crate::Error::Serde(e.to_string()))?;

        self.operator
            .write(&path, json)
            .await
            .map_err(|e| crate::Error::Storage(format!("Failed to save snapshot: {}", e)))?;

        debug!(agent = %agent_name, "saved cost snapshot");
        Ok(())
    }

    async fn load_current_snapshot(
        &self,
        agent_name: &str,
    ) -> crate::Result<Option<AgentCostSnapshot>> {
        let path = self.current_snapshot_path(agent_name);

        match self.operator.read(&path).await {
            Ok(data) => {
                let json = String::from_utf8(data.to_vec())
                    .map_err(|e| crate::Error::Serde(format!("Invalid UTF-8: {}", e)))?;
                let snapshot: AgentCostSnapshot = serde_json::from_str(&json)
                    .map_err(|e| crate::Error::Serde(format!("Failed to parse snapshot: {}", e)))?;
                Ok(Some(snapshot))
            }
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(crate::Error::Storage(format!(
                "Failed to load snapshot: {}",
                e
            ))),
        }
    }

    async fn load_all_current_snapshots(&self) -> crate::Result<Vec<AgentCostSnapshot>> {
        let prefix = format!("{}/current/", self.prefix);
        let mut snapshots = Vec::new();

        match self.operator.list(&prefix).await {
            Ok(entries) => {
                for entry in entries {
                    let name = entry.name();
                    if name.ends_with(".json") {
                        let agent_name = name.trim_end_matches(".json");
                        if let Ok(Some(snapshot)) = self.load_current_snapshot(agent_name).await {
                            snapshots.push(snapshot);
                        }
                    }
                }
                Ok(snapshots)
            }
            Err(e) => {
                error!(error = %e, "failed to list snapshots");
                Err(crate::Error::Storage(format!(
                    "Failed to list snapshots: {}",
                    e
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monthly_cost_report_creation() {
        let agents = vec![
            AgentCostSnapshot {
                agent_name: "agent-1".to_string(),
                spent_usd: 50.0,
                budget_cents: Some(10000),
                verdict: "within budget".to_string(),
                timestamp: chrono::Utc::now(),
            },
            AgentCostSnapshot {
                agent_name: "agent-2".to_string(),
                spent_usd: 85.0,
                budget_cents: Some(10000),
                verdict: "near exhaustion".to_string(),
                timestamp: chrono::Utc::now(),
            },
        ];

        let report = MonthlyCostReport::new(2026, 3, agents);
        assert_eq!(report.year, 2026);
        assert_eq!(report.month, 3);
        assert_eq!(report.total_fleet_spend_usd, 135.0);
        assert_eq!(report.agents.len(), 2);
    }

    #[test]
    fn test_budget_summary() {
        let agents = vec![
            AgentCostSnapshot {
                agent_name: "budgeted-ok".to_string(),
                spent_usd: 50.0,
                budget_cents: Some(10000), // $100
                verdict: "within budget".to_string(),
                timestamp: chrono::Utc::now(),
            },
            AgentCostSnapshot {
                agent_name: "near-limit".to_string(),
                spent_usd: 85.0,
                budget_cents: Some(10000),
                verdict: "near exhaustion".to_string(),
                timestamp: chrono::Utc::now(),
            },
            AgentCostSnapshot {
                agent_name: "exhausted".to_string(),
                spent_usd: 100.0,
                budget_cents: Some(10000),
                verdict: "exhausted".to_string(),
                timestamp: chrono::Utc::now(),
            },
            AgentCostSnapshot {
                agent_name: "uncapped".to_string(),
                spent_usd: 200.0,
                budget_cents: None,
                verdict: "uncapped".to_string(),
                timestamp: chrono::Utc::now(),
            },
        ];

        let report = MonthlyCostReport::new(2026, 3, agents);
        let summary = report.budget_summary();

        assert_eq!(summary.total_budgeted_agents, 3);
        assert_eq!(summary.total_budget_usd, 300.0);
        assert_eq!(summary.spent_usd, 435.0); // 50 + 85 + 100 + 200
        assert_eq!(summary.exhausted_agents, vec!["exhausted"]);
        assert_eq!(summary.near_limit_agents, vec!["near-limit"]);
        assert_eq!(summary.uncapped_agents, vec!["uncapped"]);
    }

    #[tokio::test]
    async fn test_opendal_persistence_memory() {
        // Create an in-memory OpenDAL operator for testing
        let operator = opendal::Operator::new(opendal::services::Memory::default())
            .expect("Failed to create memory operator")
            .finish();

        let persistence = OpenDALCostReportPersistence::new(operator);

        // Create and save a report
        let agents = vec![AgentCostSnapshot {
            agent_name: "test-agent".to_string(),
            spent_usd: 75.0,
            budget_cents: Some(10000),
            verdict: "within budget".to_string(),
            timestamp: chrono::Utc::now(),
        }];
        let report = MonthlyCostReport::new(2026, 3, agents);

        persistence
            .save_monthly_report(&report)
            .await
            .expect("Failed to save report");

        // Load it back
        let loaded = persistence
            .load_monthly_report(2026, 3)
            .await
            .expect("Failed to load report");
        assert!(loaded.is_some());

        let loaded_report = loaded.unwrap();
        assert_eq!(loaded_report.year, 2026);
        assert_eq!(loaded_report.month, 3);
        assert_eq!(loaded_report.agents.len(), 1);
        assert_eq!(loaded_report.agents[0].agent_name, "test-agent");
    }

    #[tokio::test]
    async fn test_current_snapshot_persistence() {
        let operator = opendal::Operator::new(opendal::services::Memory::default())
            .expect("Failed to create memory operator")
            .finish();

        let persistence = OpenDALCostReportPersistence::new(operator);

        let snapshot = AgentCostSnapshot {
            agent_name: "snapshot-agent".to_string(),
            spent_usd: 42.0,
            budget_cents: Some(5000),
            verdict: "within budget".to_string(),
            timestamp: chrono::Utc::now(),
        };

        // Save snapshot
        persistence
            .save_current_snapshot("snapshot-agent", &snapshot)
            .await
            .expect("Failed to save snapshot");

        // Load it back
        let loaded = persistence
            .load_current_snapshot("snapshot-agent")
            .await
            .expect("Failed to load snapshot");

        assert!(loaded.is_some());
        let loaded_snapshot = loaded.unwrap();
        assert_eq!(loaded_snapshot.agent_name, "snapshot-agent");
        assert_eq!(loaded_snapshot.spent_usd, 42.0);
    }
}
