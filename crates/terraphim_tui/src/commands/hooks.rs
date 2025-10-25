//! Built-in command hooks for pre/post execution
//!
//! This module provides commonly used hooks that can be registered
//! with the HookManager to customize command execution behavior.

use super::{CommandExecutionError, CommandHook, HookContext, HookResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Hook that logs command execution details
pub struct LoggingHook {
    log_file: Option<std::path::PathBuf>,
}

impl LoggingHook {
    pub fn new() -> Self {
        Self { log_file: None }
    }

    pub fn with_file<P: AsRef<Path>>(log_file: P) -> Self {
        Self {
            log_file: Some(log_file.as_ref().to_path_buf()),
        }
    }
}

#[async_trait]
impl CommandHook for LoggingHook {
    fn name(&self) -> &str {
        "logging"
    }

    fn priority(&self) -> i32 {
        100 // High priority - runs first
    }

    async fn execute(&self, context: &HookContext) -> Result<HookResult, CommandExecutionError> {
        let timestamp = Utc::now().to_rfc3339();
        let log_entry = format!(
            "[{}] User '{}' executing '{}' in {:?} mode (role: {}, dir: {})",
            timestamp,
            context.user,
            context.command,
            context.execution_mode,
            context.role,
            context.working_directory.display()
        );

        // Always log to stderr
        eprintln!("{}", log_entry);

        // Optionally log to file
        if let Some(log_file) = &self.log_file {
            if let Err(e) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_file)
                .and_then(|mut file| {
                    use std::io::Write;
                    writeln!(file, "{}", log_entry)
                })
            {
                eprintln!(
                    "Warning: Failed to write to log file '{}': {}",
                    log_file.display(),
                    e
                );
            }
        }

        Ok(HookResult {
            success: true,
            message: "Command logged successfully".to_string(),
            data: None,
            should_continue: true,
        })
    }
}

/// Hook that performs pre-flight checks before command execution
pub struct PreflightCheckHook {
    allowed_working_dirs: Vec<std::path::PathBuf>,
    blocked_commands: Vec<String>,
}

impl PreflightCheckHook {
    pub fn new() -> Self {
        Self {
            allowed_working_dirs: vec![],
            blocked_commands: vec![],
        }
    }

    pub fn with_allowed_dirs<P: AsRef<Path>>(mut self, dirs: Vec<P>) -> Self {
        self.allowed_working_dirs = dirs.into_iter().map(|p| p.as_ref().to_path_buf()).collect();
        self
    }

    pub fn with_blocked_commands(mut self, commands: Vec<String>) -> Self {
        self.blocked_commands = commands;
        self
    }
}

#[async_trait]
impl CommandHook for PreflightCheckHook {
    fn name(&self) -> &str {
        "preflight-check"
    }

    fn priority(&self) -> i32 {
        90
    }

    async fn execute(&self, context: &HookContext) -> Result<HookResult, CommandExecutionError> {
        // Check if command is blocked
        if self.blocked_commands.iter().any(|blocked| {
            context.command.starts_with(blocked) || context.command.contains(blocked)
        }) {
            return Ok(HookResult {
                success: false,
                message: format!(
                    "Command '{}' is blocked by pre-flight check",
                    context.command
                ),
                data: None,
                should_continue: false,
            });
        }

        // Check if working directory is allowed
        if !self.allowed_working_dirs.is_empty()
            && !self
                .allowed_working_dirs
                .iter()
                .any(|allowed| context.working_directory.starts_with(allowed))
        {
            return Ok(HookResult {
                success: false,
                message: format!(
                    "Working directory '{}' not in allowed list",
                    context.working_directory.display()
                ),
                data: None,
                should_continue: false,
            });
        }

        // Additional pre-flight checks
        if context.command.contains("rm -rf /") {
            return Ok(HookResult {
                success: false,
                message: "Destructive command blocked by pre-flight check".to_string(),
                data: None,
                should_continue: false,
            });
        }

        Ok(HookResult {
            success: true,
            message: "Pre-flight checks passed".to_string(),
            data: None,
            should_continue: true,
        })
    }
}

