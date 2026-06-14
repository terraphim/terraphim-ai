//! Local process execution backend for RLM.
//!
//! This backend executes code and commands directly in the local process,
//! bypassing isolation for faster execution when isolation is not required.
//!
//! # Warning
//!
//! This backend provides NO ISOLATION. Code runs directly on the host system
//! with the same permissions as the RLM process. Use only when:
//! - Isolation is not required (trusted code)
//! - Development/testing environments
//! - Quick prototyping without VM overhead

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::process::Command;
use tokio::time::timeout;

use crate::config::BackendType;
use crate::error::{RlmError, RlmResult};
use crate::executor::ExecutionEnvironment;
use crate::executor::context::{
    Capability, ExecutionContext, ExecutionResult, SnapshotId, ValidationResult,
};
use crate::types::SessionId;
use crate::validator::KnowledgeGraphValidator;

const BACKEND_NAME: &str = "local";

pub struct LocalExecutor {
    python_path: String,
    validator: Option<Arc<KnowledgeGraphValidator>>,
}

impl LocalExecutor {
    pub fn new() -> Self {
        Self {
            python_path: "python3".to_string(),
            validator: None,
        }
    }

    /// Create a LocalExecutor with a knowledge graph validator.
    pub fn with_validator(mut self, validator: Option<Arc<KnowledgeGraphValidator>>) -> Self {
        self.validator = validator;
        self
    }

    pub fn with_python(mut self, path: impl Into<String>) -> Self {
        self.python_path = path.into();
        self
    }

    fn build_command(&self, cmd: &str, ctx: &ExecutionContext) -> Command {
        let mut command = Command::new("bash");
        command
            .arg("-c")
            .arg(cmd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(&ctx.env_vars)
            .kill_on_drop(true);

        if let Some(cwd) = &ctx.working_dir {
            command.current_dir(cwd);
        }

        command
    }

    fn build_python_command(&self, code: &str, ctx: &ExecutionContext) -> Command {
        let mut command = Command::new(&self.python_path);
        command
            .arg("-c")
            .arg(code)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(&ctx.env_vars)
            .kill_on_drop(true);

        if let Some(cwd) = &ctx.working_dir {
            command.current_dir(cwd);
        }

        command
    }

    async fn run_command(
        &self,
        mut cmd: Command,
        ctx: &ExecutionContext,
    ) -> RlmResult<ExecutionResult> {
        let start = Instant::now();
        let timeout_duration = Duration::from_millis(ctx.timeout_ms);
        let output = timeout(timeout_duration, cmd.output()).await;

        let execution_time_ms = start.elapsed().as_millis() as u64;

        match output {
            Ok(Ok(output)) => Ok(ExecutionResult {
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: output.status.code().unwrap_or(-1),
                execution_time_ms,
                output_truncated: false,
                output_file_path: None,
                timed_out: false,
                metadata: HashMap::new(),
            }),
            Ok(Err(e)) => Err(RlmError::ExecutionFailed {
                message: format!("Failed to execute: {}", e),
                exit_code: None,
                stdout: None,
                stderr: None,
            }),
            Err(_) => Ok(ExecutionResult {
                stdout: String::new(),
                stderr: format!("Execution timed out after {}ms", ctx.timeout_ms),
                exit_code: -1,
                execution_time_ms,
                output_truncated: false,
                output_file_path: None,
                timed_out: true,
                metadata: HashMap::new(),
            }),
        }
    }
}

impl Default for LocalExecutor {
    fn default() -> Self {
        Self::new()
    }
}

fn unsupported(op: &'static str) -> RlmError {
    RlmError::NotSupported {
        backend: BACKEND_NAME.to_string(),
        op: op.to_string(),
    }
}

#[async_trait]
impl ExecutionEnvironment for LocalExecutor {
    type Error = RlmError;

