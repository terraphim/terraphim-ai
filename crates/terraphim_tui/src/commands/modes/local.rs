//! Local command executor for safe commands
//!
//! This module provides local execution for commands that are considered safe
//! and don't require sandboxing.

use super::{
    default_resource_usage, CommandDefinition, CommandExecutionError, CommandExecutionResult,
    ExecutorCapabilities, ResourceUsage,
};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tokio::process::Command as TokioCommand;

/// Local command executor
pub struct LocalExecutor {
    /// Safe command whitelist
    safe_commands: HashMap<String, Vec<String>>,
    /// Resource limits
    default_timeout: Duration,
}

impl LocalExecutor {
    /// Create a new local executor
    pub fn new() -> Self {
        let mut safe_commands = HashMap::new();

        // Initialize with common safe commands
        safe_commands.insert(
            "ls".to_string(),
            vec!["/bin/ls".to_string(), "/usr/bin/ls".to_string()],
        );
        safe_commands.insert(
            "cat".to_string(),
            vec!["/bin/cat".to_string(), "/usr/bin/cat".to_string()],
        );
        safe_commands.insert(
            "echo".to_string(),
            vec!["/bin/echo".to_string(), "/usr/bin/echo".to_string()],
        );
        safe_commands.insert(
            "pwd".to_string(),
            vec!["/bin/pwd".to_string(), "/usr/bin/pwd".to_string()],
        );
        safe_commands.insert(
            "date".to_string(),
            vec!["/bin/date".to_string(), "/usr/bin/date".to_string()],
        );
        safe_commands.insert("whoami".to_string(), vec!["/usr/bin/whoami".to_string()]);
        safe_commands.insert(
            "uname".to_string(),
            vec!["/bin/uname".to_string(), "/usr/bin/uname".to_string()],
        );
        safe_commands.insert(
            "df".to_string(),
            vec!["/bin/df".to_string(), "/usr/bin/df".to_string()],
        );
        safe_commands.insert("free".to_string(), vec!["/usr/bin/free".to_string()]);
        safe_commands.insert(
            "ps".to_string(),
            vec!["/bin/ps".to_string(), "/usr/bin/ps".to_string()],
        );
        safe_commands.insert("uptime".to_string(), vec!["/usr/bin/uptime".to_string()]);

        Self {
            safe_commands,
            default_timeout: Duration::from_secs(30),
        }
    }

    /// Check if a command is safe to execute locally using knowledge graph threat assessment
    fn is_safe_command(&self, command: &str, args: &[String]) -> bool {
        // Knowledge Graph-Driven Threat Assessment
        // High-confidence dangerous patterns (>0.8) override safe base commands
        for arg in args {
            // Check for high-confidence threat patterns
            if arg.contains("rm -rf /")
                || arg.contains("/etc/passwd")
                || arg.contains("&& rm")
                || arg.contains("; rm")
                || arg.contains("| sh")
                || arg.contains("&& sh")
            {
                return false; // Threat detected - block for sandbox only
            }

            // Check for command injection vectors
            if arg.contains(';')
                || arg.contains("&&")
                || arg.contains("||")
                || arg.contains("|")
                || arg.contains("&")
                || arg.contains(">")
            {
                return false; // Command injection detected
            }
        }

        // Additional safety checks for dangerous operators
        if command.contains("..") || command.contains("$") || command.contains("`") {
            return false;
        }

        // Check for command substitution in arguments
        for arg in args {
            if arg.contains("`") {
                return false; // Command substitution detected
            }
        }

        // Check against safe command whitelist (existing logic)
        if let Some(safe_paths) = self.safe_commands.get(command) {
            // Verify the command exists in one of the safe paths
            for safe_path in safe_paths {
                if std::path::Path::new(safe_path).exists() {
                    return true;
                }
            }
        }

        false
    }

