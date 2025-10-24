//! Hybrid executor for intelligent execution mode selection
//!
//! This module provides smart execution mode selection based on risk assessment,
//! command type, and available infrastructure.

use super::{
    CommandDefinition, CommandExecutionError, CommandExecutionResult, ExecutionMode,
    ExecutorCapabilities, FirecrackerExecutor, LocalExecutor,
};
use crate::commands::RiskLevel;
use std::collections::HashMap;

/// Hybrid executor that selects the best execution mode
pub struct HybridExecutor {
    /// Local executor for safe commands
    local_executor: LocalExecutor,
    /// Firecracker executor for isolated execution
    firecracker_executor: FirecrackerExecutor,
    /// Risk assessment settings
    risk_settings: RiskAssessmentSettings,
}

/// Settings for risk assessment
#[derive(Debug, Clone)]
pub struct RiskAssessmentSettings {
    /// Commands that always require isolation
    high_risk_commands: Vec<String>,
    /// Commands that are safe for local execution
    safe_commands: Vec<String>,
    /// Keywords that indicate high risk
    high_risk_keywords: Vec<String>,
    /// Always use VM for commands from unknown sources
    vm_for_unknown: bool,
    /// Maximum risk level for local execution
    max_local_risk_level: RiskLevel,
}

impl Default for RiskAssessmentSettings {
    fn default() -> Self {
        let high_risk_commands = vec![
            "rm".to_string(),
            "dd".to_string(),
            "mkfs".to_string(),
            "fdisk".to_string(),
            "iptables".to_string(),
            "systemctl".to_string(),
            "service".to_string(),
            "init".to_string(),
            "shutdown".to_string(),
            "reboot".to_string(),
            "halt".to_string(),
            "poweroff".to_string(),
            "chown".to_string(),
            "chmod".to_string(),
            "sudo".to_string(),
            "su".to_string(),
            "doas".to_string(),
            "passwd".to_string(),
            "useradd".to_string(),
            "userdel".to_string(),
            "groupadd".to_string(),
            "mount".to_string(),
            "umount".to_string(),
            "swapon".to_string(),
            "swapoff".to_string(),
            "lvcreate".to_string(),
            "lvremove".to_string(),
            "fdformat".to_string(),
            "fsck".to_string(),
            "debugfs".to_string(),
        ];

        let safe_commands = vec![
            "ls".to_string(),
            "cat".to_string(),
            "echo".to_string(),
            "pwd".to_string(),
            "date".to_string(),
            "whoami".to_string(),
            "uname".to_string(),
            "df".to_string(),
            "free".to_string(),
            "ps".to_string(),
            "uptime".to_string(),
            "wc".to_string(),
            "head".to_string(),
            "tail".to_string(),
            "grep".to_string(),
            "sort".to_string(),
            "uniq".to_string(),
            "cut".to_string(),
            "awk".to_string(),
            "sed".to_string(),
            "tr".to_string(),
            "basename".to_string(),
            "dirname".to_string(),
            "realpath".to_string(),
            "readlink".to_string(),
            "stat".to_string(),
            "file".to_string(),
            "which".to_string(),
            "whereis".to_string(),
            "type".to_string(),
            "hash".to_string(),
            "env".to_string(),
            "printenv".to_string(),
            "export".to_string(),
            "alias".to_string(),
            "unalias".to_string(),
        ];

        let high_risk_keywords = vec![
            "rm -rf".to_string(),
            "dd if=".to_string(),
            "mkfs".to_string(),
            "/dev/".to_string(),
            "iptables".to_string(),
            "systemctl".to_string(),
            "shutdown".to_string(),
            "reboot".to_string(),
            "passwd".to_string(),
            "sudo".to_string(),
            "su -".to_string(),
            "chmod 777".to_string(),
            "chown root".to_string(),
            ">/etc/".to_string(),
            ">>/etc/".to_string(),
            "curl | sh".to_string(),
            "wget | sh".to_string(),
            "eval".to_string(),
            "exec".to_string(),
            "source".to_string(),
            "\\$\\(".to_string(),
        ];

        Self {
            high_risk_commands,
            safe_commands,
            high_risk_keywords,
            vm_for_unknown: true,
            max_local_risk_level: RiskLevel::Medium,
        }
    }
}

impl HybridExecutor {
    /// Create a new hybrid executor
    pub fn new() -> Self {
        Self {
            local_executor: LocalExecutor::new(),
            firecracker_executor: FirecrackerExecutor::new(),
            risk_settings: RiskAssessmentSettings::default(),
        }
    }

    /// Create a hybrid executor with custom settings
    pub fn with_settings(risk_settings: RiskAssessmentSettings) -> Self {
        Self {
            local_executor: LocalExecutor::new(),
            firecracker_executor: FirecrackerExecutor::new(),
            risk_settings,
        }
    }

    /// Create a hybrid executor with API client for VM operations
    pub fn with_api_client(api_client: crate::client::ApiClient) -> Self {
        Self {
            local_executor: LocalExecutor::new(),
            firecracker_executor: FirecrackerExecutor::with_api_client(api_client),
            risk_settings: RiskAssessmentSettings::default(),
        }
    }

