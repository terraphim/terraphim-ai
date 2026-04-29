use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use terraphim_persistence::Persistable;
use terraphim_types::LlmUsage;

/// Persistable record for agent-level metrics
///
/// Stores aggregated metrics for an agent including total tokens, costs,
/// and execution counts. Maps to the agent_metrics table in SQLite schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetricsRecord {
    /// Unique identifier for this record (agent_name)
    pub key: String,
    /// Agent name
    pub agent_name: String,
    /// Monthly budget in cents (USD)
    pub budget_monthly_cents: i64,
    /// Total input tokens consumed
    pub total_input_tokens: i64,
    /// Total output tokens consumed
    pub total_output_tokens: i64,
    /// Total cost in sub-cents (1/10000 of a cent for precision)
    pub total_cost_sub_cents: i64,
    /// Total number of executions
    pub total_executions: i64,
    /// Number of successful executions
    pub successful_executions: i64,
    /// Number of failed executions
    pub failed_executions: i64,
    /// Timestamp of first execution
    pub first_execution_at: Option<String>,
    /// Timestamp of last execution
    pub last_execution_at: Option<String>,
    /// When this record was last updated
    pub updated_at: String,
}

impl AgentMetricsRecord {
    /// Calculate total tokens (input + output)
    pub fn total_tokens(&self) -> i64 {
        self.total_input_tokens + self.total_output_tokens
    }

    /// Calculate cost in dollars
    pub fn total_cost_usd(&self) -> f64 {
        self.total_cost_sub_cents as f64 / 1_000_000.0
    }

    /// Calculate budget utilization percentage
    pub fn budget_percentage_used(&self) -> f64 {
        if self.budget_monthly_cents == 0 {
            0.0
        } else {
            let spent_cents = self.total_cost_sub_cents as f64 / 10_000.0;
            (spent_cents / self.budget_monthly_cents as f64) * 100.0
        }
    }

    /// Record a new execution, updating aggregates
    pub fn record_execution(
        &mut self,
        input_tokens: i64,
        output_tokens: i64,
        cost_sub_cents: i64,
        success: bool,
    ) {
        let now = Utc::now().to_rfc3339();

        self.total_input_tokens += input_tokens;
        self.total_output_tokens += output_tokens;
        self.total_cost_sub_cents += cost_sub_cents;
        self.total_executions += 1;

        if success {
            self.successful_executions += 1;
        } else {
            self.failed_executions += 1;
        }

        if self.first_execution_at.is_none() {
            self.first_execution_at = Some(now.clone());
        }
        self.last_execution_at = Some(now.clone());
        self.updated_at = now;
    }
}

#[async_trait::async_trait]
impl Persistable for AgentMetricsRecord {
    fn new(key: String) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            key: key.clone(),
            agent_name: key.clone(),
            budget_monthly_cents: 0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost_sub_cents: 0,
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            first_execution_at: None,
            last_execution_at: None,
            updated_at: now,
        }
    }

    async fn save(&self) -> terraphim_persistence::Result<()> {
        self.save_to_all().await
    }

    async fn save_to_one(&self, profile_name: &str) -> terraphim_persistence::Result<()> {
        self.save_to_profile(profile_name).await
    }

    async fn load(&mut self) -> terraphim_persistence::Result<Self> {
        let key = self.get_key();
        self.load_from_operator(&key, &self.load_config().await?.1)
            .await
    }

    fn get_key(&self) -> String {
        format!(
            "usage/metrics/{}.json",
            self.normalize_key(&self.agent_name)
        )
    }
}

/// Persistable record for individual execution history entries
///
/// Maps to the execution_history table in SQLite schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// Unique identifier for this record (auto-generated from timestamp + agent)
    pub key: String,
    /// Agent name that executed
    pub agent_name: String,
    /// Input tokens consumed
    pub input_tokens: i64,
    /// Output tokens consumed
    pub output_tokens: i64,
    /// Total tokens (input + output)
    pub total_tokens: i64,
    /// Cost in sub-cents
    pub cost_sub_cents: i64,
    /// Model used
    pub model: Option<String>,
    /// Provider used
    pub provider: Option<String>,
    /// Whether execution succeeded
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Latency in milliseconds
    pub latency_ms: Option<i64>,
    /// When execution started (RFC3339)
    pub started_at: String,
    /// When execution completed (RFC3339)
    pub completed_at: Option<String>,
    /// Associated Gitea issue number
    pub gitea_issue: Option<i64>,
}

