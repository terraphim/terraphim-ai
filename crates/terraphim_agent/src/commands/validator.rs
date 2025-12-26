//! Command validation with knowledge graph integration
//!
//! This module provides validation for commands against knowledge graphs,
//! role permissions, and security policies.

use super::{CommandValidationError, ExecutionMode};
use crate::client::ApiClient;
use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Security event for auditing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub timestamp: SystemTime,
    pub user: String,
    pub command: String,
    pub action: SecurityAction,
    pub result: SecurityResult,
    pub details: String,
}

/// Security action types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityAction {
    CommandValidation,
    KnowledgeGraphCheck,
    PermissionCheck,
    RateLimitCheck,
    BlacklistCheck,
    TimeRestrictionCheck,
}

/// Security validation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityResult {
    Allowed,
    Denied(String),
    Warning(String),
}

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimit {
    pub max_requests: usize,
    pub window: Duration,
    pub current_requests: Vec<SystemTime>,
}

/// Time-based restrictions
#[derive(Debug, Clone)]
pub struct TimeRestrictions {
    pub allowed_hours: Vec<u8>, // 0-23 hours when commands are allowed
    pub allowed_days: Vec<u8>,  // 0-6 days (Sunday=0)
    pub maintenance_windows: Vec<MaintenanceWindow>,
}

/// Maintenance window definition
#[derive(Debug, Clone)]
pub struct MaintenanceWindow {
    pub start_day: u8,
    pub start_hour: u8,
    pub duration_hours: u8,
    pub reason: String,
}

/// Command validator that checks against knowledge graph and security policies
pub struct CommandValidator {
    /// API client for rolegraph access
    api_client: Option<Arc<ApiClient>>,
    /// Role-based permissions
    role_permissions: HashMap<String, Vec<String>>,
    /// Cached knowledge graph concepts per role
    concept_cache: HashMap<String, Vec<String>>,
    /// Security audit log
    audit_log: Vec<SecurityEvent>,
    /// Rate limiting per command
    rate_limits: HashMap<String, RateLimit>,
    /// Blacklisted commands
    blacklisted_commands: Vec<String>,
    /// Time-based restrictions
    time_restrictions: TimeRestrictions,
}

impl CommandValidator {
    /// Create a new command validator
    pub fn new() -> Self {
        let mut role_permissions = HashMap::new();

        // Initialize with default role permissions
        role_permissions.insert(
            "Default".to_string(),
            vec!["read".to_string(), "search".to_string(), "help".to_string()],
        );
        role_permissions.insert(
            "Terraphim Engineer".to_string(),
            vec![
                "read".to_string(),
                "write".to_string(),
                "execute".to_string(),
                "search".to_string(),
                "configure".to_string(),
                "help".to_string(),
            ],
        );

        // Initialize rate limits
        let mut rate_limits = HashMap::new();
        rate_limits.insert(
            "search".to_string(),
            RateLimit {
                max_requests: 100,
                window: Duration::from_secs(60),
                current_requests: Vec::new(),
            },
        );
        rate_limits.insert(
            "deploy".to_string(),
            RateLimit {
                max_requests: 5,
                window: Duration::from_secs(3600),
                current_requests: Vec::new(),
            },
        );

        // Initialize blacklisted commands
        let blacklisted_commands = vec![
            "rm -rf /".to_string(),
            "dd if=/dev/zero".to_string(),
            "mkfs".to_string(),
            "fdisk".to_string(),
        ];

        // Initialize time restrictions (business hours for production, permissive for testing)
        let time_restrictions = if cfg!(test) {
            // Permissive for testing - allow all hours and days
            TimeRestrictions {
                allowed_hours: vec![], // All hours allowed
                allowed_days: vec![],  // All days allowed
                maintenance_windows: vec![],
            }
        } else {
            // Business hours for production
            TimeRestrictions {
                allowed_hours: (9..=17).collect(), // 9 AM to 5 PM
                allowed_days: (1..=5).collect(),   // Monday to Friday
                maintenance_windows: Vec::new(),
            }
        };

        Self {
            api_client: None,
            role_permissions,
            concept_cache: HashMap::new(),
            audit_log: Vec::new(),
            rate_limits,
            blacklisted_commands,
            time_restrictions,
        }
    }