    /// Assess command risk and determine execution mode
    fn assess_command_risk(
        &self,
        command_str: &str,
        definition: &CommandDefinition,
    ) -> ExecutionMode {
        // Start with the command's preferred execution mode
        let preferred_mode = &definition.execution_mode;

        // If command explicitly requires a specific mode, respect it
        match preferred_mode {
            ExecutionMode::Local => {
                // Verify it's actually safe for local execution
                if self.is_safe_for_local_execution(command_str, definition) {
                    return ExecutionMode::Local;
                }
            }
            ExecutionMode::Firecracker => {
                return ExecutionMode::Firecracker;
            }
            ExecutionMode::Hybrid => {
                // Perform risk assessment
                return self.determine_execution_mode(command_str, definition);
            }
        }

        // Default to hybrid mode determination
        self.determine_execution_mode(command_str, definition)
    }

    /// Determine the best execution mode based on risk assessment
    fn determine_execution_mode(
        &self,
        command_str: &str,
        definition: &CommandDefinition,
    ) -> ExecutionMode {
        // Check command risk level
        match definition.risk_level {
            RiskLevel::Critical | RiskLevel::High => {
                return ExecutionMode::Firecracker;
            }
            RiskLevel::Medium => {
                // Medium risk: check other factors
                if self.has_high_risk_indicators(command_str) {
                    return ExecutionMode::Firecracker;
                }
                if definition.resource_limits.is_some() {
                    return ExecutionMode::Firecracker;
                }
            }
            RiskLevel::Low => {
                // Low risk: could be safe for local execution
                if self.is_safe_for_local_execution(command_str, definition) {
                    return ExecutionMode::Local;
                }
            }
        }

        // Default to Firecracker for safety
        ExecutionMode::Firecracker
    }

    /// Check if command is safe for local execution
    fn is_safe_for_local_execution(
        &self,
        command_str: &str,
        definition: &CommandDefinition,
    ) -> bool {
        // Check risk level
        // Check risk level using pattern matching to avoid Copy trait requirement
        let risk_too_high = matches!(
            (
                definition.risk_level.clone(),
                self.risk_settings.max_local_risk_level.clone()
            ),
            (RiskLevel::High, _)
                | (RiskLevel::Critical, _)
                | (RiskLevel::Medium, RiskLevel::Critical)
        );
        if risk_too_high {
            return false;
        }

        // Check if command is in safe list
        let command = self.extract_command_name(command_str);
        if self.risk_settings.safe_commands.contains(&command) {
            return true;
        }

        // Check if command is in high-risk list
        if self.risk_settings.high_risk_commands.contains(&command) {
            return false;
        }

        // Check for high-risk keywords
        for keyword in &self.risk_settings.high_risk_keywords {
            if command_str.contains(keyword) {
                return false;
            }
        }

        // Check command arguments for dangerous patterns
        if self.has_dangerous_arguments(command_str) {
            return false;
        }

        // Check resource limits
        if definition.resource_limits.is_some() {
            return false; // Resource limits require VM enforcement
        }

        // Check network requirement
        if definition
            .resource_limits
            .as_ref()
            .map(|limits| limits.network_access)
            .unwrap_or(false)
        {
            return false; // Network access requires VM isolation
        }

        true
    }

