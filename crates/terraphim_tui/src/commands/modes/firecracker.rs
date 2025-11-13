//! Firecracker VM executor for isolated command execution
//!
//! This module provides command execution within Firecracker microVMs
//! for complete isolation and security.

use super::{
    default_resource_usage, CommandDefinition, CommandExecutionError, CommandExecutionResult,
    ExecutorCapabilities, ResourceUsage,
};
use crate::client::ApiClient;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Firecracker VM executor
pub struct FirecrackerExecutor {
    /// API client for VM management
    api_client: Option<ApiClient>,
    /// Default timeout
    default_timeout: Duration,
    /// VM pool settings
    vm_settings: VmSettings,
}

/// VM settings for firecracker execution
#[derive(Debug, Clone)]
pub struct VmSettings {
    /// Memory limit in MB
    pub memory_mb: u64,
    /// CPU limit
    pub vcpu_count: u8,
    /// Disk limit in MB
    pub disk_mb: u64,
    /// Network enabled
    pub network_enabled: bool,
    /// Root filesystem
    pub root_fs: String,
}

impl Default for VmSettings {
    fn default() -> Self {
        Self {
            memory_mb: 512,
            vcpu_count: 1,
            disk_mb: 1024,
            network_enabled: false,
            root_fs: "ubuntu:22.04".to_string(), // Default to Ubuntu
        }
    }
}

impl FirecrackerExecutor {
    /// Create a new firecracker executor
    pub fn new() -> Self {
        Self {
            api_client: None,
            default_timeout: Duration::from_secs(300), // 5 minutes for VM operations
            vm_settings: VmSettings::default(),
        }
    }

    /// Create a new firecracker executor with API client
    pub fn with_api_client(api_client: ApiClient) -> Self {
        Self {
            api_client: Some(api_client),
            default_timeout: Duration::from_secs(300),
            vm_settings: VmSettings::default(),
        }
    }

    /// Set VM settings
    pub fn with_vm_settings(mut self, settings: VmSettings) -> Self {
        self.vm_settings = settings;
        self
    }

    /// Prepare VM for command execution
    async fn prepare_vm(&self, command: &str) -> Result<String, CommandExecutionError> {
        let api_client = self.api_client.as_ref().ok_or_else(|| {
            CommandExecutionError::VmExecutionError("No API client available".to_string())
        })?;

        // Generate a unique VM ID for this command
        let vm_id = format!(
            "firecracker-{}-{}",
            command.replace(['/', ' '], "-"),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );

        // Note: VM allocation is handled differently - for now use existing VM or allocate one
        // We'll use the existing VM pool functionality
        let _response = api_client.get_vm_status(&vm_id).await.map_err(|_| {
            CommandExecutionError::VmExecutionError(format!("VM '{}' not available", vm_id))
        })?;

        Ok(vm_id)
    }

    /// Execute command in VM
    async fn execute_in_vm(
        &self,
        vm_id: &str,
        command: &str,
        args: &[String],
        _timeout: Duration,
    ) -> Result<CommandExecutionResult, CommandExecutionError> {
        let api_client = self.api_client.as_ref().ok_or_else(|| {
            CommandExecutionError::VmExecutionError("No API client available".to_string())
        })?;

        let start_time = Instant::now();

        // Construct the full command string
        let full_command = format!("{} {}", command, args.join(" "));

        // Determine the language for VM execution
        let language = self.detect_language(command);

        // Execute the command in the VM
        let response = api_client
            .execute_vm_code(&full_command, &language, Some(vm_id))
            .await
            .map_err(|e| {
                CommandExecutionError::VmExecutionError(format!("VM execution failed: {}", e))
            })?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(CommandExecutionResult {
            command: full_command,
            execution_mode: super::ExecutionMode::Firecracker,
            exit_code: response.exit_code,
            stdout: response.stdout.clone(),
            stderr: response.stderr.clone(),
            duration_ms,
            resource_usage: Some(self.calculate_resource_usage(&response)),
        })
    }

    /// Detect programming language for VM execution
    fn detect_language(&self, command: &str) -> String {
        // Simple language detection based on command content
        if command.contains("python") || command.contains("pip") {
            "python".to_string()
        } else if command.contains("node") || command.contains("npm") {
            "javascript".to_string()
        } else if command.contains("java") || command.contains("javac") {
            "java".to_string()
        } else if command.contains("go") {
            "go".to_string()
        } else if command.contains("rust") || command.contains("cargo") {
            "rust".to_string()
        } else {
            "bash".to_string() // Default to bash
        }
    }

    /// Calculate resource usage from VM response
    fn calculate_resource_usage(
        &self,
        _response: &crate::client::VmExecuteResponse,
    ) -> ResourceUsage {
        // This would be enhanced in a real implementation
        // For now, return default values
        default_resource_usage()
    }

    /// Clean up VM after execution
    async fn cleanup_vm(&self, vm_id: &str) -> Result<(), CommandExecutionError> {
        if let Some(api_client) = &self.api_client {
            // Note: VM cleanup is handled differently for now
            // In a full implementation, we'd release the VM back to the pool
            let _response = api_client.get_vm_status(vm_id).await.map_err(|e| {
                CommandExecutionError::VmExecutionError(format!("Failed to check VM status: {}", e))
            })?;
        }

        Ok(())
    }