    /// Set rate limit for a specific command (for testing)
    pub fn set_rate_limit(&mut self, command: &str, max_requests: u32, window: Duration) {
        self.rate_limits.insert(
            command.to_string(),
            RateLimit {
                max_requests: max_requests as usize,
                window,
                current_requests: Vec::new(),
            },
        );
    }

    /// Create a new command validator with API client
    pub fn with_api_client(api_client: Arc<ApiClient>) -> Self {
        let mut validator = Self::new();
        validator.api_client = Some(api_client);
        validator
    }

    /// Validate if a command can be executed by the given role
    pub async fn validate_command_execution(
        &mut self,
        command: &str,
        role: &str,
        _parameters: &HashMap<String, String>,
    ) -> Result<ExecutionMode, CommandValidationError> {
        self.validate_command_execution_with_mode(command, role, _parameters, None)
            .await
    }

    /// Validate if a command can be executed by the given role, with optional execution mode override
    pub async fn validate_command_execution_with_mode(
        &mut self,
        command: &str,
        role: &str,
        _parameters: &HashMap<String, String>,
        definition_execution_mode: Option<ExecutionMode>,
    ) -> Result<ExecutionMode, CommandValidationError> {
        // Check if role has required permissions
        if let Some(permissions) = self.role_permissions.get(role) {
            // Check all required permissions using has_required_permissions
            if !self.has_required_permissions(command, permissions) {
                return Err(CommandValidationError::InsufficientPermissions(format!(
                    "'{}' role lacks required permissions for command",
                    role
                )));
            }
        }

        // Check if command exists in knowledge graph
        if self.api_client.is_some() {
            let kg_concepts = self.get_knowledge_graph_concepts(role).await?;
            let command_concept = self.extract_command_concept(command);

            if !kg_concepts.is_empty() && !kg_concepts.contains(&command_concept) {
                return Err(CommandValidationError::MissingKnowledgeGraphConcepts(
                    command.to_string(),
                    vec![command_concept],
                ));
            }
        }

        // Determine execution mode based on risk assessment and optional definition override
        let execution_mode =
            self.determine_execution_mode_with_override(command, role, definition_execution_mode);

        Ok(execution_mode)
    }

    /// Get knowledge graph concepts for a role
    async fn get_knowledge_graph_concepts(
        &mut self,
        role: &str,
    ) -> Result<Vec<String>, CommandValidationError> {
        // Check cache first
        if let Some(cached_concepts) = self.concept_cache.get(role) {
            return Ok(cached_concepts.clone());
        }

        // Fetch from API
        if let Some(api_client) = &self.api_client {
            match api_client.get_rolegraph_edges(Some(role)).await {
                Ok(rolegraph_response) => {
                    let concepts: Vec<String> = rolegraph_response
                        .nodes
                        .into_iter()
                        .map(|node| node.label.to_lowercase())
                        .collect();

                    // Cache the concepts
                    self.concept_cache
                        .insert(role.to_string(), concepts.clone());
                    Ok(concepts)
                }
                Err(e) => {
                    // Log warning but allow execution without knowledge graph validation
                    eprintln!("Warning: Failed to fetch rolegraph for '{}': {}", role, e);
                    Ok(vec![])
                }
            }
        } else {
            Ok(vec![])
        }
    }

    /// Check if a command performs write operations
    fn is_write_operation(&self, command: &str) -> bool {
        let write_commands = vec![
            "rm", "mv", "cp", "touch", "mkdir", "rmdir", "chmod", "chown", "write", "create",
            "delete", "update", "modify", "edit",
        ];

        write_commands.iter().any(|cmd| command.contains(cmd))
    }

    /// Extract command concept for knowledge graph validation
    fn extract_command_concept(&self, command: &str) -> String {
        command
            .split_whitespace()
            .next()
            .unwrap_or("unknown")
            .to_lowercase()
    }

    /// Determine execution mode based on command and role
    #[allow(dead_code)]
    fn determine_execution_mode(&self, command: &str, role: &str) -> ExecutionMode {
        self.determine_execution_mode_with_override(command, role, None)
    }