impl ExecutionRecord {
    /// Calculate cost in dollars
    pub fn cost_usd(&self) -> f64 {
        self.cost_sub_cents as f64 / 1_000_000.0
    }

    /// Get duration since started
    pub fn duration_ms(&self) -> Option<i64> {
        if let (Some(completed), Ok(started)) = (
            &self.completed_at,
            DateTime::parse_from_rfc3339(&self.started_at),
        ) {
            if let Ok(completed_dt) = DateTime::parse_from_rfc3339(completed) {
                return Some((completed_dt - started).num_milliseconds());
            }
        }
        self.latency_ms
    }

    /// Create a key for this execution record
    #[allow(dead_code)]
    fn create_key(agent_name: &str, timestamp: &DateTime<Utc>) -> String {
        format!(
            "usage/executions/{}/{}.json",
            Persistable::normalize_key(
                &AgentMetricsRecord::new(agent_name.to_string()),
                agent_name
            ),
            timestamp.format("%Y%m%d_%H%M%S_%f")
        )
    }

    /// Convert from LlmUsage (terraphim_types) into an ExecutionRecord
    pub fn from_llm_usage(usage: &LlmUsage, agent_name: &str) -> Self {
        let now = Utc::now();
        let cost_sub_cents = usage
            .cost_usd
            .map(|c| (c * 1_000_000.0) as i64)
            .unwrap_or(0);
        Self {
            key: Self::create_key(agent_name, &now),
            agent_name: agent_name.to_string(),
            input_tokens: usage.input_tokens as i64,
            output_tokens: usage.output_tokens as i64,
            total_tokens: (usage.input_tokens + usage.output_tokens) as i64,
            cost_sub_cents,
            model: Some(usage.model.clone()),
            provider: Some(usage.provider.clone()),
            success: true,
            error_message: None,
            latency_ms: Some(usage.latency_ms as i64),
            started_at: now.to_rfc3339(),
            completed_at: Some(now.to_rfc3339()),
            gitea_issue: None,
        }
    }
}

#[async_trait::async_trait]
impl Persistable for ExecutionRecord {
    fn new(key: String) -> Self {
        Self {
            key,
            agent_name: String::new(),
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            cost_sub_cents: 0,
            model: None,
            provider: None,
            success: true,
            error_message: None,
            latency_ms: None,
            started_at: Utc::now().to_rfc3339(),
            completed_at: None,
            gitea_issue: None,
        }
    }

    async fn save(&self) -> terraphim_persistence::Result<()> {
        self.save_to_all().await
    }

    async fn save_to_one(&self, profile_name: &str) -> terraphim_persistence::Result<()> {
        self.save_to_profile(profile_name).await
    }

    async fn load(&mut self) -> terraphim_persistence::Result<Self> {
        let key = self.get_key();
        self.load_from_operator(&key, &self.load_config().await?.1)
            .await
    }

    fn get_key(&self) -> String {
        self.key.clone()
    }
}

/// Persistable record for budget snapshots
///
/// Maps to the budget_snapshots table in SQLite schema.
/// Used for auditing and tracking budget utilization over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetSnapshotRecord {
    /// Unique identifier for this record
    pub key: String,
    /// Agent name
    pub agent_name: String,
    /// Budget in cents
    pub budget_cents: i64,
    /// Amount spent in sub-cents
    pub spent_sub_cents: i64,
    /// Percentage of budget used
    pub percentage_used: f64,
    /// Budget verdict (within_budget, approaching_limit, exceeded)
    pub verdict: BudgetVerdict,
    /// When this snapshot was taken
    pub snapshot_at: String,
}

/// Budget verdict enum
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BudgetVerdict {
    WithinBudget,
    ApproachingLimit,
    Exceeded,
}

impl BudgetSnapshotRecord {
    /// Calculate spent amount in dollars
    pub fn spent_usd(&self) -> f64 {
        self.spent_sub_cents as f64 / 1_000_000.0
    }

    /// Calculate budget in dollars
    pub fn budget_usd(&self) -> f64 {
        self.budget_cents as f64 / 100.0
    }

