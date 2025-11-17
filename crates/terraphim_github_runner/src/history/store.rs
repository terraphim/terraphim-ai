//! Execution history storage

use crate::{ExecutionRecord, InterpretedAction, RunnerResult, RunnerError, StepResult};
use ahash::AHashMap;
use chrono::Utc;

/// Storage for execution history
#[derive(Debug, Clone)]
pub struct ExecutionHistory {
    /// Records by job ID
    records: AHashMap<String, Vec<ExecutionRecord>>,
    /// Records by action (for pattern matching)
    by_action: AHashMap<String, Vec<String>>,
    /// Success count by action
    success_count: AHashMap<String, usize>,
    /// Total execution count by action
    total_count: AHashMap<String, usize>,
}

impl ExecutionHistory {
    /// Create a new execution history
    pub fn new() -> Self {
        Self {
            records: AHashMap::new(),
            by_action: AHashMap::new(),
            success_count: AHashMap::new(),
            total_count: AHashMap::new(),
        }
    }

    /// Record a step execution
    pub async fn record_step(
        &mut self,
        job_id: &str,
        step_index: usize,
        interpreted: &InterpretedAction,
        result: &StepResult,
    ) -> RunnerResult<()> {
        let record = ExecutionRecord {
            id: uuid::Uuid::new_v4().to_string(),
            workflow_id: String::new(), // Would be set from job context
            job_id: job_id.to_string(),
            step_index,
            action: interpreted.original.clone(),
            interpreted_commands: interpreted.commands.clone(),
            vm_snapshot_id: result.vm_snapshot_id.clone(),
            duration_ms: result.duration_ms,
            exit_code: result.exit_code,
            stdout_hash: format!("{:x}", result.stdout.len()),
            artifacts_produced: result.artifacts.clone(),
            kg_context: interpreted.kg_terms.clone(),
            timestamp: Utc::now(),
            repository: String::new(), // Would be set from job context
            branch: String::new(),     // Would be set from job context
        };

        // Store by job
        self.records
            .entry(job_id.to_string())
            .or_default()
            .push(record.clone());

        // Index by action
        self.by_action
            .entry(interpreted.original.clone())
            .or_default()
            .push(record.id.clone());

        // Update statistics
        *self.total_count.entry(interpreted.original.clone()).or_insert(0) += 1;
        if result.exit_code == 0 {
            *self.success_count.entry(interpreted.original.clone()).or_insert(0) += 1;
        }

        Ok(())
    }

    /// Get records for a job
    pub fn get_job_records(&self, job_id: &str) -> Option<&Vec<ExecutionRecord>> {
        self.records.get(job_id)
    }

    /// Get all records for an action
    pub fn get_action_records(&self, action: &str) -> Vec<&ExecutionRecord> {
        let record_ids = match self.by_action.get(action) {
            Some(ids) => ids,
            None => return Vec::new(),
        };

        let mut results = Vec::new();
        for records in self.records.values() {
            for record in records {
                if record_ids.contains(&record.id) {
                    results.push(record);
                }
            }
        }
        results
    }

    /// Get success rate for an action
    pub fn get_success_rate(&self, action: &str) -> f64 {
        let total = *self.total_count.get(action).unwrap_or(&0);
        if total == 0 {
            return 0.0;
        }

        let success = *self.success_count.get(action).unwrap_or(&0);
        success as f64 / total as f64
    }

    /// Get average duration for an action
    pub fn get_average_duration(&self, action: &str) -> Option<u64> {
        let records = self.get_action_records(action);
        if records.is_empty() {
            return None;
        }

        let total: u64 = records.iter().map(|r| r.duration_ms).sum();
        Some(total / records.len() as u64)
    }

    /// Get recent successful execution for caching
    pub fn get_cached_execution(&self, action: &str, context_hash: &str) -> Option<&ExecutionRecord> {
        let records = self.get_action_records(action);

        // Find most recent successful execution with matching context
        records
            .into_iter()
            .filter(|r| r.exit_code == 0)
            .filter(|r| {
                // Simple context matching - would be more sophisticated in real impl
                r.stdout_hash == context_hash || r.vm_snapshot_id.is_some()
            })
            .max_by_key(|r| r.timestamp)
    }

