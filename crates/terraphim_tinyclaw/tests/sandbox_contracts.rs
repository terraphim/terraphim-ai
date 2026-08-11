//! Hermetic contract tests for `SandboxTool` (#3146).
//!
//! Uses `TerraphimRlm::with_executor` with a mock executor so no real
//! backend (docker/firecracker) is needed in CI. The mock implements the
//! `ExecutionEnvironment` trait and records calls.

use serde_json::json;
use std::sync::atomic::{AtomicU32, Ordering};
use terraphim_rlm::config::{BackendType, RlmConfig};
use terraphim_rlm::error::RlmError;
use terraphim_rlm::executor::{
    Capability, ExecutionContext, ExecutionEnvironment, ExecutionResult, SnapshotId,
    ValidationResult,
};
use terraphim_rlm::rlm::TerraphimRlm;
use terraphim_rlm::types::SessionId;
use terraphim_tinyclaw::tools::sandbox::{SandboxTool, SandboxToolConfig};
use terraphim_tinyclaw::tools::{Tool, ToolError};

/// Mock executor: records calls, returns canned results.
#[derive(Debug, Default)]
struct MockExecutor {
    code_calls: AtomicU32,
    bash_calls: AtomicU32,
}

#[async_trait::async_trait]
impl ExecutionEnvironment for MockExecutor {
    type Error = RlmError;

    async fn execute_code(
        &self,
        code: &str,
        _ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, RlmError> {
        self.code_calls.fetch_add(1, Ordering::SeqCst);
        Ok(ExecutionResult {
            stdout: format!("mock-out: {code}"),
            stderr: String::new(),
            exit_code: 0,
            execution_time_ms: 1,
            output_truncated: false,
            output_file_path: None,
            timed_out: false,
            metadata: Default::default(),
        })
    }

    async fn execute_command(
        &self,
        cmd: &str,
        _ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, RlmError> {
        self.bash_calls.fetch_add(1, Ordering::SeqCst);
        Ok(ExecutionResult {
            stdout: format!("mock-shell: {cmd}"),
            stderr: String::new(),
            exit_code: 0,
            execution_time_ms: 1,
            output_truncated: false,
            output_file_path: None,
            timed_out: false,
            metadata: Default::default(),
        })
    }

    async fn validate(&self, _input: &str) -> Result<ValidationResult, RlmError> {
        Ok(ValidationResult {
            is_valid: true,
            matched_terms: vec![],
            unknown_terms: vec![],
            suggestions: Default::default(),
            strictness: Default::default(),
            message: "mock validation passes".to_string(),
            retry_count: 0,
            escalation_required: false,
        })
    }

    async fn create_snapshot(
        &self,
        session_id: &SessionId,
        name: &str,
    ) -> Result<SnapshotId, RlmError> {
        Ok(SnapshotId::new(name, *session_id))
    }

    async fn restore_snapshot(&self, _id: &SnapshotId) -> Result<(), RlmError> {
        Ok(())
    }

    async fn list_snapshots(&self, _session_id: &SessionId) -> Result<Vec<SnapshotId>, RlmError> {
        Ok(vec![])
    }

    async fn delete_snapshot(&self, _id: &SnapshotId) -> Result<(), RlmError> {
        Ok(())
    }

    async fn delete_session_snapshots(&self, _session_id: &SessionId) -> Result<(), RlmError> {
        Ok(())
    }

    fn capabilities(&self) -> &[Capability] {
        &[]
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Local
    }

    async fn health_check(&self) -> Result<bool, RlmError> {
        Ok(true)
    }

    async fn cleanup(&self) -> Result<(), RlmError> {
        Ok(())
    }
}

fn make_tool() -> SandboxTool {
    let mock = MockExecutor::default();
    let rlm = TerraphimRlm::with_executor(RlmConfig::default(), mock).expect("rlm builds");
    let cfg = SandboxToolConfig {
        backend: "local".to_string(),
        timeout_secs: 30,
        max_output_bytes: 1024,
    };
    SandboxTool::new(rlm, cfg)
}

#[tokio::test]
async fn sandbox_execute_code_returns_result() {
    let tool = make_tool();
    let out = tool
        .execute(json!({"op": "execute_code", "code": "print('hi')"}))
        .await
        .expect("execute_code should succeed");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["op"], "execute_code");
    assert_eq!(v["success"], true);
    assert!(v["stdout"].as_str().unwrap().contains("mock-out"));
    assert!(v["session_id"].as_str().unwrap().len() > 10);
}

#[tokio::test]
async fn sandbox_execute_bash_returns_result() {
    let tool = make_tool();
    let out = tool
        .execute(json!({"op": "execute_bash", "command": "echo hi"}))
        .await
        .expect("execute_bash should succeed");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["success"], true);
    assert!(v["stdout"].as_str().unwrap().contains("mock-shell"));
}

#[tokio::test]
async fn sandbox_auto_creates_and_reuses_session() {
    let tool = make_tool();
    let a = tool
        .execute(json!({"op": "execute_bash", "command": "one"}))
        .await
        .unwrap();
    let b = tool
        .execute(json!({"op": "execute_bash", "command": "two"}))
        .await
        .unwrap();
    let va: serde_json::Value = serde_json::from_str(&a).unwrap();
    let vb: serde_json::Value = serde_json::from_str(&b).unwrap();
    assert_eq!(va["session_id"], vb["session_id"], "default session reused");
}

#[tokio::test]
async fn sandbox_session_lifecycle() {
    let tool = make_tool();
    let created = tool
        .execute(json!({"op": "session_create"}))
        .await
        .expect("session_create");
    let vc: serde_json::Value = serde_json::from_str(&created).unwrap();
    let sid = vc["session_id"].as_str().unwrap().to_string();

    let status = tool
        .execute(json!({"op": "session_status", "session_id": sid}))
        .await
        .expect("session_status");
    let vs: serde_json::Value = serde_json::from_str(&status).unwrap();
    assert_eq!(vs["session_id"], sid);

    let destroyed = tool
        .execute(json!({"op": "session_destroy", "session_id": sid}))
        .await
        .expect("session_destroy");
    let vd: serde_json::Value = serde_json::from_str(&destroyed).unwrap();
    assert_eq!(vd["destroyed"], true);
}

#[tokio::test]
async fn sandbox_unknown_op_and_missing_args() {
    let tool = make_tool();
    let err = tool
        .execute(json!({"op": "nope"}))
        .await
        .expect_err("unknown op must fail");
    assert!(matches!(err, ToolError::InvalidArguments { .. }));

    let err2 = tool
        .execute(json!({"op": "execute_code"}))
        .await
        .expect_err("missing code must fail");
    assert!(matches!(err2, ToolError::InvalidArguments { .. }));
}

#[tokio::test]
async fn sandbox_invalid_session_id_rejected() {
    let tool = make_tool();
    let err = tool
        .execute(json!({"op": "session_status", "session_id": "not-a-ulid"}))
        .await
        .expect_err("bad session id must fail");
    assert!(matches!(err, ToolError::InvalidArguments { .. }));
}