    /// Create a new snapshot from agent metrics
    pub fn from_agent_metrics(metrics: &AgentMetricsRecord) -> Self {
        let percentage = metrics.budget_percentage_used();
        let verdict = if percentage >= 100.0 {
            BudgetVerdict::Exceeded
        } else if percentage >= 80.0 {
            BudgetVerdict::ApproachingLimit
        } else {
            BudgetVerdict::WithinBudget
        };

        Self {
            key: String::new(),
            agent_name: metrics.agent_name.clone(),
            budget_cents: metrics.budget_monthly_cents,
            spent_sub_cents: metrics.total_cost_sub_cents,
            percentage_used: percentage,
            verdict,
            snapshot_at: Utc::now().to_rfc3339(),
        }
    }
}

#[async_trait::async_trait]
impl Persistable for BudgetSnapshotRecord {
    fn new(key: String) -> Self {
        Self {
            key,
            agent_name: String::new(),
            budget_cents: 0,
            spent_sub_cents: 0,
            percentage_used: 0.0,
            verdict: BudgetVerdict::WithinBudget,
            snapshot_at: Utc::now().to_rfc3339(),
        }
    }

    async fn save(&self) -> terraphim_persistence::Result<()> {
        self.save_to_all().await
    }

    async fn save_to_one(&self, profile_name: &str) -> terraphim_persistence::Result<()> {
        self.save_to_profile(profile_name).await
    }

    async fn load(&mut self) -> terraphim_persistence::Result<Self> {
        let key = self.get_key();
        self.load_from_operator(&key, &self.load_config().await?.1)
            .await
    }

    fn get_key(&self) -> String {
        if self.key.is_empty() {
            format!(
                "usage/budgets/{}/{}.json",
                Persistable::normalize_key(
                    &AgentMetricsRecord::new(self.agent_name.clone()),
                    &self.agent_name
                ),
                Persistable::normalize_key(
                    &AgentMetricsRecord::new(self.agent_name.clone()),
                    &self.snapshot_at
                )
            )
        } else {
            self.key.clone()
        }
    }
}

/// Persistable record for provider usage snapshots
///
/// Maps to the provider_usage table in SQLite schema.
/// Stores cached snapshots of external provider usage data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderUsageSnapshot {
    /// Unique identifier for this record
    pub key: String,
    /// Provider ID (e.g., "claude", "kimi", "zai")
    pub provider_id: String,
    /// Raw JSON snapshot from provider API
    pub snapshot_json: String,
    /// When this snapshot was fetched
    pub fetched_at: String,
    /// Cache TTL in seconds
    pub ttl_seconds: u64,
}

impl ProviderUsageSnapshot {
    /// Check if this snapshot is expired
    pub fn is_expired(&self) -> bool {
        if let Ok(fetched) = DateTime::parse_from_rfc3339(&self.fetched_at) {
            let expiry = fetched + chrono::Duration::seconds(self.ttl_seconds as i64);
            return Utc::now().fixed_offset() > expiry;
        }
        true
    }

    /// Get remaining TTL in seconds
    pub fn remaining_ttl_seconds(&self) -> i64 {
        if let Ok(fetched) = DateTime::parse_from_rfc3339(&self.fetched_at) {
            let expiry = fetched + chrono::Duration::seconds(self.ttl_seconds as i64);
            let remaining = expiry - Utc::now().fixed_offset();
            return remaining.num_seconds().max(0);
        }
        0
    }
}

#[async_trait::async_trait]
impl Persistable for ProviderUsageSnapshot {
    fn new(key: String) -> Self {
        Self {
            key,
            provider_id: String::new(),
            snapshot_json: String::new(),
            fetched_at: Utc::now().to_rfc3339(),
            ttl_seconds: 300, // Default 5 minute TTL
        }
    }

    async fn save(&self) -> terraphim_persistence::Result<()> {
        self.save_to_all().await
    }

    async fn save_to_one(&self, profile_name: &str) -> terraphim_persistence::Result<()> {
        self.save_to_profile(profile_name).await
    }

    async fn load(&mut self) -> terraphim_persistence::Result<Self> {
        let key = self.get_key();
        self.load_from_operator(&key, &self.load_config().await?.1)
            .await
    }

    fn get_key(&self) -> String {
        if self.key.is_empty() {
            format!(
                "usage/providers/{}_{}.json",
                Persistable::normalize_key(
                    &AgentMetricsRecord::new(self.provider_id.clone()),
                    &self.provider_id
                ),
                Persistable::normalize_key(
                    &AgentMetricsRecord::new(self.provider_id.clone()),
                    &self.fetched_at
                )
            )
        } else {
            self.key.clone()
        }
    }
}

