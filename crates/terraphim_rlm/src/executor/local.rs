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
use std::path::PathBuf;
use std::process::Stdio;
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

pub struct LocalExecutor {
    python_path: String,
    output_dir: PathBuf,
}

impl LocalExecutor {
    pub fn new() -> Self {
        Self {
            python_path: "python3".to_string(),
            output_dir: std::env::temp_dir().join("terraphim_rlm_local"),
        }
    }

    pub fn with_python(mut self, path: impl Into<String>) -> Self {
        self.python_path = path.into();
        self
    }

    pub fn with_output_dir(mut self, path: PathBuf) -> Self {
        self.output_dir = path;
        self
    }

    fn build_command(&self, cmd: &str, ctx: &ExecutionContext) -> Command {
        let mut command = Command::new("bash");
        command
            .arg("-c")
            .arg(cmd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(&ctx.env_vars);

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
            .envs(&ctx.env_vars);

        if let Some(cwd) = &ctx.working_dir {
            command.current_dir(cwd);
        }

        command
    }

    async fn run_command(&self, mut cmd: Command) -> RlmResult<ExecutionResult> {
        let start = Instant::now();

        let output = timeout(Duration::from_millis(30000), cmd.output()).await;

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
                stderr: "Execution timed out after 30 seconds".to_string(),
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

#[async_trait]
impl ExecutionEnvironment for LocalExecutor {
    type Error = RlmError;

    async fn execute_code(
        &self,
        code: &str,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, Self::Error> {
        let cmd = self.build_python_command(code, ctx);
        self.run_command(cmd).await
    }

    async fn execute_command(
        &self,
        cmd: &str,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, Self::Error> {
        let command = self.build_command(cmd, ctx);
        self.run_command(command).await
    }

    async fn validate(&self, _input: &str) -> Result<ValidationResult, Self::Error> {
        Ok(ValidationResult::valid(vec![]))
    }

    async fn create_snapshot(
        &self,
        _session_id: &SessionId,
        _name: &str,
    ) -> Result<SnapshotId, Self::Error> {
        Ok(SnapshotId::new(_name, _session_id.clone()))
    }

    async fn restore_snapshot(&self, _id: &SnapshotId) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn list_snapshots(
        &self,
        _session_id: &SessionId,
    ) -> Result<Vec<SnapshotId>, Self::Error> {
        Ok(vec![])
    }

    async fn delete_snapshot(&self, _id: &SnapshotId) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn delete_session_snapshots(&self, _session_id: &SessionId) -> Result<(), Self::Error> {
        Ok(())
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
}
