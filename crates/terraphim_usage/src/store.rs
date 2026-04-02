use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use terraphim_persistence::{Persistable, Result};

/// Persistable agent metrics record.
///
/// Stored as JSON via the `Persistable` trait using the fastest configured
/// storage operator (SQLite by default). Key format: `usage/metrics/<agent_name>.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetricsRecord {
    pub agent_name: String,
    pub budget_monthly_cents: Option<u64>,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_sub_cents: u64,
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub first_execution_at: Option<String>,
    pub last_execution_at: Option<String>,
    pub updated_at: String,
}

#[async_trait]
impl Persistable for AgentMetricsRecord {
    fn new(key: String) -> Self {
        Self {
            agent_name: key,
            budget_monthly_cents: None,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost_sub_cents: 0,
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            first_execution_at: None,
            last_execution_at: None,
            updated_at: Utc::now().to_rfc3339(),
        }
    }

    async fn save(&self) -> Result<()> {
        self.save_to_all().await
    }

    async fn save_to_one(&self, profile_name: &str) -> Result<()> {
        self.save_to_profile(profile_name).await
    }

    async fn load(&mut self) -> Result<Self>
    where
        Self: Sized,
    {
        let op = &self.load_config().await?.1;
        let key = self.get_key();
        self.load_from_operator(&key, op).await
    }

    fn get_key(&self) -> String {
        let normalized = self.normalize_key(&self.agent_name);
        format!("usage/metrics/{}.json", normalized)
    }
}

/// Persistable execution history record.
///
/// Key format: `usage/executions/<agent_name>/<timestamp>.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub agent_name: String,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub cost_sub_cents: Option<u64>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub latency_ms: Option<u64>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub gitea_issue: Option<u64>,
}

impl ExecutionRecord {
    /// Create a unique key for this execution record.
    pub fn storage_key(&self) -> String {
        let normalized = self.agent_name.replace(|c: char| !c.is_alphanumeric(), "_");
        let ts = self.started_at.replace(|c: char| !c.is_alphanumeric(), "_");
        format!("usage/executions/{}/{}.json", normalized, ts)
    }
}

#[async_trait]
impl Persistable for ExecutionRecord {
    fn new(key: String) -> Self {
        Self {
            agent_name: key,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            cost_sub_cents: None,
            model: None,
            provider: None,
            success: false,
            error_message: None,
            latency_ms: None,
            started_at: Utc::now().to_rfc3339(),
            completed_at: None,
            gitea_issue: None,
        }
    }

    async fn save(&self) -> Result<()> {
        self.save_to_all().await
    }

    async fn save_to_one(&self, profile_name: &str) -> Result<()> {
        self.save_to_profile(profile_name).await
    }

    async fn load(&mut self) -> Result<Self>
    where
        Self: Sized,
    {
        let op = &self.load_config().await?.1;
        let key = self.get_key();
        self.load_from_operator(&key, op).await
    }

    fn get_key(&self) -> String {
        self.storage_key()
    }
}

/// Persistable provider usage snapshot.
///
/// Key format: `usage/providers/<provider_id>.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderUsageSnapshot {
    pub provider_id: String,
    pub snapshot_json: String,
    pub fetched_at: String,
}

#[async_trait]
impl Persistable for ProviderUsageSnapshot {
    fn new(key: String) -> Self {
        Self {
            provider_id: key,
            snapshot_json: String::new(),
            fetched_at: Utc::now().to_rfc3339(),
        }
    }

    async fn save(&self) -> Result<()> {
        self.save_to_all().await
    }

    async fn save_to_one(&self, profile_name: &str) -> Result<()> {
        self.save_to_profile(profile_name).await
    }

    async fn load(&mut self) -> Result<Self>
    where
        Self: Sized,
    {
        let op = &self.load_config().await?.1;
        let key = self.get_key();
        self.load_from_operator(&key, op).await
    }

    fn get_key(&self) -> String {
        let normalized = self.normalize_key(&self.provider_id);
        format!("usage/providers/{}.json", normalized)
    }
}

/// Persistable budget snapshot.
///
/// Key format: `usage/budgets/<agent_name>/<timestamp>.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetSnapshotRecord {
    pub agent_name: String,
    pub budget_cents: Option<u64>,
    pub spent_sub_cents: u64,
    pub percentage_used: f64,
    pub verdict: String,
    pub snapshot_at: String,
}

impl BudgetSnapshotRecord {
    pub fn storage_key(&self) -> String {
        let normalized = self.agent_name.replace(|c: char| !c.is_alphanumeric(), "_");
        let ts = self
            .snapshot_at
            .replace(|c: char| !c.is_alphanumeric(), "_");
        format!("usage/budgets/{}/{}.json", normalized, ts)
    }
}