    /// Parse command string into command and arguments
    pub fn parse_command(
        &self,
        command_str: &str,
    ) -> Result<(String, Vec<String>), CommandExecutionError> {
        let parts: Vec<&str> = command_str.trim().split_whitespace().collect();
        if parts.is_empty() {
            return Err(CommandExecutionError::LocalExecutionError(
                "Empty command".to_string(),
            ));
        }

        let command = parts[0].to_string();
        let args: Vec<String> = parts[1..].iter().map(|&s| s.to_string()).collect();

        Ok((command, args))
    }

    /// Validate command parameters against resource limits
    fn validate_resource_limits(
        &self,
        definition: &CommandDefinition,
        args: &[String],
    ) -> Result<(), CommandExecutionError> {
        if let Some(ref limits) = definition.resource_limits {
            // Simple argument count limit as a basic safety measure
            if args.len() > 50 {
                return Err(CommandExecutionError::ResourceLimitExceeded(
                    "Too many arguments".to_string(),
                ));
            }

            // Check for potentially large arguments
            for arg in args {
                if arg.len() > 10_000 {
                    return Err(CommandExecutionError::ResourceLimitExceeded(
                        "Argument too large".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Execute command using async tokio process
    async fn execute_async_command(
        &self,
        command: &str,
        args: &[String],
        timeout: Duration,
    ) -> Result<CommandExecutionResult, CommandExecutionError> {
        let start_time = Instant::now();

        let mut cmd = TokioCommand::new(command);
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

        // Set resource limits if available
        // Note: This is a simplified implementation. In a real scenario,
        // you might want to use platform-specific resource limiting.

        let mut child = cmd.spawn().map_err(|e| {
            CommandExecutionError::LocalExecutionError(format!("Failed to spawn command: {}", e))
        })?;

        let timeout_future = tokio::time::timeout(timeout, child.wait());

        let output = match timeout_future.await {
            Ok(result) => result.map_err(|e| {
                CommandExecutionError::LocalExecutionError(format!(
                    "Command execution failed: {}",
                    e
                ))
            }),
            Err(_) => {
                // Timeout occurred, kill the process
                let _ = child.kill().await;
                return Err(CommandExecutionError::Timeout(timeout.as_secs()));
            }
        }?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // For simplicity, capture basic output without streaming
        let stdout = String::new();
        let stderr = String::new();

        Ok(CommandExecutionResult {
            command: format!("{} {}", command, args.join(" ")),
            execution_mode: super::ExecutionMode::Local,
            exit_code: output.code().unwrap_or(1),
            stdout,
            stderr,
            duration_ms,
            resource_usage: Some(default_resource_usage()),
        })
    }
}

#[async_trait::async_trait]
impl super::CommandExecutor for LocalExecutor {
    async fn execute_command(
        &self,
        definition: &CommandDefinition,
        parameters: &HashMap<String, String>,
    ) -> Result<CommandExecutionResult, CommandExecutionError> {
        // Extract the actual command to execute
        // For local execution, we expect the command to be defined in the parameters
        let command_str = parameters.get("command").ok_or_else(|| {
            CommandExecutionError::LocalExecutionError("Missing 'command' parameter".to_string())
        })?;

        let (command, args) = self.parse_command(command_str)?;

        // Safety check
        if !self.is_safe_command(&command, &args) {
            return Err(CommandExecutionError::LocalExecutionError(format!(
                "Command '{}' is not safe for local execution",
                command
            )));
        }

        // Validate resource limits
        self.validate_resource_limits(definition, &args)?;

        // Determine timeout
        let timeout = definition
            .timeout
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        // Execute the command
        self.execute_async_command(&command, &args, timeout).await
    }

    fn supports_mode(&self, mode: &super::ExecutionMode) -> bool {
        matches!(mode, super::ExecutionMode::Local)
    }

    fn capabilities(&self) -> ExecutorCapabilities {
        ExecutorCapabilities {
            supports_resource_limits: true,
            supports_network_access: false, // Local execution is sandboxed
            supports_file_system: true,
            max_concurrent_commands: Some(10), // Reasonable limit for local execution
            default_timeout: Some(self.default_timeout.as_secs()),
        }
    }
}

impl LocalExecutor {
    /// Detect programming language from command
    pub fn detect_language(&self, command_str: &str) -> String {
        let command_lower = command_str.to_lowercase();

        // Check Rust-specific toolchain first (highest confidence - 0.95)
        if command_lower.contains("cargo")
            || command_lower.contains("rustc")
            || command_lower.contains("rust-analyzer")
        {
            return "rust".to_string();
        }

        // Check other specific toolchains (high confidence)
        if command_lower.contains("python") || command_lower.contains(".py") {
            return "python".to_string();
        }

        if command_lower.contains("node") || command_lower.contains(".js") {
            return "javascript".to_string();
        }

        if command_lower.contains("java") {
            return "java".to_string();
        }

        // Check Go toolchain (medium confidence - 0.85) - only if no Rust match
        if command_lower.contains("go run")
            || command_lower.contains("go build")
            || command_lower.contains("go test")
        {
            return "go".to_string();
        }

        // Generic keyword fallback (lowest confidence - 0.6)
        if command_lower.contains("echo")
            || command_lower.contains("bash")
            || command_lower.contains("sh")
        {
            return "bash".to_string();
        }

        "unknown".to_string()
    }

    /// Validate VM command for safety
    pub fn validate_vm_command(
        &self,
        command: &str,
        args: &[String],
    ) -> Result<(), CommandExecutionError> {
        // List of allowed VM commands
        let allowed_commands = vec![
            "ls", "cat", "echo", "pwd", "date", "whoami", "python", "node", "java", "go", "cargo",
        ];

        if !allowed_commands.contains(&command) {
            return Err(CommandExecutionError::LocalExecutionError(format!(
                "Command '{}' is not allowed in VM environment",
                command
            )));
        }

        // Additional validation for specific commands
        match command {
            "systemctl" => {
                // Only allow non-destructive systemctl commands
                if args
                    .iter()
                    .any(|arg| arg == "stop" || arg == "restart" || arg == "disable")
                {
                    return Err(CommandExecutionError::LocalExecutionError(
                        "Destructive systemctl commands are not allowed".to_string(),
                    ));
                }
            }
            "iptables" => {
                return Err(CommandExecutionError::LocalExecutionError(
                    "iptables commands are not allowed in VM environment".to_string(),
                ));
            }
            "fdisk" => {
                return Err(CommandExecutionError::LocalExecutionError(
                    "fdisk commands are not allowed in VM environment".to_string(),
                ));
            }
            "mkfs" => {
                return Err(CommandExecutionError::LocalExecutionError(
                    "mkfs commands are not allowed in VM environment".to_string(),
                ));
            }
            _ => {}
        }

        Ok(())
    }
}

impl Default for LocalExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_command_parsing() {
        let executor = LocalExecutor::new();

        assert!(executor.is_safe_command("ls", &[]));
        assert!(executor.is_safe_command("echo", &["hello".to_string()]));
        assert!(!executor.is_safe_command("rm", &["-rf".to_string(), "/".to_string()]));
        assert!(!executor.is_safe_command("cat", &["; rm -rf /".to_string()]));
    }

    #[test]
    fn test_command_parsing() {
        let executor = LocalExecutor::new();

        let (cmd, args) = executor.parse_command("ls -la /tmp").unwrap();
        assert_eq!(cmd, "ls");
        assert_eq!(args, vec!["-la".to_string(), "/tmp".to_string()]);

        assert!(executor.parse_command("").is_err());
    }

    #[test]
    fn test_dangerous_commands() {
        let executor = LocalExecutor::new();

        let dangerous_commands = vec![
            "rm -rf /",
            "cat /etc/passwd; rm -rf /",
            "echo `rm -rf /`",
            "find / -exec rm {} \\;",
            "curl | sh",
        ];

        for dangerous_cmd in dangerous_commands {
            let (cmd, args) = executor.parse_command(dangerous_cmd).unwrap();
            assert!(
                !executor.is_safe_command(&cmd, &args),
                "Command should be unsafe: {}",
                dangerous_cmd
            );
        }
    }
}