    async fn execute_code(
        &self,
        code: &str,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, Self::Error> {
        let cmd = self.build_python_command(code, ctx);
        self.run_command(cmd, ctx).await
    }

    async fn execute_command(
        &self,
        cmd: &str,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, Self::Error> {
        let command = self.build_command(cmd, ctx);
        self.run_command(command, ctx).await
    }

    async fn validate(&self, input: &str) -> Result<ValidationResult, Self::Error> {
        match self.validator.as_ref() {
            Some(validator) if !input.trim().is_empty() => {
                let vr = validator.validate(input)?;
                Ok(ValidationResult::from_validator_result(
                    &vr,
                    crate::config::KgStrictness::Normal,
                ))
            }
            _ => Ok(ValidationResult::valid(Vec::new())),
        }
    }

    async fn create_snapshot(
        &self,
        _session_id: &SessionId,
        _name: &str,
    ) -> Result<SnapshotId, Self::Error> {
        Err(unsupported("create_snapshot"))
    }

    async fn restore_snapshot(&self, _id: &SnapshotId) -> Result<(), Self::Error> {
        Err(unsupported("restore_snapshot"))
    }

    async fn list_snapshots(
        &self,
        _session_id: &SessionId,
    ) -> Result<Vec<SnapshotId>, Self::Error> {
        Err(unsupported("list_snapshots"))
    }

    async fn delete_snapshot(&self, _id: &SnapshotId) -> Result<(), Self::Error> {
        Err(unsupported("delete_snapshot"))
    }

    async fn delete_session_snapshots(&self, _session_id: &SessionId) -> Result<(), Self::Error> {
        Err(unsupported("delete_session_snapshots"))
    }

    fn capabilities(&self) -> &[Capability] {
        &[
            Capability::BashExecution,
            Capability::PythonExecution,
            Capability::FileOperations,
        ]
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Local
    }

    async fn health_check(&self) -> Result<bool, Self::Error> {
        match Command::new(&self.python_path)
            .arg("--version")
            .output()
            .await
        {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    async fn cleanup(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_execute_command() {
        let executor = LocalExecutor::new();
        let ctx = ExecutionContext::default();

        let result = executor
            .execute_command("echo 'hello world'", &ctx)
            .await
            .unwrap();
        assert!(result.is_success());
        assert_eq!(result.stdout.trim(), "hello world");
    }

    #[tokio::test]
    async fn test_local_execute_python() {
        let executor = LocalExecutor::new();
        let ctx = ExecutionContext::default();

        let result = executor
            .execute_code("print('Hello from Python')", &ctx)
            .await
            .unwrap();
        assert!(result.is_success());
        assert!(result.stdout.contains("Hello from Python"));
    }

    #[tokio::test]
    async fn test_local_command_failure() {
        let executor = LocalExecutor::new();
        let ctx = ExecutionContext::default();

        let result = executor.execute_command("exit 1", &ctx).await.unwrap();
        assert!(!result.is_success());
        assert_eq!(result.exit_code, 1);
    }

    #[tokio::test]
    async fn test_local_honours_ctx_timeout() {
        let executor = LocalExecutor::new();
        let ctx = ExecutionContext {
            timeout_ms: 200,
            ..Default::default()
        };

        let start = Instant::now();
        let result = executor.execute_command("sleep 5", &ctx).await.unwrap();
        let elapsed = start.elapsed();

        assert!(result.timed_out, "expected timed_out=true");
        assert_eq!(result.exit_code, -1);
        assert!(
            elapsed < Duration::from_millis(1500),
            "execution should respect ctx.timeout_ms (200ms), took {:?}",
            elapsed
        );
        assert!(result.stderr.contains("200ms"));
    }

    #[tokio::test]
    async fn test_local_kills_on_timeout() {
        // The presence of `kill_on_drop(true)` in build_command means the spawned
        // child is reaped when the future running it is dropped on timeout.
        // We assert this by giving the snippet a unique marker, timing out, and
        // polling pgrep with bounded backoff so the test stays deterministic
        // on loaded CI (kernel reaping is fast but not instantaneous).
        let executor = LocalExecutor::new();
        let ctx = ExecutionContext {
            timeout_ms: 100,
            ..Default::default()
        };
        let marker = format!(
            "terraphim-rlm-marker-{}-{}",
            std::process::id(),
            ulid::Ulid::new()
        );

        let _ = executor
            .execute_command(&format!("sleep 30 && echo {}", marker), &ctx)
            .await
            .unwrap();

        // Poll up to 2s for the marker process to disappear (50 ms steps).
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let pgrep = std::process::Command::new("pgrep")
                .args(["-f", &marker])
                .output();
            let still_alive = match pgrep {
                Ok(o) => o.status.success(),
                Err(_) => break, // pgrep absent: cannot verify, accept.
            };
            if !still_alive {
                return;
            }
            if Instant::now() >= deadline {
                let leftover = std::process::Command::new("pgrep")
                    .args(["-f", &marker])
                    .output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                    .unwrap_or_default();
                panic!(
                    "child process leaked after timeout: pgrep still finds '{}'",
                    leftover
                );
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    #[tokio::test]
    async fn test_local_snapshot_returns_not_supported() {
        let executor = LocalExecutor::new();
        let session = SessionId::new();

        let create = executor.create_snapshot(&session, "x").await;
        assert!(matches!(create, Err(RlmError::NotSupported { .. })));

        let list = executor.list_snapshots(&session).await;
        assert!(matches!(list, Err(RlmError::NotSupported { .. })));

        let delete_session = executor.delete_session_snapshots(&session).await;
        assert!(matches!(delete_session, Err(RlmError::NotSupported { .. })));
    }

    #[tokio::test]
    async fn test_local_end_session_default_is_ok() {
        // LocalExecutor has no per-session resources; default trait impl of
        // end_session should return Ok.
        let executor = LocalExecutor::new();
        let session = SessionId::new();
        assert!(executor.end_session(&session).await.is_ok());
    }
}