    /// Get statistics
    pub fn statistics(&self) -> HistoryStatistics {
        let total_executions: usize = self.total_count.values().sum();
        let total_successes: usize = self.success_count.values().sum();

        let mut most_executed = None;
        let mut most_failed = None;

        if !self.total_count.is_empty() {
            most_executed = self.total_count
                .iter()
                .max_by_key(|(_, count)| *count)
                .map(|(action, _)| action.clone());

            // Find action with highest failure count
            let failures: AHashMap<_, _> = self.total_count
                .iter()
                .map(|(action, total)| {
                    let success = self.success_count.get(action).unwrap_or(&0);
                    (action.clone(), total - success)
                })
                .collect();

            most_failed = failures
                .iter()
                .max_by_key(|(_, count)| *count)
                .filter(|(_, count)| **count > 0)
                .map(|(action, _)| action.clone());
        }

        HistoryStatistics {
            total_executions,
            total_successes,
            total_failures: total_executions - total_successes,
            unique_actions: self.total_count.len(),
            jobs_recorded: self.records.len(),
            most_executed,
            most_failed,
        }
    }

    /// Clear old records
    pub fn cleanup(&mut self, max_age_days: u64) {
        let cutoff = Utc::now() - chrono::Duration::days(max_age_days as i64);

        for records in self.records.values_mut() {
            records.retain(|r| r.timestamp >= cutoff);
        }

        // Remove empty jobs
        self.records.retain(|_, records| !records.is_empty());

        // Rebuild indexes
        self.rebuild_indexes();
    }

    /// Rebuild internal indexes
    fn rebuild_indexes(&mut self) {
        self.by_action.clear();
        self.success_count.clear();
        self.total_count.clear();

        for records in self.records.values() {
            for record in records {
                self.by_action
                    .entry(record.action.clone())
                    .or_default()
                    .push(record.id.clone());

                *self.total_count.entry(record.action.clone()).or_insert(0) += 1;
                if record.exit_code == 0 {
                    *self.success_count.entry(record.action.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    /// Export history to JSON
    pub fn to_json(&self) -> RunnerResult<String> {
        let all_records: Vec<_> = self.records.values().flatten().collect();
        serde_json::to_string_pretty(&all_records)
            .map_err(|e| RunnerError::HistoryStorage(e.to_string()))
    }
}

impl Default for ExecutionHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about execution history
#[derive(Debug, Clone)]
pub struct HistoryStatistics {
    /// Total number of executions
    pub total_executions: usize,
    /// Total successful executions
    pub total_successes: usize,
    /// Total failed executions
    pub total_failures: usize,
    /// Number of unique actions
    pub unique_actions: usize,
    /// Number of jobs recorded
    pub jobs_recorded: usize,
    /// Most frequently executed action
    pub most_executed: Option<String>,
    /// Action with most failures
    pub most_failed: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ArtifactRef;

    #[tokio::test]
    async fn test_record_and_retrieve() {
        let mut history = ExecutionHistory::new();

        let interpreted = InterpretedAction {
            original: "npm ci".to_string(),
            purpose: "Install deps".to_string(),
            prerequisites: Vec::new(),
            produces: Vec::new(),
            cacheable: true,
            commands: vec!["npm ci".to_string()],
            required_env: Vec::new(),
            kg_terms: Vec::new(),
            confidence: 0.9,
        };

        let result = StepResult {
            step_id: "step-0".to_string(),
            exit_code: 0,
            stdout: "installed".to_string(),
            stderr: String::new(),
            duration_ms: 1000,
            outputs: AHashMap::new(),
            artifacts: Vec::new(),
            vm_snapshot_id: None,
        };

        history.record_step("job-1", 0, &interpreted, &result).await.unwrap();

        // Verify records
        let records = history.get_job_records("job-1").unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].action, "npm ci");

        // Verify statistics
        let stats = history.statistics();
        assert_eq!(stats.total_executions, 1);
        assert_eq!(stats.total_successes, 1);
    }

    #[tokio::test]
    async fn test_success_rate() {
        let mut history = ExecutionHistory::new();

        let interpreted = InterpretedAction {
            original: "test".to_string(),
            purpose: "Test".to_string(),
            prerequisites: Vec::new(),
            produces: Vec::new(),
            cacheable: false,
            commands: Vec::new(),
            required_env: Vec::new(),
            kg_terms: Vec::new(),
            confidence: 0.9,
        };

        // Record 2 successes and 1 failure
        for i in 0..3 {
            let result = StepResult {
                step_id: format!("step-{}", i),
                exit_code: if i < 2 { 0 } else { 1 },
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 100,
                outputs: AHashMap::new(),
                artifacts: Vec::new(),
                vm_snapshot_id: None,
            };
            history.record_step(&format!("job-{}", i), 0, &interpreted, &result).await.unwrap();
        }

        let rate = history.get_success_rate("test");
        assert!((rate - 0.666).abs() < 0.01);
    }
}
