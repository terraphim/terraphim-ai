use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::envelope::{MatrixResult, StepEnvelope};

/// Persisted state for a single flow run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowRunState {
    /// Name of the flow being run.
    pub flow_name: String,
    /// Unique correlation ID for this run instance.
    pub correlation_id: Uuid,
    /// Current execution status.
    pub status: FlowRunStatus,
    /// Timestamp when this run started.
    pub started_at: DateTime<Utc>,
    /// Timestamp when this run finished, if complete.
    pub finished_at: Option<DateTime<Utc>>,
    /// Index of the next step to execute on resume.
    pub next_step_index: usize,
    /// Optional issue id supplied by local flow context.
    #[serde(default)]
    pub issue: Option<String>,
    /// Ordered envelopes from completed steps.
    pub step_envelopes: Vec<StepEnvelope>,
    /// Results from matrix-expanded steps. Key is step name; value is the
    /// ordered list of sub-execution envelopes (one per matrix params row).
    #[serde(default)]
    pub matrix_envelopes: HashMap<String, Vec<StepEnvelope>>,
    /// Error message if the run failed.
    #[serde(default)]
    pub error: Option<String>,
    /// Current iteration count for re-iteration loops.
    /// Incremented each time a checkpoint with loop_target resumes.
    #[serde(default)]
    pub iteration_count: u32,
}

/// Current status of a flow run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowRunStatus {
    /// The flow is actively executing.
    Running,
    /// The flow is paused at a checkpoint.
    Paused,
    /// The flow completed all steps successfully.
    Completed,
    /// The flow terminated with an error.
    Failed,
    /// The flow was forcibly aborted.
    Aborted,
}

impl FlowRunState {
    /// Create a new flow run state in the `Running` status.
    pub fn new(flow_name: &str) -> Self {
        Self {
            flow_name: flow_name.to_string(),
            correlation_id: Uuid::new_v4(),
            status: FlowRunStatus::Running,
            started_at: Utc::now(),
            finished_at: None,
            next_step_index: 0,
            issue: None,
            step_envelopes: Vec::new(),
            matrix_envelopes: HashMap::new(),
            error: None,
            iteration_count: 0,
        }
    }

    /// Attach an issue ID to this flow run and return the updated state.
    pub fn with_issue(mut self, issue: String) -> Self {
        self.issue = Some(issue);
        self
    }

    /// Create a new flow run state pre-populated as `Failed` with the given reason.
    pub fn failed(flow_name: &str, reason: &str) -> Self {
        let mut state = Self::new(flow_name);
        state.status = FlowRunStatus::Failed;
        state.finished_at = Some(Utc::now());
        state.error = Some(reason.to_string());
        state
    }

    /// Return the envelope for the named step if it has already completed.
    pub fn step_output(&self, step_name: &str) -> Option<&StepEnvelope> {
        self.step_envelopes
            .iter()
            .find(|e| e.step_name == step_name)
    }

    /// Return aggregated results for a matrix step, or `None` if no matrix
    /// envelopes exist for that step name.
    pub fn matrix_result(&self, step_name: &str) -> Option<MatrixResult> {
        self.matrix_envelopes
            .get(step_name)
            .map(|envelopes| MatrixResult::from_envelopes(envelopes))
    }

    /// Atomically write the flow run state to a JSON file in `dir`.
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

    /// Load a flow run state from a JSON file at `path`.
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
        assert_eq!(state.issue, None);
        assert!(state.step_envelopes.is_empty());
        assert!(state.matrix_envelopes.is_empty());
        assert!(state.error.is_none());
        assert!(state.finished_at.is_none());

        // Correlation ID should be a valid UUID
        assert_ne!(state.correlation_id, Uuid::nil());
    }

    #[test]
    fn test_state_with_issue() {
        let state = FlowRunState::new("test-flow").with_issue("1890".to_string());
        assert_eq!(state.issue.as_deref(), Some("1890"));
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
    fn test_matrix_result_lookup() {
        let mut state = FlowRunState::new("test-flow");

        // Populate matrix envelopes for a step
        state.matrix_envelopes.insert(
            "run-model".to_string(),
            vec![
                create_test_envelope("run-model-matrix-0", 0),
                create_test_envelope("run-model-matrix-1", 1),
                create_test_envelope("run-model-matrix-2", 0),
            ],
        );

        let result = state.matrix_result("run-model").unwrap();
        assert_eq!(result.success_count, 2);
        assert_eq!(result.failure_count, 1);
        assert_eq!(result.all_exit_codes, "0,1,0");

        // Non-existent matrix step
        assert!(state.matrix_result("nonexistent").is_none());
    }

    #[test]
    fn test_state_save_load_roundtrip() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create a state with some data including matrix envelopes
        let mut original = FlowRunState::new("test-flow");
        original.next_step_index = 2;
        original
            .step_envelopes
            .push(create_test_envelope("gather-changes", 0));
        original
            .step_envelopes
            .push(create_test_envelope("analyze", 0));
        original.matrix_envelopes.insert(
            "run-model".to_string(),
            vec![
                create_test_envelope("run-model-matrix-0", 0),
                create_test_envelope("run-model-matrix-1", 0),
            ],
        );

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
        assert_eq!(loaded.matrix_envelopes.len(), 1);

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

    #[test]
    fn test_iteration_count_default() {
        let state = FlowRunState::new("test-flow");
        assert_eq!(state.iteration_count, 0);
    }

    #[test]
    fn test_iteration_count_serialization() {
        let mut state = FlowRunState::new("test-flow");
        state.iteration_count = 3;

        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("\"iteration_count\":3"));

        let deserialized: FlowRunState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.iteration_count, 3);
    }

    #[test]
    fn test_iteration_count_backward_compat() {
        // Old JSON without iteration_count should deserialize to 0
        let old_json = r#"{
            "flow_name": "test-flow",
            "correlation_id": "550e8400-e29b-41d4-a716-446655440000",
            "status": "running",
            "started_at": "2024-01-01T00:00:00Z",
            "finished_at": null,
            "next_step_index": 0,
            "issue": null,
            "step_envelopes": [],
            "matrix_envelopes": {},
            "error": null
        }"#;

        let state: FlowRunState = serde_json::from_str(old_json).unwrap();
        assert_eq!(state.iteration_count, 0);
    }

    #[test]
    fn test_iteration_count_roundtrip_in_save_load() {
        let temp_dir = tempfile::tempdir().unwrap();

        let mut original = FlowRunState::new("test-flow");
        original.iteration_count = 2;
        original.next_step_index = 5;

        let path = original.save_to_file(temp_dir.path()).unwrap();
        let loaded = FlowRunState::load_from_file(&path).unwrap();

        assert_eq!(loaded.iteration_count, 2);
        assert_eq!(loaded.next_step_index, 5);
    }
}