    /// Check for high-risk indicators in command
    fn has_high_risk_indicators(&self, command_str: &str) -> bool {
        // Check for suspicious patterns
        let suspicious_patterns = vec![
            "&&",
            "||",
            ";",
            "|",
            ">",
            ">>",
            "<",
            "<<",
            "$(",
            "`",
            "eval",
            "exec",
            "source",
            "/dev/",
            "/proc/",
            "/sys/",
            "/etc/",
            "chmod +x",
            "chown",
            "chgrp",
            "iptables",
            "ufw",
            "firewall",
            "systemctl",
            "service",
            "init",
            "shutdown",
            "reboot",
            "halt",
        ];

        for pattern in &suspicious_patterns {
            if command_str.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// Check for dangerous arguments
    fn has_dangerous_arguments(&self, command_str: &str) -> bool {
        let args: Vec<&str> = command_str.split_whitespace().collect();
        if args.len() < 2 {
            return false;
        }

        // Check arguments for dangerous patterns
        for arg in &args[1..] {
            if arg.starts_with('/')
                && (arg.contains("/etc/") || arg.contains("/dev/") || arg.contains("/proc/"))
            {
                return true;
            }
            if arg.contains("&&") || arg.contains("||") || arg.contains(";") {
                return true;
            }
            if arg.starts_with('$') || arg.contains('`') {
                return true;
            }
        }

        false
    }

    /// Extract command name from full command string
    fn extract_command_name(&self, command_str: &str) -> String {
        command_str
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string()
    }

    /// Get execution statistics
    pub async fn get_execution_stats(&self) -> ExecutionStats {
        // This would be implemented with actual statistics tracking
        ExecutionStats {
            total_executions: 0,
            local_executions: 0,
            vm_executions: 0,
            blocked_executions: 0,
            average_execution_time_ms: 0.0,
        }
    }
}

#[async_trait::async_trait]
impl super::CommandExecutor for HybridExecutor {
    async fn execute_command(
        &self,
        definition: &CommandDefinition,
        parameters: &HashMap<String, String>,
    ) -> Result<CommandExecutionResult, CommandExecutionError> {
        // Extract command string from parameters
        let command_str = parameters.get("command").ok_or_else(|| {
            CommandExecutionError::ExecutionFailed(
                "hybrid".to_string(),
                "Missing 'command' parameter".to_string(),
            )
        })?;

        // Determine execution mode
        let execution_mode = self.assess_command_risk(command_str, definition);

        // Execute with the appropriate executor
        match execution_mode {
            ExecutionMode::Local => {
                self.local_executor
                    .execute_command(definition, parameters)
                    .await
            }
            ExecutionMode::Firecracker => {
                self.firecracker_executor
                    .execute_command(definition, parameters)
                    .await
            }
            ExecutionMode::Hybrid => {
                // This shouldn't happen with proper risk assessment, but handle it
                self.local_executor
                    .execute_command(definition, parameters)
                    .await
            }
        }
    }

    fn supports_mode(&self, mode: &ExecutionMode) -> bool {
        // Hybrid executor supports all modes by delegating to appropriate executors
        match mode {
            ExecutionMode::Local => self.local_executor.supports_mode(mode),
            ExecutionMode::Firecracker => self.firecracker_executor.supports_mode(mode),
            ExecutionMode::Hybrid => true, // Hybrid mode is what this executor provides
        }
    }

    fn capabilities(&self) -> ExecutorCapabilities {
        // Combine capabilities from both executors
        let local_caps = self.local_executor.capabilities();
        let vm_caps = self.firecracker_executor.capabilities();

        ExecutorCapabilities {
            supports_resource_limits: vm_caps.supports_resource_limits, // VMs have better resource limiting
            supports_network_access: vm_caps.supports_network_access,
            supports_file_system: local_caps.supports_file_system || vm_caps.supports_file_system,
            max_concurrent_commands: Some(
                local_caps.max_concurrent_commands.unwrap_or(0)
                    + vm_caps.max_concurrent_commands.unwrap_or(0),
            ),
            default_timeout: vm_caps.default_timeout, // Use VM timeout as default for safety
        }
    }
}

/// Execution statistics
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    pub total_executions: u64,
    pub local_executions: u64,
    pub vm_executions: u64,
    pub blocked_executions: u64,
    pub average_execution_time_ms: f64,
}

impl Default for HybridExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::CommandDefinition;
    use super::super::ExecutionMode;
    use super::*;
    use crate::RiskLevel;

    #[test]
    fn test_risk_assessment_safe_commands() {
        let hybrid = HybridExecutor::new();

        let safe_definition = CommandDefinition {
            name: "test".to_string(),
            description: "Test command".to_string(),
            risk_level: RiskLevel::Low,
            execution_mode: ExecutionMode::Hybrid,
            ..Default::default()
        };

        let mode = hybrid.assess_command_risk("ls -la", &safe_definition);
        assert_eq!(mode, ExecutionMode::Local);
    }

    #[test]
    fn test_risk_assessment_high_risk_commands() {
        let hybrid = HybridExecutor::new();

        let risky_definition = CommandDefinition {
            name: "dangerous".to_string(),
            description: "Dangerous command".to_string(),
            risk_level: RiskLevel::High,
            execution_mode: ExecutionMode::Hybrid,
            ..Default::default()
        };

        let mode = hybrid.assess_command_risk("rm -rf /", &risky_definition);
        assert_eq!(mode, ExecutionMode::Firecracker);
    }

    #[test]
    fn test_dangerous_argument_detection() {
        let hybrid = HybridExecutor::new();

        assert!(hybrid.has_dangerous_arguments("rm -rf /etc/passwd"));
        assert!(hybrid.has_dangerous_arguments("cat /etc/shadow"));
        assert!(hybrid.has_dangerous_arguments("echo 'test; rm -rf /'"));
        assert!(!hybrid.has_dangerous_arguments("ls -la"));
        assert!(!hybrid.has_dangerous_arguments("echo hello"));
    }

    #[test]
    fn test_high_risk_keywords() {
        let hybrid = HybridExecutor::new();

        let settings = RiskAssessmentSettings::default();
        assert!(settings
            .high_risk_keywords
            .iter()
            .any(|k| command_str.contains(k)));
    }

    #[test]
    fn test_command_name_extraction() {
        let hybrid = HybridExecutor::new();

        assert_eq!(hybrid.extract_command_name("ls -la /tmp"), "ls");
        assert_eq!(
            hybrid.extract_command_name("python script.py --verbose"),
            "python"
        );
        assert_eq!(hybrid.extract_command_name("  echo  hello  "), "echo");
        assert_eq!(hybrid.extract_command_name(""), "");
    }
}