    /// Determine execution mode with optional override from command definition
    fn determine_execution_mode_with_override(
        &self,
        command: &str,
        role: &str,
        definition_mode: Option<ExecutionMode>,
    ) -> ExecutionMode {
        // High-risk commands always use firecracker, regardless of definition
        if self.is_high_risk_command(command) {
            return ExecutionMode::Firecracker;
        }

        // If command definition specifies an execution mode, respect it for non-high-risk commands
        if let Some(mode) = definition_mode {
            return mode;
        }

        // Safe commands can use local execution for engineers
        if role == "Terraphim Engineer" && self.is_safe_command(command) {
            return ExecutionMode::Local;
        }

        // Default to hybrid mode
        ExecutionMode::Hybrid
    }

    /// Check if command is high risk
    fn is_high_risk_command(&self, command: &str) -> bool {
        let high_risk_patterns = [
            "rm -rf",
            "dd if=",
            "mkfs",
            "fdisk",
            "iptables",
            "systemctl",
            "shutdown",
            "reboot",
            "passwd",
            "chown root",
            "chmod 777",
        ];

        high_risk_patterns
            .iter()
            .any(|pattern| command.contains(pattern))
    }

    /// Check if command is safe for local execution
    fn is_safe_command(&self, command: &str) -> bool {
        let safe_commands = [
            "ls", "cat", "echo", "pwd", "date", "whoami", "grep", "find", "head", "tail", "wc",
            "sort",
        ];

        safe_commands.iter().any(|cmd| command.starts_with(cmd))
    }

    /// Check if command is a system command
    fn is_system_command(&self, command: &str) -> bool {
        let system_commands = [
            "systemctl",
            "shutdown",
            "reboot",
            "passwd",
            "chown",
            "chmod",
            "iptables",
            "fdisk",
            "mkfs",
        ];

        system_commands.iter().any(|cmd| command.starts_with(cmd))
    }

    /// Add role permissions
    pub fn add_role_permissions(&mut self, role: String, permissions: Vec<String>) {
        self.role_permissions.insert(role, permissions);
    }

    /// Check if command is blacklisted
    pub fn is_blacklisted(&self, command: &str) -> bool {
        self.blacklisted_commands
            .iter()
            .any(|blacklisted| command.contains(blacklisted) || command.starts_with(blacklisted))
    }

    /// Check rate limiting for command
    pub fn check_rate_limit(&mut self, command: &str) -> Result<(), CommandValidationError> {
        let command_name = self.extract_command_concept(command);

        if let Some(rate_limit) = self.rate_limits.get_mut(&command_name) {
            let now = SystemTime::now();

            // Clean old requests outside the window
            rate_limit.current_requests.retain(|&timestamp| {
                now.duration_since(timestamp).unwrap_or(Duration::MAX) < rate_limit.window
            });

            // Check if under limit
            if rate_limit.current_requests.len() >= rate_limit.max_requests {
                return Err(CommandValidationError::ValidationFailed(format!(
                    "Rate limit exceeded for command '{}': {}/{} requests per {:?}",
                    command_name,
                    rate_limit.current_requests.len(),
                    rate_limit.max_requests,
                    rate_limit.window
                )));
            }

            // Add current request
            rate_limit.current_requests.push(now);
        }

        Ok(())
    }

    /// Check time-based restrictions
    pub fn check_time_restrictions(&self) -> Result<(), CommandValidationError> {
        let now = std::time::SystemTime::now();
        let datetime = chrono::DateTime::<chrono::Utc>::from(now);

        // Convert to local time
        let local_time = datetime.with_timezone(&chrono::Local);

        // Check day restrictions
        if !self.time_restrictions.allowed_days.is_empty()
            && !self
                .time_restrictions
                .allowed_days
                .contains(&(local_time.weekday().num_days_from_sunday() as u8))
        {
            return Err(CommandValidationError::ValidationFailed(
                "Commands not allowed on this day".to_string(),
            ));
        }

        // Check hour restrictions
        if !self.time_restrictions.allowed_hours.is_empty()
            && !self
                .time_restrictions
                .allowed_hours
                .contains(&(local_time.hour() as u8))
        {
            return Err(CommandValidationError::ValidationFailed(format!(
                "Commands not allowed at this time: {}:00",
                local_time.hour()
            )));
        }

        Ok(())
    }