/// Usage store for querying and aggregating usage data.
///
/// Provides high-level query methods on top of the persistable record types.
#[derive(Debug, Clone)]
pub struct UsageStore;

impl UsageStore {
    /// Create a new usage store.
    pub fn new() -> Self {
        Self
    }

    /// List all agent metrics records.
    pub async fn list_agent_metrics(&self) -> crate::Result<Vec<AgentMetricsRecord>> {
        let storage = terraphim_persistence::DeviceStorage::instance()
            .await
            .map_err(|e| crate::UsageError::StorageError(e.to_string()))?;
        let op = &storage.fastest_op;

        let entries = op
            .list("usage/metrics/")
            .await
            .map_err(|e| crate::UsageError::StorageError(e.to_string()))?;

        let mut metrics = Vec::new();
        for entry in entries {
            let path: &str = entry.path();
            if path.ends_with(".json") {
                if let Some(filename) = path
                    .strip_prefix("usage/metrics/")
                    .and_then(|s: &str| s.strip_suffix(".json"))
                {
                    let mut record = AgentMetricsRecord::new(filename.to_string());
                    match record.load().await {
                        Ok(loaded) => metrics.push(loaded),
                        Err(e) => eprintln!("Warning: Failed to load {}: {}", path, e),
                    }
                }
            }
        }

        Ok(metrics)
    }

    /// Get agent metrics for a specific agent.
    pub async fn get_agent_metrics(
        &self,
        agent_name: &str,
    ) -> crate::Result<Option<AgentMetricsRecord>> {
        let mut record = AgentMetricsRecord::new(agent_name.to_string());
        match record.load().await {
            Ok(loaded) => Ok(Some(loaded)),
            Err(terraphim_persistence::Error::NotFound(_)) => Ok(None),
            Err(e) => Err(crate::UsageError::StorageError(e.to_string())),
        }
    }

    /// Save agent metrics.
    pub async fn save_agent_metrics(&self, metrics: &AgentMetricsRecord) -> crate::Result<()> {
        metrics
            .save()
            .await
            .map_err(|e| crate::UsageError::StorageError(e.to_string()))
    }

    /// Save execution record.
    pub async fn save_execution(&self, execution: &ExecutionRecord) -> crate::Result<()> {
        execution
            .save()
            .await
            .map_err(|e| crate::UsageError::StorageError(e.to_string()))
    }

    /// Save budget snapshot.
    pub async fn save_budget_snapshot(&self, snapshot: &BudgetSnapshotRecord) -> crate::Result<()> {
        snapshot
            .save()
            .await
            .map_err(|e| crate::UsageError::StorageError(e.to_string()))
    }

    /// Save provider usage snapshot.
    pub async fn save_provider_snapshot(
        &self,
        snapshot: &ProviderUsageSnapshot,
    ) -> crate::Result<()> {
        snapshot
            .save()
            .await
            .map_err(|e| crate::UsageError::StorageError(e.to_string()))
    }