    /// Parse command string into command and arguments
    fn parse_command(
        &self,
        command_str: &str,
    ) -> Result<(String, Vec<String>), CommandExecutionError> {
        let parts: Vec<&str> = command_str.split_whitespace().collect();
        if parts.is_empty() {
            return Err(CommandExecutionError::VmExecutionError(
                "Empty command".to_string(),
            ));
        }

        let command = parts[0].to_string();
        let args: Vec<String> = parts[1..].iter().map(|&s| s.to_string()).collect();

        Ok((command, args))
    }

    /// Validate command for VM execution
    fn validate_vm_command(
        &self,
        command: &str,
        args: &[String],
    ) -> Result<(), CommandExecutionError> {
        // Check for commands that might not work well in VMs
        let vm_incompatible_commands = [
            "systemctl",
            "service",
            "init",
            "shutdown",
            "reboot",
            "mount",
            "umount",
            "fdisk",
            "mkfs",
            "iptables",
            "ufw",
            "firewall",
        ];

        if vm_incompatible_commands.contains(&command) {
            return Err(CommandExecutionError::VmExecutionError(format!(
                "Command '{}' is not compatible with VM execution",
                command
            )));
        }

        // Check for extremely long commands
        let total_length = command.len() + args.iter().map(|a| a.len()).sum::<usize>();
        if total_length > 100_000 {
            return Err(CommandExecutionError::VmExecutionError(
                "Command too long for VM execution".to_string(),
            ));
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl super::CommandExecutor for FirecrackerExecutor {
    async fn execute_command(
        &self,
        definition: &CommandDefinition,
        parameters: &HashMap<String, String>,
    ) -> Result<CommandExecutionResult, CommandExecutionError> {
        // Extract the actual command to execute
        let command_str = parameters.get("command").ok_or_else(|| {
            CommandExecutionError::VmExecutionError("Missing 'command' parameter".to_string())
        })?;

        let (command, args) = self.parse_command(command_str)?;

        // Validate command for VM execution
        self.validate_vm_command(&command, &args)?;

        // Prepare VM
        let vm_id = self.prepare_vm(&command).await?;

        // Determine timeout
        let timeout = definition
            .timeout
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        // Execute command with timeout
        let execution_result = tokio::time::timeout(
            timeout,
            self.execute_in_vm(&vm_id, &command, &args, timeout),
        )
        .await
        .map_err(|_| CommandExecutionError::Timeout(timeout.as_secs()))??;

        // Clean up VM (don't fail the whole operation if cleanup fails)
        let _ = self.cleanup_vm(&vm_id).await;

        Ok(execution_result)
    }

    fn supports_mode(&self, mode: &super::ExecutionMode) -> bool {
        matches!(mode, super::ExecutionMode::Firecracker)
    }

    fn capabilities(&self) -> ExecutorCapabilities {
        ExecutorCapabilities {
            supports_resource_limits: true,
            supports_network_access: self.vm_settings.network_enabled,
            supports_file_system: true,
            max_concurrent_commands: Some(5), // VMs are resource intensive
            default_timeout: Some(self.default_timeout.as_secs()),
        }
    }
}

impl Default for FirecrackerExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        let executor = LocalExecutor::new();

        assert_eq!(executor.detect_language("python script.py"), "python");
        assert_eq!(executor.detect_language("node app.js"), "javascript");
        assert_eq!(executor.detect_language("java Main"), "java");
        assert_eq!(executor.detect_language("go run main.go"), "go");
        assert_eq!(executor.detect_language("cargo build"), "rust");
        assert_eq!(executor.detect_language("echo hello"), "bash");
    }

    #[test]
    fn test_vm_command_validation() {
        let executor = LocalExecutor::new();

        // Valid commands
        assert!(executor.validate_vm_command("ls", &[]).is_ok());
        assert!(executor
            .validate_vm_command("python", &["script.py".to_string()])
            .is_ok());

        // Invalid commands for VMs
        assert!(executor
            .validate_vm_command("systemctl", &["restart".to_string(), "nginx".to_string()])
            .is_err());
        assert!(executor
            .validate_vm_command("iptables", &["-L".to_string()])
            .is_err());
        assert!(executor
            .validate_vm_command("fdisk", &["/dev/sda".to_string()])
            .is_err());
    }

    #[test]
    fn test_command_parsing() {
        let executor = LocalExecutor::new();

        let (cmd, args) = executor
            .parse_command("python script.py --verbose")
            .unwrap();
        assert_eq!(cmd, "python");
        assert_eq!(args, vec!["script.py".to_string(), "--verbose".to_string()]);

        assert!(executor.parse_command("").is_err());
    }

    #[test]
    fn test_vm_settings_default() {
        let settings = VmSettings::default();
        assert_eq!(settings.memory_mb, 512);
        assert_eq!(settings.vcpu_count, 1);
        assert_eq!(settings.disk_mb, 1024);
        assert!(!settings.network_enabled);
        assert_eq!(settings.root_fs, "ubuntu:22.04");
    }
}
