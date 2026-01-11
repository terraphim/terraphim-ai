//! SSH-based command execution for VMs.
//!
//! This module provides SSH-based execution capabilities for running commands
//! and code inside VMs that have been allocated from the pool.
//!
//! ## Features
//!
//! - Command execution via SSH
//! - Python code execution
//! - Output capture with size limits
//! - Timeout handling
//! - Output streaming to file for large outputs

use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use crate::error::{RlmError, RlmResult};

use super::context::{ExecutionContext, ExecutionResult};

/// SSH executor for running commands on VMs.
pub struct SshExecutor {
    /// Default SSH user for VM connections.
    default_user: String,
    /// SSH private key path.
    private_key_path: Option<PathBuf>,
    /// SSH connection timeout in milliseconds.
    connect_timeout_ms: u64,
    /// Directory for streaming large outputs.
    output_dir: PathBuf,
}

impl SshExecutor {
    /// Create a new SSH executor.
    pub fn new() -> Self {
        Self {
            default_user: "root".to_string(),
            private_key_path: None,
            connect_timeout_ms: 5000,
            output_dir: std::env::temp_dir().join("terraphim_rlm_output"),
        }
    }

    /// Set the default SSH user.
    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.default_user = user.into();
        self
    }

    /// Set the SSH private key path.
    pub fn with_private_key(mut self, path: impl Into<PathBuf>) -> Self {
        self.private_key_path = Some(path.into());
        self
    }

    /// Set the output directory for large outputs.
    pub fn with_output_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.output_dir = path.into();
        self
    }

    /// Build SSH command arguments.
    fn build_ssh_args(&self, host: &str, user: Option<&str>) -> Vec<String> {
        let mut args = vec![
            "-o".to_string(),
            "StrictHostKeyChecking=no".to_string(),
            "-o".to_string(),
            "UserKnownHostsFile=/dev/null".to_string(),
            "-o".to_string(),
            format!("ConnectTimeout={}", self.connect_timeout_ms / 1000),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
        ];

        if let Some(key_path) = &self.private_key_path {
            args.push("-i".to_string());
            args.push(key_path.to_string_lossy().to_string());
        }

        let ssh_user = user.unwrap_or(&self.default_user);
        args.push(format!("{}@{}", ssh_user, host));

        args
    }

    /// Execute a bash command on a remote VM.
    ///
    /// # Arguments
    ///
    /// * `host` - VM IP address
    /// * `command` - Command to execute
    /// * `ctx` - Execution context
    ///
    /// # Returns
    ///
    /// Execution result with stdout, stderr, and exit code.
    pub async fn execute_command(
        &self,
        host: &str,
        command: &str,
        ctx: &ExecutionContext,
    ) -> RlmResult<ExecutionResult> {
        let start = Instant::now();

        log::debug!(
            "SSH execute_command on {}: {}",
            host,
            if command.len() > 100 {
                format!("{}...", &command[..100])
            } else {
                command.to_string()
            }
        );

        let mut ssh_args = self.build_ssh_args(host, None);

        // Add environment variables as export commands
        let mut full_command = String::new();
        for (key, value) in &ctx.env_vars {
            full_command.push_str(&format!("export {}={}; ", key, shell_escape(value)));
        }

        // Add working directory change if specified
        if let Some(ref working_dir) = ctx.working_dir {
            full_command.push_str(&format!("cd {} && ", shell_escape(working_dir)));
        }

        // Add the actual command
        full_command.push_str(command);

        ssh_args.push(full_command);

        // Execute with timeout
        let result = self
            .run_with_timeout(ssh_args, Duration::from_millis(ctx.timeout_ms), ctx)
            .await;

        let execution_time = start.elapsed().as_millis() as u64;

        match result {
            Ok(mut res) => {
                res.execution_time_ms = execution_time;
                Ok(res)
            }
            Err(e) => {
                if matches!(e, RlmError::ExecutionTimeout { .. }) {
                    Ok(ExecutionResult::timeout(String::new(), e.to_string())
                        .with_execution_time(execution_time))
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Execute Python code on a remote VM.
    ///
    /// The code is written to a temporary file and executed with Python.
    ///
    /// # Arguments
    ///
    /// * `host` - VM IP address
    /// * `code` - Python code to execute
    /// * `ctx` - Execution context
    ///
    /// # Returns
    ///
    /// Execution result with stdout, stderr, and exit code.
    pub async fn execute_python(
        &self,
        host: &str,
        code: &str,
        ctx: &ExecutionContext,
    ) -> RlmResult<ExecutionResult> {
        let start = Instant::now();

        log::debug!(
            "SSH execute_python on {}: {} bytes of code",
            host,
            code.len()
        );

        // Create Python execution command using here-doc
        // This avoids issues with escaping and temporary files
        let python_command = format!(
            r#"python3 -u << 'TERRAPHIM_EOF'
{}
TERRAPHIM_EOF"#,
            code
        );

        let mut ssh_args = self.build_ssh_args(host, None);

        // Add environment variables
        let mut full_command = String::new();
        for (key, value) in &ctx.env_vars {
            full_command.push_str(&format!("export {}={}; ", key, shell_escape(value)));
        }

        // Add working directory change if specified
        if let Some(ref working_dir) = ctx.working_dir {
            full_command.push_str(&format!("cd {} && ", shell_escape(working_dir)));
        }

        full_command.push_str(&python_command);

        ssh_args.push(full_command);

        // Execute with timeout
        let result = self
            .run_with_timeout(ssh_args, Duration::from_millis(ctx.timeout_ms), ctx)
            .await;

        let execution_time = start.elapsed().as_millis() as u64;

        match result {
            Ok(mut res) => {
                res.execution_time_ms = execution_time;
                Ok(res)
            }
            Err(e) => {
                if matches!(e, RlmError::ExecutionTimeout { .. }) {
                    Ok(ExecutionResult::timeout(String::new(), e.to_string())
                        .with_execution_time(execution_time))
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Run a command with timeout and output handling.
    async fn run_with_timeout(
        &self,
        args: Vec<String>,
        timeout: Duration,
        ctx: &ExecutionContext,
    ) -> RlmResult<ExecutionResult> {
        let mut child = Command::new("ssh")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()
            .map_err(|e| RlmError::ExecutionFailed {
                message: format!("Failed to spawn SSH process: {}", e),
                exit_code: None,
                stdout: None,
                stderr: None,
            })?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        // Collect output with size limits
        let collect_task = async {
            let mut stdout_buf = String::new();
            let mut stderr_buf = String::new();
            let mut output_truncated = false;
            let mut output_file_path: Option<String> = None;

            if let Some(stdout_pipe) = stdout {
                let reader = BufReader::new(stdout_pipe);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    if stdout_buf.len() + line.len() + 1 > ctx.max_output_bytes as usize {
                        // Stream to file
                        if output_file_path.is_none() {
                            let file_path = self.output_dir.join(format!(
                                "output_{}.txt",
                                ctx.session_id
                            ));
                            tokio::fs::create_dir_all(&self.output_dir).await.ok();

                            // Write existing buffer to file
                            if let Ok(mut file) =
                                tokio::fs::File::create(&file_path).await
                            {
                                file.write_all(stdout_buf.as_bytes()).await.ok();
                                file.write_all(b"\n").await.ok();
                                file.write_all(line.as_bytes()).await.ok();
                                file.write_all(b"\n").await.ok();
                                output_file_path = Some(file_path.to_string_lossy().to_string());
                            }

                            output_truncated = true;
                            stdout_buf.push_str("\n[Output truncated - see file]");
                        } else if let Some(ref path) = output_file_path {
                            // Append to existing file
                            if let Ok(mut file) = tokio::fs::OpenOptions::new()
                                .append(true)
                                .open(path)
                                .await
                            {
                                file.write_all(line.as_bytes()).await.ok();
                                file.write_all(b"\n").await.ok();
                            }
                        }
                    } else {
                        if !stdout_buf.is_empty() {
                            stdout_buf.push('\n');
                        }
                        stdout_buf.push_str(&line);
                    }
                }
            }

            if let Some(stderr_pipe) = stderr {
                let reader = BufReader::new(stderr_pipe);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    if !stderr_buf.is_empty() {
                        stderr_buf.push('\n');
                    }
                    stderr_buf.push_str(&line);
                }
            }

            (stdout_buf, stderr_buf, output_truncated, output_file_path)
        };

        // Wait for process with timeout
        let result = tokio::select! {
            output = collect_task => {
                let status = child.wait().await.map_err(|e| RlmError::ExecutionFailed {
                    message: format!("Failed to wait for SSH process: {}", e),
                    exit_code: None,
                    stdout: None,
                    stderr: None,
                })?;

                let (stdout, stderr, truncated, file_path) = output;
                let exit_code = status.code().unwrap_or(-1);

                Ok(ExecutionResult {
                    stdout,
                    stderr,
                    exit_code,
                    execution_time_ms: 0, // Will be set by caller
                    output_truncated: truncated,
                    output_file_path: file_path,
                    timed_out: false,
                    metadata: std::collections::HashMap::new(),
                })
            }
            _ = tokio::time::sleep(timeout) => {
                // Kill the process
                child.kill().await.ok();
                Err(RlmError::ExecutionTimeout {
                    timeout_ms: timeout.as_millis() as u64,
                })
            }
        };

        result
    }

    /// Check if SSH connection to a host is possible.
    pub async fn check_connection(&self, host: &str) -> bool {
        let args = self.build_ssh_args(host, None);

        let result = Command::new("ssh")
            .args(&args)
            .arg("echo")
            .arg("ok")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        matches!(result, Ok(status) if status.success())
    }
}

impl Default for SshExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Escape a string for safe use in shell commands.
fn shell_escape(s: &str) -> String {
    // Use single quotes and escape any existing single quotes
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SessionId;

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("hello"), "'hello'");
        assert_eq!(shell_escape("hello world"), "'hello world'");
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
        assert_eq!(shell_escape(""), "''");
    }

    #[test]
    fn test_ssh_executor_creation() {
        let executor = SshExecutor::new()
            .with_user("ubuntu")
            .with_private_key("/path/to/key")
            .with_output_dir("/tmp/output");

        assert_eq!(executor.default_user, "ubuntu");
        assert_eq!(
            executor.private_key_path,
            Some(PathBuf::from("/path/to/key"))
        );
        assert_eq!(executor.output_dir, PathBuf::from("/tmp/output"));
    }

    #[test]
    fn test_build_ssh_args() {
        let executor = SshExecutor::new().with_user("ubuntu");
        let args = executor.build_ssh_args("192.168.1.1", None);

        assert!(args.contains(&"ubuntu@192.168.1.1".to_string()));
        assert!(args.contains(&"StrictHostKeyChecking=no".to_string()));
        assert!(args.contains(&"BatchMode=yes".to_string()));
    }

    #[test]
    fn test_build_ssh_args_with_key() {
        let executor = SshExecutor::new()
            .with_user("root")
            .with_private_key("/home/user/.ssh/id_rsa");

        let args = executor.build_ssh_args("10.0.0.1", Some("admin"));

        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"/home/user/.ssh/id_rsa".to_string()));
        assert!(args.contains(&"admin@10.0.0.1".to_string()));
    }

    #[test]
    fn test_execution_context_with_env_vars() {
        let session_id = SessionId::new();
        let ctx = ExecutionContext::for_session(session_id)
            .with_env("PATH", "/usr/bin:/bin")
            .with_env("HOME", "/home/user");

        assert_eq!(ctx.env_vars.len(), 2);
        assert_eq!(ctx.env_vars.get("PATH"), Some(&"/usr/bin:/bin".to_string()));
    }
}
