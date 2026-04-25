use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::envelope::StepEnvelope;

/// Persistent state for a single in-progress or completed flow run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowRunState {
    pub flow_name: String,
    pub correlation_id: Uuid,
    pub status: FlowRunStatus,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub next_step_index: usize,
    pub step_envelopes: Vec<StepEnvelope>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Lifecycle status of a flow run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowRunStatus {
    Running,
    Paused,
    Completed,
    Failed,
    Aborted,
}

impl FlowRunState {
    pub fn new(flow_name: &str) -> Self {
        Self {
            flow_name: flow_name.to_string(),
            correlation_id: Uuid::new_v4(),
            status: FlowRunStatus::Running,
            started_at: Utc::now(),
            finished_at: None,
            next_step_index: 0,
            step_envelopes: Vec::new(),
            error: None,
        }
    }

    pub fn failed(flow_name: &str, reason: &str) -> Self {
        let mut state = Self::new(flow_name);
        state.status = FlowRunStatus::Failed;
        state.finished_at = Some(Utc::now());
        state.error = Some(reason.to_string());
        state
    }

    pub fn step_output(&self, step_name: &str) -> Option<&StepEnvelope> {
        self.step_envelopes
            .iter()
            .find(|e| e.step_name == step_name)
    }

    pub fn save_to_file(&self, dir: &Path) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(dir)?;
        let filename = format!("flow-{}-{}.json", self.flow_name, self.correlation_id);
        let path = dir.join(&filename);
        let tmp_path = dir.join(format!("{}.tmp", filename));
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(&tmp_path, &json)?;
        std::fs::rename(&tmp_path, &path)?;
        Ok(path)
    }

    pub fn load_from_file(path: &Path) -> std::io::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn create_test_envelope(name: &str, exit_code: i32) -> StepEnvelope {
        StepEnvelope {
            step_name: name.to_string(),
            started_at: Utc::now(),
            finished_at: Utc::now(),
            exit_code,
            stdout: format!("stdout for {}", name),
            stderr: format!("stderr for {}", name),
            cost_usd: None,
            session_id: None,
            input_tokens: None,
            output_tokens: None,
            stdout_file: None,
        }
    }

    #[test]
    fn test_state_new() {
        let state = FlowRunState::new("test-flow");

        assert_eq!(state.flow_name, "test-flow");
        assert_eq!(state.status, FlowRunStatus::Running);
        assert_eq!(state.next_step_index, 0);
        assert!(state.step_envelopes.is_empty());
        assert!(state.error.is_none());
        assert!(state.finished_at.is_none());

        // Correlation ID should be a valid UUID
        assert_ne!(state.correlation_id, Uuid::nil());
    }

    #[test]
    fn test_state_failed() {
        let state = FlowRunState::failed("test-flow", "Something went wrong");

        assert_eq!(state.flow_name, "test-flow");
        assert_eq!(state.status, FlowRunStatus::Failed);
        assert_eq!(state.error, Some("Something went wrong".to_string()));
        assert!(state.finished_at.is_some());
        assert_eq!(state.next_step_index, 0);
    }

    #[test]
    fn test_step_output_lookup() {
        let mut state = FlowRunState::new("test-flow");

        // Add some envelopes
        state.step_envelopes.push(create_test_envelope("step-1", 0));
        state.step_envelopes.push(create_test_envelope("step-2", 1));
        state.step_envelopes.push(create_test_envelope("step-3", 0));

        // Look up existing steps
        let step1 = state.step_output("step-1");
        assert!(step1.is_some());
        assert_eq!(step1.unwrap().exit_code, 0);

        let step2 = state.step_output("step-2");
        assert!(step2.is_some());
        assert_eq!(step2.unwrap().exit_code, 1);

        // Look up non-existent step
        let missing = state.step_output("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_state_save_load_roundtrip() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create a state with some data
        let mut original = FlowRunState::new("test-flow");
        original.next_step_index = 2;
        original
            .step_envelopes
            .push(create_test_envelope("gather-changes", 0));
        original
            .step_envelopes
            .push(create_test_envelope("analyze", 0));

        // Save to file
        let path = original.save_to_file(temp_dir.path()).unwrap();

        // Verify file exists
        assert!(path.exists());
        assert!(path.to_string_lossy().contains("flow-test-flow-"));

        // Load back
        let loaded = FlowRunState::load_from_file(&path).unwrap();

        // Verify all fields
        assert_eq!(loaded.flow_name, original.flow_name);
        assert_eq!(loaded.correlation_id, original.correlation_id);
        assert_eq!(loaded.status, original.status);
        assert_eq!(loaded.next_step_index, original.next_step_index);
        assert_eq!(loaded.step_envelopes.len(), original.step_envelopes.len());
        assert_eq!(loaded.error, original.error);

        // Verify envelope data
        assert_eq!(loaded.step_envelopes[0].step_name, "gather-changes");
        assert_eq!(loaded.step_envelopes[1].step_name, "analyze");
    }

    #[test]
    fn test_state_save_load_with_error() {
        let temp_dir = tempfile::tempdir().unwrap();

        let original = FlowRunState::failed("test-flow", "Connection timeout");

        let path = original.save_to_file(temp_dir.path()).unwrap();
        let loaded = FlowRunState::load_from_file(&path).unwrap();

        assert_eq!(loaded.flow_name, "test-flow");
        assert_eq!(loaded.status, FlowRunStatus::Failed);
        assert_eq!(loaded.error, Some("Connection timeout".to_string()));
        assert!(loaded.finished_at.is_some());
    }

    #[test]
    fn test_state_load_invalid_json() {
        let temp_dir = tempfile::tempdir().unwrap();
        let invalid_path = temp_dir.path().join("invalid.json");

        // Write invalid JSON
        std::fs::write(&invalid_path, "not valid json").unwrap();

        // Attempt to load should fail
        let result = FlowRunState::load_from_file(&invalid_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_state_load_nonexistent_file() {
        let result = FlowRunState::load_from_file(Path::new("/nonexistent/path/flow.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_flow_run_status_variants() {
        // Test that all variants serialize/deserialize correctly
        let statuses = vec![
            FlowRunStatus::Running,
            FlowRunStatus::Paused,
            FlowRunStatus::Completed,
            FlowRunStatus::Failed,
            FlowRunStatus::Aborted,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: FlowRunStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, deserialized);
        }

        // Test snake_case serialization
        assert_eq!(
            serde_json::to_string(&FlowRunStatus::Running).unwrap(),
            "\"running\""
        );
        assert_eq!(
            serde_json::to_string(&FlowRunStatus::Paused).unwrap(),
            "\"paused\""
        );
        assert_eq!(
            serde_json::to_string(&FlowRunStatus::Completed).unwrap(),
            "\"completed\""
        );
        assert_eq!(
            serde_json::to_string(&FlowRunStatus::Failed).unwrap(),
            "\"failed\""
        );
        assert_eq!(
            serde_json::to_string(&FlowRunStatus::Aborted).unwrap(),
            "\"aborted\""
        );
    }

    #[test]
    fn test_state_timestamp_ordering() {
        let state1 = FlowRunState::new("flow-1");
        std::thread::sleep(Duration::from_millis(10));
        let state2 = FlowRunState::new("flow-2");

        // state2 should have a later timestamp
        assert!(state2.started_at >= state1.started_at);
    }

    #[test]
    fn test_correlation_id_uniqueness() {
        let ids: Vec<Uuid> = (0..100)
            .map(|_| FlowRunState::new("test").correlation_id)
            .collect();

        // All IDs should be unique
        let unique_count = ids.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(unique_count, ids.len());
    }
}