#[async_trait]
impl Persistable for BudgetSnapshotRecord {
    fn new(key: String) -> Self {
        Self {
            agent_name: key,
            budget_cents: None,
            spent_sub_cents: 0,
            percentage_used: 0.0,
            verdict: String::new(),
            snapshot_at: Utc::now().to_rfc3339(),
        }
    }

    async fn save(&self) -> Result<()> {
        self.save_to_all().await
    }

    async fn save_to_one(&self, profile_name: &str) -> Result<()> {
        self.save_to_profile(profile_name).await
    }

    async fn load(&mut self) -> Result<Self>
    where
        Self: Sized,
    {
        let op = &self.load_config().await?.1;
        let key = self.get_key();
        self.load_from_operator(&key, op).await
    }

    fn get_key(&self) -> String {
        self.storage_key()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn init_memory_persistence() {
        terraphim_persistence::DeviceStorage::init_memory_only()
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_agent_metrics_save_and_load() {
        init_memory_persistence().await;

        let metrics = AgentMetricsRecord {
            agent_name: "test-agent".to_string(),
            budget_monthly_cents: Some(10000),
            total_input_tokens: 1000,
            total_output_tokens: 500,
            total_cost_sub_cents: 150,
            total_executions: 10,
            successful_executions: 9,
            failed_executions: 1,
            first_execution_at: Some("2026-04-01T00:00:00Z".to_string()),
            last_execution_at: Some("2026-04-02T00:00:00Z".to_string()),
            updated_at: Utc::now().to_rfc3339(),
        };

        metrics.save().await.unwrap();

        let mut loaded = AgentMetricsRecord::new("test-agent".to_string());
        let result = loaded.load().await;
        assert!(result.is_ok());
        let loaded = result.unwrap();
        assert_eq!(loaded.total_input_tokens, 1000);
        assert_eq!(loaded.total_output_tokens, 500);
        assert_eq!(loaded.total_cost_sub_cents, 150);
    }

    #[tokio::test]
    async fn test_execution_record_save_and_load() {
        init_memory_persistence().await;

        let exec = ExecutionRecord {
            agent_name: "test-agent".to_string(),
            input_tokens: Some(1000),
            output_tokens: Some(500),
            total_tokens: Some(1500),
            cost_sub_cents: Some(15),
            model: Some("claude-3-5-sonnet".to_string()),
            provider: Some("anthropic".to_string()),
            success: true,
            error_message: None,
            latency_ms: Some(2500),
            started_at: "2026-04-02T10:00:00Z".to_string(),
            completed_at: Some("2026-04-02T10:00:02Z".to_string()),
            gitea_issue: Some(42),
        };

        exec.save().await.unwrap();

        // Load using the same key that was used for save
        let mut loaded = ExecutionRecord::new("test-agent".to_string());
        loaded.started_at = "2026-04-02T10:00:00Z".to_string();
        let result = loaded.load().await;
        assert!(result.is_ok());
        let loaded = result.unwrap();
        assert_eq!(loaded.input_tokens, Some(1000));
        assert_eq!(loaded.gitea_issue, Some(42));
    }

    #[tokio::test]
    async fn test_provider_usage_save_and_load() {
        init_memory_persistence().await;

        let snapshot = ProviderUsageSnapshot {
            provider_id: "claude".to_string(),
            snapshot_json: r#"{"session": 42, "weekly": 60}"#.to_string(),
            fetched_at: "2026-04-02T10:00:00Z".to_string(),
        };

        snapshot.save().await.unwrap();

        let mut loaded = ProviderUsageSnapshot::new("claude".to_string());
        let result = loaded.load().await;
        assert!(result.is_ok());
        let loaded = result.unwrap();
        assert_eq!(loaded.snapshot_json, r#"{"session": 42, "weekly": 60}"#);
    }

    #[tokio::test]
    async fn test_budget_snapshot_save_and_load() {
        init_memory_persistence().await;

        let now = "2026-04-02T10:00:00Z";
        let budget = BudgetSnapshotRecord {
            agent_name: "test-agent".to_string(),
            budget_cents: Some(10000),
            spent_sub_cents: 8000,
            percentage_used: 80.0,
            verdict: "NearExhaustion".to_string(),
            snapshot_at: now.to_string(),
        };

        budget.save().await.unwrap();

        // Load using the same timestamp that was used for save
        let mut loaded = BudgetSnapshotRecord::new("test-agent".to_string());
        loaded.snapshot_at = now.to_string();
        let result = loaded.load().await;
        assert!(result.is_ok());
        let loaded = result.unwrap();
        assert_eq!(loaded.spent_sub_cents, 8000);
        assert_eq!(loaded.percentage_used, 80.0);
    }
}