    /// Query execution history for a date range.
    pub async fn query_executions(
        &self,
        since: &str,
        until: Option<&str>,
        agent_filter: Option<&str>,
    ) -> crate::Result<Vec<ExecutionRecord>> {
        let storage = terraphim_persistence::DeviceStorage::instance()
            .await
            .map_err(|e| crate::UsageError::StorageError(e.to_string()))?;
        let op = &storage.fastest_op;

        let since_dt: chrono::DateTime<chrono::FixedOffset> =
            chrono::DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", since))
                .or_else(|_| {
                    let d = chrono::NaiveDate::parse_from_str(since, "%Y-%m-%d")?;
                    Ok::<_, chrono::ParseError>(chrono::DateTime::from_naive_utc_and_offset(
                        d.and_hms_opt(0, 0, 0).unwrap(),
                        chrono::FixedOffset::east_opt(0).unwrap(),
                    ))
                })
                .map_err(|_| {
                    crate::UsageError::StorageError(format!("Invalid since date '{}'", since))
                })?;

        let until_dt: Option<chrono::DateTime<chrono::FixedOffset>> = match until {
            Some(u) => Some(
                chrono::DateTime::parse_from_rfc3339(&format!("{}T23:59:59Z", u))
                    .or_else(|_| {
                        let d = chrono::NaiveDate::parse_from_str(u, "%Y-%m-%d")?;
                        Ok::<_, chrono::ParseError>(chrono::DateTime::from_naive_utc_and_offset(
                            d.and_hms_opt(23, 59, 59).unwrap(),
                            chrono::FixedOffset::east_opt(0).unwrap(),
                        ))
                    })
                    .map_err(|_| {
                        crate::UsageError::StorageError(format!("Invalid until date '{}'", u))
                    })?,
            ),
            None => None,
        };

        // List execution files
        let prefix = match agent_filter {
            Some(agent) => format!(
                "usage/executions/{}/",
                Persistable::normalize_key(&AgentMetricsRecord::new(agent.to_string()), agent)
            ),
            None => "usage/executions/".to_string(),
        };

        let entries = op
            .list(&prefix)
            .await
            .map_err(|e| crate::UsageError::StorageError(e.to_string()))?;

        let mut executions = Vec::new();
        for entry in entries {
            let path: &str = entry.path();
            if path.ends_with(".json") {
                let content = op
                    .read(path)
                    .await
                    .map_err(|e| crate::UsageError::StorageError(e.to_string()))?;
                let exec: ExecutionRecord = serde_json::from_slice(&content.to_vec())
                    .map_err(crate::UsageError::SerializationError)?;
                let exec_dt = match chrono::DateTime::parse_from_rfc3339(&exec.started_at) {
                    Ok(dt) => dt,
                    Err(_) => continue,
                };

                if exec_dt >= since_dt {
                    if let Some(until) = until_dt {
                        if exec_dt <= until {
                            executions.push(exec);
                        }
                    } else {
                        executions.push(exec);
                    }
                }
            }
        }

        #[allow(clippy::unnecessary_sort_by)]
        executions.sort_by(|a, b| b.started_at.cmp(&a.started_at));

        Ok(executions)
    }

    /// Query budget snapshots for an agent.
    pub async fn query_budget_snapshots(
        &self,
        agent_name: &str,
        limit: usize,
    ) -> crate::Result<Vec<BudgetSnapshotRecord>> {
        let storage = terraphim_persistence::DeviceStorage::instance()
            .await
            .map_err(|e| crate::UsageError::StorageError(e.to_string()))?;
        let op = &storage.fastest_op;

        let prefix = format!(
            "usage/budgets/{}/",
            Persistable::normalize_key(
                &AgentMetricsRecord::new(agent_name.to_string()),
                agent_name
            )
        );

        let entries = op
            .list(&prefix)
            .await
            .map_err(|e| crate::UsageError::StorageError(e.to_string()))?;

        let mut snapshots = Vec::new();
        for entry in entries {
            let path: &str = entry.path();
            if path.ends_with(".json") {
                match Self::load_budget_from_path(op, path).await {
                    Ok(snapshot) => snapshots.push(snapshot),
                    Err(e) => eprintln!("Warning: Failed to load {}: {}", path, e),
                }
            }
        }

        #[allow(clippy::unnecessary_sort_by)]
        snapshots.sort_by(|a, b| b.snapshot_at.cmp(&a.snapshot_at));
        snapshots.truncate(limit);

        Ok(snapshots)
    }

    /// Get the latest provider usage snapshot.
    pub async fn get_provider_snapshot(
        &self,
        provider_id: &str,
    ) -> crate::Result<Option<ProviderUsageSnapshot>> {
        let storage = terraphim_persistence::DeviceStorage::instance()
            .await
            .map_err(|e| crate::UsageError::StorageError(e.to_string()))?;
        let op = &storage.fastest_op;

        let prefix = format!(
            "usage/providers/{}_",
            Persistable::normalize_key(
                &AgentMetricsRecord::new(provider_id.to_string()),
                provider_id
            )
        );

        let entries = op
            .list("usage/providers/")
            .await
            .map_err(|e| crate::UsageError::StorageError(e.to_string()))?;

        let mut latest: Option<ProviderUsageSnapshot> = None;
        for entry in entries {
            let path: &str = entry.path();
            if path.starts_with(&prefix) && path.ends_with(".json") {
                match Self::load_provider_snapshot_from_path(op, path).await {
                    Ok(snapshot) => {
                        if latest.is_none()
                            || snapshot.fetched_at > latest.as_ref().unwrap().fetched_at
                        {
                            latest = Some(snapshot);
                        }
                    }
                    Err(e) => eprintln!("Warning: Failed to load {}: {}", path, e),
                }
            }
        }

        Ok(latest)
    }