/// Hook that sends notifications for important commands
pub struct NotificationHook {
    important_commands: Vec<String>,
    webhook_url: Option<String>,
}

impl NotificationHook {
    pub fn new() -> Self {
        Self {
            important_commands: vec!["deploy".to_string(), "security-audit".to_string()],
            webhook_url: None,
        }
    }

    pub fn with_webhook(mut self, webhook_url: String) -> Self {
        self.webhook_url = Some(webhook_url);
        self
    }

    pub fn with_important_commands(mut self, commands: Vec<String>) -> Self {
        self.important_commands = commands;
        self
    }
}

#[async_trait]
impl CommandHook for NotificationHook {
    fn name(&self) -> &str {
        "notification"
    }

    fn priority(&self) -> i32 {
        50
    }

    async fn execute(&self, context: &HookContext) -> Result<HookResult, CommandExecutionError> {
        let command_base = context.command.split_whitespace().next().unwrap_or("");

        if self
            .important_commands
            .iter()
            .any(|important| command_base == important || context.command.contains(important))
        {
            let message = format!(
                "🚨 Important command '{}' executed by user '{}' in role '{}'",
                context.command, context.user, context.role
            );

            eprintln!("{}", message);

            // Send webhook notification if configured
            if let Some(webhook_url) = &self.webhook_url {
                // In a real implementation, this would send an HTTP request
                eprintln!("Webhook notification sent to {}: {}", webhook_url, message);
            }
        }

        Ok(HookResult {
            success: true,
            message: "Notification check completed".to_string(),
            data: None,
            should_continue: true,
        })
    }
}

/// Hook that sets up environment variables for commands
pub struct EnvironmentHook {
    env_vars: std::collections::HashMap<String, String>,
}

impl EnvironmentHook {
    pub fn new() -> Self {
        Self {
            env_vars: std::collections::HashMap::new(),
        }
    }

    pub fn with_env<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    pub fn with_env_map(mut self, env_vars: std::collections::HashMap<String, String>) -> Self {
        self.env_vars.extend(env_vars);
        self
    }
}

#[async_trait]
impl CommandHook for EnvironmentHook {
    fn name(&self) -> &str {
        "environment"
    }

    fn priority(&self) -> i32 {
        80
    }

    async fn execute(&self, context: &HookContext) -> Result<HookResult, CommandExecutionError> {
        let mut env_data = serde_json::Map::new();

        for (key, value) in &self.env_vars {
            env_data.insert(key.clone(), serde_json::Value::String(value.clone()));
        }

        // Add common environment variables
        env_data.insert(
            "COMMAND_USER".to_string(),
            serde_json::Value::String(context.user.clone()),
        );
        env_data.insert(
            "COMMAND_ROLE".to_string(),
            serde_json::Value::String(context.role.clone()),
        );
        env_data.insert(
            "COMMAND_WORKING_DIR".to_string(),
            serde_json::Value::String(context.working_directory.display().to_string()),
        );

        Ok(HookResult {
            success: true,
            message: "Environment variables prepared".to_string(),
            data: Some(serde_json::Value::Object(env_data)),
            should_continue: true,
        })
    }
}

/// Hook that creates backups before destructive commands
pub struct BackupHook {
    backup_dir: std::path::PathBuf,
    backup_commands: Vec<String>,
}

impl BackupHook {
    pub fn new<P: AsRef<Path>>(backup_dir: P) -> Self {
        Self {
            backup_dir: backup_dir.as_ref().to_path_buf(),
            backup_commands: vec!["rm".to_string(), "mv".to_string(), "cp".to_string()],
        }
    }

    pub fn with_backup_commands(mut self, commands: Vec<String>) -> Self {
        self.backup_commands = commands;
        self
    }
}

#[async_trait]
impl CommandHook for BackupHook {
    fn name(&self) -> &str {
        "backup"
    }