    /// Log security event
    pub fn log_security_event(
        &mut self,
        user: &str,
        command: &str,
        action: SecurityAction,
        result: SecurityResult,
        details: &str,
    ) {
        let event = SecurityEvent {
            timestamp: SystemTime::now(),
            user: user.to_string(),
            command: command.to_string(),
            action,
            result: result.clone(),
            details: details.to_string(),
        };

        self.audit_log.push(event);

        // Keep only last 1000 events
        if self.audit_log.len() > 1000 {
            self.audit_log.drain(0..100);
        }
    }

    /// Get recent security events
    pub fn get_recent_events(&self, limit: usize) -> Vec<&SecurityEvent> {
        self.audit_log.iter().rev().take(limit).collect()
    }

    /// Comprehensive security validation
    pub async fn validate_command_security(
        &mut self,
        command: &str,
        role: &str,
        user: &str,
    ) -> Result<(), CommandValidationError> {
        // 1. Check blacklist
        if self.is_blacklisted(command) {
            let details = format!("Command '{}' is blacklisted for security reasons", command);
            self.log_security_event(
                user,
                command,
                SecurityAction::BlacklistCheck,
                SecurityResult::Denied(details.clone()),
                &details,
            );
            return Err(CommandValidationError::ValidationFailed(details));
        }
        self.log_security_event(
            user,
            command,
            SecurityAction::BlacklistCheck,
            SecurityResult::Allowed,
            "Command not blacklisted",
        );

        // 2. Check rate limits
        if let Err(e) = self.check_rate_limit(command) {
            self.log_security_event(
                user,
                command,
                SecurityAction::RateLimitCheck,
                SecurityResult::Denied(e.to_string()),
                "Rate limit exceeded",
            );
            return Err(e);
        }
        self.log_security_event(
            user,
            command,
            SecurityAction::RateLimitCheck,
            SecurityResult::Allowed,
            "Rate limit check passed",
        );

        // 3. Check time restrictions
        if let Err(e) = self.check_time_restrictions() {
            self.log_security_event(
                user,
                command,
                SecurityAction::TimeRestrictionCheck,
                SecurityResult::Denied(e.to_string()),
                "Time restriction violation",
            );
            return Err(e);
        }
        self.log_security_event(
            user,
            command,
            SecurityAction::TimeRestrictionCheck,
            SecurityResult::Allowed,
            "Time restrictions satisfied",
        );

        // 4. Check role permissions
        if let Some(permissions) = self.role_permissions.get(role) {
            if !self.has_required_permissions(command, permissions) {
                let details = format!("Role '{}' lacks required permissions for command", role);
                self.log_security_event(
                    user,
                    command,
                    SecurityAction::PermissionCheck,
                    SecurityResult::Denied(details.clone()),
                    &details,
                );
                return Err(CommandValidationError::InsufficientPermissions(details));
            }
        }
        self.log_security_event(
            user,
            command,
            SecurityAction::PermissionCheck,
            SecurityResult::Allowed,
            "Role permissions verified",
        );

        Ok(())
    }

    /// Check if user has required permissions for command
    fn has_required_permissions(&self, command: &str, permissions: &[String]) -> bool {
        if self.is_write_operation(command) && !permissions.contains(&"write".to_string()) {
            return false;
        }

        if self.is_high_risk_command(command) && !permissions.contains(&"execute".to_string()) {
            return false;
        }

        // Additional check: system commands should not be executable by default role
        if self.is_system_command(command) && !permissions.contains(&"execute".to_string()) {
            return false;
        }

        true
    }

    /// Get security statistics
    pub fn get_security_stats(&self) -> SecurityStats {
        let total_events = self.audit_log.len();
        let denied_events = self
            .audit_log
            .iter()
            .filter(|e| matches!(e.result, SecurityResult::Denied(_)))
            .count();
        let recent_hour = SystemTime::now()
            .checked_sub(Duration::from_secs(3600))
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let recent_events = self
            .audit_log
            .iter()
            .filter(|e| e.timestamp > recent_hour)
            .count();

        SecurityStats {
            total_events,
            denied_events,
            recent_events,
            active_rate_limits: self.rate_limits.len(),
            blacklisted_commands: self.blacklisted_commands.len(),
        }
    }
}

impl Default for CommandValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Security statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityStats {
    pub total_events: usize,
    pub denied_events: usize,
    pub recent_events: usize,
    pub active_rate_limits: usize,
    pub blacklisted_commands: usize,
}