    /// Export all usage data to a serializable format.
    pub async fn export_usage_data(
        &self,
        since: &str,
        until: Option<&str>,
    ) -> crate::Result<UsageExport> {
        let metrics = self.list_agent_metrics().await?;
        let executions = self.query_executions(since, until, None).await?;

        Ok(UsageExport {
            exported_at: Utc::now().to_rfc3339(),
            since: since.to_string(),
            until: until.map(|s| s.to_string()),
            agent_metrics: metrics,
            executions,
        })
    }

    #[allow(dead_code)]
    async fn load_execution_from_path(
        op: &opendal::Operator,
        path: &str,
    ) -> crate::Result<ExecutionRecord> {
        let content = op
            .read(path)
            .await
            .map_err(|e| crate::UsageError::StorageError(e.to_string()))?;
        let exec: ExecutionRecord = serde_json::from_slice(&content.to_vec())
            .map_err(crate::UsageError::SerializationError)?;
        Ok(exec)
    }

    async fn load_budget_from_path(
        op: &opendal::Operator,
        path: &str,
    ) -> crate::Result<BudgetSnapshotRecord> {
        let content = op
            .read(path)
            .await
            .map_err(|e| crate::UsageError::StorageError(e.to_string()))?;
        let snapshot: BudgetSnapshotRecord = serde_json::from_slice(&content.to_vec())
            .map_err(crate::UsageError::SerializationError)?;
        Ok(snapshot)
    }

    async fn load_provider_snapshot_from_path(
        op: &opendal::Operator,
        path: &str,
    ) -> crate::Result<ProviderUsageSnapshot> {
        let content = op
            .read(path)
            .await
            .map_err(|e| crate::UsageError::StorageError(e.to_string()))?;
        let snapshot: ProviderUsageSnapshot = serde_json::from_slice(&content.to_vec())
            .map_err(crate::UsageError::SerializationError)?;
        Ok(snapshot)
    }
}

impl Default for UsageStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Exported usage data structure for JSON/CSV export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageExport {
    /// When the export was generated
    pub exported_at: String,
    /// Start of export period
    pub since: String,
    /// End of export period (if specified)
    pub until: Option<String>,
    /// Agent metrics
    pub agent_metrics: Vec<AgentMetricsRecord>,
    /// Execution records
    pub executions: Vec<ExecutionRecord>,
}

/// Alert configuration for budget thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Agent name (or "*" for all agents)
    pub agent_pattern: String,
    /// Threshold percentages that trigger alerts
    pub thresholds: Vec<u8>,
    /// Last alert sent for each threshold
    pub last_alerted: HashMap<u8, String>,
    /// Whether alerts are enabled
    pub enabled: bool,
}

impl AlertConfig {
    /// Create default alert config for an agent
    pub fn default_for_agent(agent_name: &str) -> Self {
        Self {
            agent_pattern: agent_name.to_string(),
            thresholds: vec![50, 80, 95, 100],
            last_alerted: HashMap::new(),
            enabled: true,
        }
    }

    /// Check if an alert should be sent for a given percentage
    pub fn should_alert(&self, percentage: f64) -> Option<u8> {
        if !self.enabled {
            return None;
        }

        let mut best: Option<u8> = None;
        for threshold in &self.thresholds {
            if percentage >= *threshold as f64 {
                if let Some(last_alerted) = self.last_alerted.get(threshold) {
                    if let Ok(last_dt) = DateTime::parse_from_rfc3339(last_alerted) {
                        let hours_since = (Utc::now().fixed_offset() - last_dt).num_hours();
                        if hours_since < 24 {
                            continue;
                        }
                    }
                }
                best = Some(*threshold);
            }
        }
        best
    }

    /// Mark a threshold as alerted
    pub fn mark_alerted(&mut self, threshold: u8) {
        self.last_alerted.insert(threshold, Utc::now().to_rfc3339());
    }
}

#[async_trait::async_trait]
impl Persistable for AlertConfig {
    fn new(key: String) -> Self {
        Self {
            agent_pattern: key.clone(),
            thresholds: vec![50, 80, 95, 100],
            last_alerted: HashMap::new(),
            enabled: true,
        }
    }

    async fn save(&self) -> terraphim_persistence::Result<()> {
        self.save_to_all().await
    }

    async fn save_to_one(&self, profile_name: &str) -> terraphim_persistence::Result<()> {
        self.save_to_profile(profile_name).await
    }