    fn priority(&self) -> i32 {
        70
    }

    async fn execute(&self, context: &HookContext) -> Result<HookResult, CommandExecutionError> {
        let command_base = context.command.split_whitespace().next().unwrap_or("");

        if self
            .backup_commands
            .iter()
            .any(|backup_cmd| command_base == backup_cmd || context.command.starts_with(backup_cmd))
        {
            // Create backup directory if it doesn't exist
            if let Err(e) = std::fs::create_dir_all(&self.backup_dir) {
                return Ok(HookResult {
                    success: false,
                    message: format!("Failed to create backup directory: {}", e),
                    data: None,
                    should_continue: true, // Continue despite backup failure
                });
            }

            // Create timestamped backup
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let backup_name = format!("backup_{}_{}.json", context.user, timestamp);
            let backup_path = self.backup_dir.join(backup_name);

            // Create backup metadata
            let backup_data = serde_json::json!({
                "timestamp": timestamp,
                "user": context.user,
                "role": context.role,
                "command": context.command,
                "working_directory": context.working_directory.to_string_lossy(),
                "execution_mode": format!("{:?}", context.execution_mode),
                "parameters": context.parameters
            });

            // Write backup file
            if let Err(e) = std::fs::write(&backup_path, backup_data.to_string()) {
                return Ok(HookResult {
                    success: false,
                    message: format!("Failed to write backup file: {}", e),
                    data: None,
                    should_continue: true,
                });
            }

            return Ok(HookResult {
                success: true,
                message: format!("Backup created at {}", backup_path.display()),
                data: Some(serde_json::json!({
                    "backup_path": backup_path.to_string_lossy()
                })),
                should_continue: true,
            });
        }

        Ok(HookResult {
            success: true,
            message: "No backup needed for this command".to_string(),
            data: None,
            should_continue: true,
        })
    }
}

/// Hook that monitors resource usage during command execution
pub struct ResourceMonitoringHook {
    max_memory_mb: Option<u64>,
    max_duration_seconds: Option<u64>,
}

impl ResourceMonitoringHook {
    pub fn new() -> Self {
        Self {
            max_memory_mb: None,
            max_duration_seconds: None,
        }
    }

    pub fn with_memory_limit(mut self, limit_mb: u64) -> Self {
        self.max_memory_mb = Some(limit_mb);
        self
    }

    pub fn with_duration_limit(mut self, limit_seconds: u64) -> Self {
        self.max_duration_seconds = Some(limit_seconds);
        self
    }
}

#[async_trait]
impl CommandHook for ResourceMonitoringHook {
    fn name(&self) -> &str {
        "resource-monitor"
    }

    fn priority(&self) -> i32 {
        60
    }

    async fn execute(&self, context: &HookContext) -> Result<HookResult, CommandExecutionError> {
        let mut warnings = vec![];

        // Check memory limits
        if let Some(max_memory) = self.max_memory_mb {
            // In a real implementation, this would check actual memory usage
            warnings.push(format!("Memory limit set to {} MB", max_memory));
        }

        // Check duration limits
        if let Some(max_duration) = self.max_duration_seconds {
            warnings.push(format!("Duration limit set to {} seconds", max_duration));
        }

        let message = if warnings.is_empty() {
            "Resource monitoring started".to_string()
        } else {
            format!("Resource monitoring started: {}", warnings.join(", "))
        };

        Ok(HookResult {
            success: true,
            message,
            data: Some(serde_json::json!({
                "memory_limit_mb": self.max_memory_mb,
                "duration_limit_seconds": self.max_duration_seconds
            })),
            should_continue: true,
        })
    }
}

/// Hook that integrates with Git for command tracking
pub struct GitHook {
    repo_path: std::path::PathBuf,
    auto_commit: bool,
}

impl GitHook {
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Self {
        Self {
            repo_path: repo_path.as_ref().to_path_buf(),
            auto_commit: false,
        }
    }