    async fn load(&mut self) -> terraphim_persistence::Result<Self> {
        let key = self.get_key();
        self.load_from_operator(&key, &self.load_config().await?.1)
            .await
    }

    fn get_key(&self) -> String {
        format!(
            "usage/alerts/{}.json",
            Persistable::normalize_key(
                &AgentMetricsRecord::new(self.agent_pattern.clone()),
                &self.agent_pattern
            )
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_metrics_record_execution() {
        let mut metrics = AgentMetricsRecord::new("test-agent".to_string());
        metrics.budget_monthly_cents = 10000; // $100 budget

        metrics.record_execution(100, 50, 50000, true); // 5 cents cost

        assert_eq!(metrics.total_input_tokens, 100);
        assert_eq!(metrics.total_output_tokens, 50);
        assert_eq!(metrics.total_tokens(), 150);
        assert_eq!(metrics.total_executions, 1);
        assert_eq!(metrics.successful_executions, 1);
        assert_eq!(metrics.total_cost_usd(), 0.05);
    }

    #[test]
    fn test_budget_percentage_calculation() {
        let mut metrics = AgentMetricsRecord::new("test-agent".to_string());
        metrics.budget_monthly_cents = 10000; // $100 budget
        metrics.total_cost_sub_cents = 50000000; // $50 spent (50 * 1_000_000 sub-cents)

        assert_eq!(metrics.budget_percentage_used(), 50.0);
    }

    #[test]
    fn test_budget_verdict() {
        let mut metrics = AgentMetricsRecord::new("test-agent".to_string());
        metrics.budget_monthly_cents = 10000; // $100 budget

        // Within budget (<80%)
        metrics.total_cost_sub_cents = 40000000; // $40
        let snapshot = BudgetSnapshotRecord::from_agent_metrics(&metrics);
        assert_eq!(snapshot.verdict, BudgetVerdict::WithinBudget);

        // Approaching limit (>= 80%)
        metrics.total_cost_sub_cents = 80000000; // $80
        let snapshot = BudgetSnapshotRecord::from_agent_metrics(&metrics);
        assert_eq!(snapshot.verdict, BudgetVerdict::ApproachingLimit);

        // Exceeded (>= 100%)
        metrics.total_cost_sub_cents = 100000000; // $100
        let snapshot = BudgetSnapshotRecord::from_agent_metrics(&metrics);
        assert_eq!(snapshot.verdict, BudgetVerdict::Exceeded);
    }

    #[test]
    fn test_alert_config_should_alert() {
        let config = AlertConfig::default_for_agent("test-agent");

        // Should alert at 80% threshold
        assert_eq!(config.should_alert(85.0), Some(80));

        // Should alert at 50% threshold
        assert_eq!(config.should_alert(55.0), Some(50));

        // Should not alert below lowest threshold
        assert_eq!(config.should_alert(45.0), None);
    }

    #[test]
    fn test_alert_config_respects_24h_cooldown() {
        let mut config = AlertConfig::default_for_agent("test-agent");

        // First alert should trigger
        assert!(config.should_alert(85.0).is_some());

        // Mark as alerted
        config.mark_alerted(80);

        // Should not alert again immediately (within 24h)
        assert_eq!(config.should_alert(85.0), Some(50)); // Only lower threshold
    }

    #[test]
    fn test_provider_snapshot_expiry() {
        let mut snapshot = ProviderUsageSnapshot::new("test".to_string());
        snapshot.fetched_at = Utc::now().to_rfc3339();
        snapshot.ttl_seconds = 300; // 5 minutes

        // Should not be expired immediately
        assert!(!snapshot.is_expired());

        // Set fetched_at to 10 minutes ago
        let old_time = Utc::now() - chrono::Duration::seconds(600);
        snapshot.fetched_at = old_time.to_rfc3339();

        // Should be expired now
        assert!(snapshot.is_expired());
    }

    #[test]
    fn test_execution_record_cost_calculation() {
        let exec = ExecutionRecord {
            key: "test".to_string(),
            agent_name: "test-agent".to_string(),
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 150,
            cost_sub_cents: 25000, // 2.5 cents
            model: Some("claude-3".to_string()),
            provider: Some("claude".to_string()),
            success: true,
            error_message: None,
            latency_ms: Some(1000),
            started_at: Utc::now().to_rfc3339(),
            completed_at: None,
            gitea_issue: None,
        };

        assert_eq!(exec.cost_usd(), 0.025);
    }
}