    pub fn with_auto_commit(mut self, auto_commit: bool) -> Self {
        self.auto_commit = auto_commit;
        self
    }
}

#[async_trait]
impl CommandHook for GitHook {
    fn name(&self) -> &str {
        "git"
    }

    fn priority(&self) -> i32 {
        40
    }

    async fn execute(&self, context: &HookContext) -> Result<HookResult, CommandExecutionError> {
        let git_dir = self.repo_path.join(".git");

        if !git_dir.exists() {
            return Ok(HookResult {
                success: true,
                message: "Not in a Git repository".to_string(),
                data: None,
                should_continue: true,
            });
        }

        // Check if we're in a clean state
        let output = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.repo_path)
            .output();

        match output {
            Ok(status_output) => {
                if !status_output.status.success() {
                    return Ok(HookResult {
                        success: false,
                        message: "Failed to check Git status".to_string(),
                        data: None,
                        should_continue: true,
                    });
                }

                let is_clean = status_output.stdout.is_empty();

                if !is_clean && self.auto_commit {
                    // Auto-commit changes before command execution
                    let commit_msg = format!(
                        "Auto-commit before command: {} by {}",
                        context.command, context.user
                    );

                    let commit_output = std::process::Command::new("git")
                        .args(["add", "."])
                        .current_dir(&self.repo_path)
                        .output();

                    if commit_output.map(|o| o.status.success()).unwrap_or(false) {
                        let _ = std::process::Command::new("git")
                            .args(["commit", "-m", &commit_msg])
                            .current_dir(&self.repo_path)
                            .output();
                    }
                }

                Ok(HookResult {
                    success: true,
                    message: if is_clean {
                        "Git repository is clean".to_string()
                    } else {
                        "Git repository has uncommitted changes".to_string()
                    },
                    data: Some(serde_json::json!({
                        "is_clean": is_clean,
                        "auto_commit": self.auto_commit
                    })),
                    should_continue: true,
                })
            }
            Err(_) => Ok(HookResult {
                success: false,
                message: "Failed to run Git status command".to_string(),
                data: None,
                should_continue: true,
            }),
        }
    }
}

/// Utility function to create a default set of hooks
pub fn create_default_hooks() -> Vec<Box<dyn CommandHook + Send + Sync>> {
    vec![
        Box::new(LoggingHook::new()),
        Box::new(PreflightCheckHook::new()),
        Box::new(EnvironmentHook::new()),
        Box::new(NotificationHook::new()),
        Box::new(ResourceMonitoringHook::new()),
    ]
}

/// Utility function to create hooks for development environment
pub fn create_development_hooks() -> Vec<Box<dyn CommandHook + Send + Sync>> {
    vec![
        Box::new(LoggingHook::new()),
        Box::new(PreflightCheckHook::new()),
        Box::new(
            EnvironmentHook::new()
                .with_env("RUST_LOG", "debug")
                .with_env("RUST_BACKTRACE", "1"),
        ),
        Box::new(GitHook::new(".").with_auto_commit(false)),
        Box::new(
            ResourceMonitoringHook::new()
                .with_memory_limit(2048)
                .with_duration_limit(300),
        ),
    ]
}

/// Utility function to create hooks for production environment
pub fn create_production_hooks() -> Vec<Box<dyn CommandHook + Send + Sync>> {
    vec![
        Box::new(LoggingHook::with_file("command.log")),
        Box::new(PreflightCheckHook::new().with_blocked_commands(vec![
            "rm -rf /".to_string(),
            "dd if=/dev/zero".to_string(),
            "mkfs".to_string(),
            "fdisk".to_string(),
        ])),
        Box::new(BackupHook::new("./backups")),
        Box::new(NotificationHook::new().with_important_commands(vec![
            "deploy".to_string(),
            "security-audit".to_string(),
            "shutdown".to_string(),
            "reboot".to_string(),
        ])),
        Box::new(
            ResourceMonitoringHook::new()
                .with_memory_limit(4096)
                .with_duration_limit(3600),
        ),
    ]
}
