//! Command registry for managing markdown-defined commands
//!
//! This module provides the command registry that handles loading, storing, and managing
//! command definitions discovered from markdown files.

use super::{CommandDefinition, CommandRegistryError, ExecutionMode, ParsedCommand, RiskLevel};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Command registry that manages all discovered commands
#[derive(Debug)]
pub struct CommandRegistry {
    /// Registry storage
    commands: Arc<RwLock<HashMap<String, Arc<ParsedCommand>>>>,
    /// Command aliases mapping
    aliases: Arc<RwLock<HashMap<String, String>>>,
    /// Category-based command mapping
    categories: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Command parser
    parser: super::markdown_parser::MarkdownCommandParser,
    /// Command directories to watch
    command_directories: Vec<PathBuf>,
}

impl CommandRegistry {
    /// Create a new command registry
    pub fn new() -> Result<Self, CommandRegistryError> {
        let parser = super::markdown_parser::MarkdownCommandParser::new()?;

        Ok(Self {
            commands: Arc::new(RwLock::new(HashMap::new())),
            aliases: Arc::new(RwLock::new(HashMap::new())),
            categories: Arc::new(RwLock::new(HashMap::new())),
            parser,
            command_directories: Vec::new(),
        })
    }

    /// Add a command directory to watch
    pub fn add_command_directory<P: AsRef<Path>>(&mut self, directory: P) {
        self.command_directories
            .push(directory.as_ref().to_path_buf());
    }

    /// Load all commands from configured directories
    pub async fn load_all_commands(&self) -> Result<usize, CommandRegistryError> {
        let mut total_loaded = 0;

        for directory in &self.command_directories {
            if directory.exists() {
                match self.parser.parse_directory(directory).await {
                    Ok(commands) => {
                        for command in commands {
                            self.register_command(command).await?;
                            total_loaded += 1;
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to load commands from {}: {}",
                            directory.display(),
                            e
                        );
                    }
                }
            } else {
                eprintln!(
                    "Warning: Command directory does not exist: {}",
                    directory.display()
                );
            }
        }

        Ok(total_loaded)
    }

    /// Register a single command in the registry
    pub async fn register_command(
        &self,
        command: ParsedCommand,
    ) -> Result<(), CommandRegistryError> {
        let name = command.definition.name.clone();

        // Check for duplicates
        {
            let commands = self.commands.read().await;
            if commands.contains_key(&name) {
                return Err(CommandRegistryError::DuplicateCommand(name));
            }
        }

        // Register the command
        {
            let mut commands = self.commands.write().await;
            commands.insert(name.clone(), Arc::new(command.clone()));
        }

        // Register aliases
        {
            let mut aliases = self.aliases.write().await;
            for alias in &command.definition.aliases {
                aliases.insert(alias.clone(), name.clone());
            }
        }

        // Register category
        if let Some(ref category) = command.definition.category {
            let mut categories = self.categories.write().await;
            categories
                .entry(category.clone())
                .or_insert_with(Vec::new)
                .push(name.clone());
        }

        Ok(())
    }

    /// Get a command by name
    pub async fn get_command(&self, name: &str) -> Option<Arc<ParsedCommand>> {
        let commands = self.commands.read().await;
        commands.get(name).cloned()
    }

    /// Get a command by name (checking aliases)
    pub async fn resolve_command(&self, name: &str) -> Option<Arc<ParsedCommand>> {
        // First check direct name
        if let Some(command) = self.get_command(name).await {
            return Some(command);
        }

        // Then check aliases
        let aliases = self.aliases.read().await;
        if let Some(resolved_name) = aliases.get(name) {
            let commands = self.commands.read().await;
            return commands.get(resolved_name).cloned();
        }

        None
    }

    /// List all available command names
    pub async fn list_commands(&self) -> Vec<String> {
        let commands = self.commands.read().await;
        commands.keys().cloned().collect()
    }

    /// List commands by category
    pub async fn list_commands_by_category(&self, category: &str) -> Vec<String> {
        let categories = self.categories.read().await;
        categories.get(category).cloned().unwrap_or_default()
    }

    /// List all categories
    pub async fn list_categories(&self) -> Vec<String> {
        let categories = self.categories.read().await;
        categories.keys().cloned().collect()
    }

    /// Search commands by name or description
    pub async fn search_commands(&self, query: &str) -> Vec<Arc<ParsedCommand>> {
        let commands = self.commands.read().await;
        let query_lower = query.to_lowercase();

        commands
            .values()
            .filter(|cmd| {
                cmd.definition.name.to_lowercase().contains(&query_lower)
                    || cmd
                        .definition
                        .description
                        .to_lowercase()
                        .contains(&query_lower)
                    || cmd.content.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect()
    }

    /// Get commands by execution mode
    pub async fn get_commands_by_execution_mode(
        &self,
        mode: ExecutionMode,
    ) -> Vec<Arc<ParsedCommand>> {
        let commands = self.commands.read().await;
        commands
            .values()
            .filter(|cmd| cmd.definition.execution_mode == mode)
            .cloned()
            .collect()
    }

    /// Get commands by risk level
    pub async fn get_commands_by_risk_level(
        &self,
        risk_level: RiskLevel,
    ) -> Vec<Arc<ParsedCommand>> {
        let commands = self.commands.read().await;
        commands
            .values()
            .filter(|cmd| cmd.definition.risk_level == risk_level)
            .cloned()
            .collect()
    }

    /// Reload commands from directories (useful for development)
    pub async fn reload(&self) -> Result<usize, CommandRegistryError> {
        // Clear existing commands
        {
            let mut commands = self.commands.write().await;
            commands.clear();
        }
        {
            let mut aliases = self.aliases.write().await;
            aliases.clear();
        }
        {
            let mut categories = self.categories.write().await;
            categories.clear();
        }

        // Reload all commands
        self.load_all_commands().await
    }

    /// Get registry statistics
    pub async fn get_stats(&self) -> RegistryStats {
        let commands = self.commands.read().await;
        let aliases = self.aliases.read().await;
        let categories = self.categories.read().await;

        let mut risk_level_counts = HashMap::new();
        let mut execution_mode_counts = HashMap::new();

        for command in commands.values() {
            *risk_level_counts
                .entry(command.definition.risk_level.clone())
                .or_insert(0) += 1;
            *execution_mode_counts
                .entry(command.definition.execution_mode.clone())
                .or_insert(0) += 1;
        }

        RegistryStats {
            total_commands: commands.len(),
            total_aliases: aliases.len(),
            total_categories: categories.len(),
            risk_level_counts,
            execution_mode_counts,
        }
    }

    /// Validate command parameters against a definition
    pub async fn validate_parameters(
        &self,
        command_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<(), super::CommandValidationError> {
        let command = self.resolve_command(command_name).await.ok_or_else(|| {
            super::CommandValidationError::CommandNotFound(command_name.to_string())
        })?;

        // Check required parameters
        for param in &command.definition.parameters {
            if param.required && !parameters.contains_key(&param.name) {
                return Err(super::CommandValidationError::MissingParameter(
                    param.name.clone(),
                ));
            }

            if let Some(value) = parameters.get(&param.name) {
                // Validate parameter type
                if let Err(e) = self.validate_parameter_type(param, value) {
                    return Err(super::CommandValidationError::InvalidParameter(
                        param.name.clone(),
                        e,
                    ));
                }

                // Validate parameter constraints
                if let Err(e) = self.validate_parameter_constraints(param, value) {
                    return Err(super::CommandValidationError::InvalidParameter(
                        param.name.clone(),
                        e,
                    ));
                }
            }
        }

        Ok(())
    }

    /// Validate parameter type
    fn validate_parameter_type(
        &self,
        param: &super::CommandParameter,
        value: &serde_json::Value,
    ) -> Result<(), String> {
        match param.param_type.as_str() {
            "string" => {
                if !value.is_string() {
                    return Err("Expected string".to_string());
                }
            }
            "number" => {
                if !value.is_number() {
                    return Err("Expected number".to_string());
                }
            }
            "boolean" => {
                if !value.is_boolean() {
                    return Err("Expected boolean".to_string());
                }
            }
            "array" => {
                if !value.is_array() {
                    return Err("Expected array".to_string());
                }
            }
            "object" => {
                if !value.is_object() {
                    return Err("Expected object".to_string());
                }
            }
            _ => {
                return Err(format!("Unknown parameter type: {}", param.param_type));
            }
        }

        Ok(())
    }

    /// Validate parameter constraints
    fn validate_parameter_constraints(
        &self,
        param: &super::CommandParameter,
        value: &serde_json::Value,
    ) -> Result<(), String> {
        if let Some(ref validation) = param.validation {
            // Validate min/max for numbers
            if value.is_number() {
                let num = value.as_f64().unwrap();
                if let Some(min) = validation.min {
                    if num < min {
                        return Err(format!("Value {} is below minimum {}", num, min));
                    }
                }
                if let Some(max) = validation.max {
                    if num > max {
                        return Err(format!("Value {} is above maximum {}", num, max));
                    }
                }
            }

            // Validate allowed values
            if let Some(ref allowed_values) = validation.allowed_values {
                let value_str = match value {
                    serde_json::Value::String(s) => s.clone(),
                    _ => value.to_string(),
                };
                if !allowed_values.contains(&value_str) {
                    return Err(format!(
                        "Value '{}' not in allowed values: {:?}",
                        value_str, allowed_values
                    ));
                }
            }

            // Validate regex pattern for strings
            if let Some(ref pattern) = validation.pattern {
                if let Some(s) = value.as_str() {
                    let regex = regex::Regex::new(pattern)
                        .map_err(|e| format!("Invalid regex pattern: {}", e))?;
                    if !regex.is_match(s) {
                        return Err(format!("Value '{}' does not match pattern: {}", s, pattern));
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if a command can be executed with the given role and permissions
    pub async fn can_execute_command(
        &self,
        command_name: &str,
        role: &str,
        user_permissions: &[String],
    ) -> Result<ExecutionMode, super::CommandValidationError> {
        let command = self.resolve_command(command_name).await.ok_or_else(|| {
            super::CommandValidationError::CommandNotFound(command_name.to_string())
        })?;

        // Check permissions (basic implementation - could be enhanced)
        for required_permission in &command.definition.permissions {
            if !user_permissions.contains(required_permission) {
                return Err(super::CommandValidationError::InsufficientPermissions(
                    command_name.to_string(),
                ));
            }
        }

        // Return the execution mode from the command definition
        Ok(command.definition.execution_mode.clone())
    }

    /// Suggest commands based on partial input
    pub async fn suggest_commands(
        &self,
        partial: &str,
        limit: Option<usize>,
    ) -> Vec<Arc<ParsedCommand>> {
        let commands = self.commands.read().await;
        let partial_lower = partial.to_lowercase();

        let mut matches: Vec<(Arc<ParsedCommand>, usize)> = commands
            .values()
            .filter_map(|cmd| {
                let score = self.calculate_match_score(&cmd.definition.name, &partial_lower);
                if score > 0 {
                    Some((cmd.clone(), score))
                } else {
                    None
                }
            })
            .collect();

        // Sort by score (descending) and take top matches
        matches.sort_by(|a, b| b.1.cmp(&a.1));

        let limit = limit.unwrap_or(10);
        matches
            .into_iter()
            .take(limit)
            .map(|(cmd, _)| cmd)
            .collect()
    }

    /// Calculate match score for command suggestion
    fn calculate_match_score(&self, command_name: &str, partial: &str) -> usize {
        if command_name.to_lowercase().starts_with(partial) {
            // Exact prefix match gets highest score
            100
        } else if command_name.to_lowercase().contains(partial) {
            // Contains match gets medium score
            50
        } else {
            // Calculate Levenshtein distance for fuzzy matching
            let distance = levenshtein_distance(command_name.to_lowercase().as_str(), partial);
            if distance <= 2 {
                100 - (distance * 20)
            } else {
                0
            }
        }
    }
}

/// Registry statistics
#[derive(Debug, Clone)]
pub struct RegistryStats {
    pub total_commands: usize,
    pub total_aliases: usize,
    pub total_categories: usize,
    pub risk_level_counts: HashMap<RiskLevel, usize>,
    pub execution_mode_counts: HashMap<ExecutionMode, usize>,
}

/// Simple Levenshtein distance implementation for fuzzy matching
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_bytes = s1.as_bytes();
    let s2_bytes = s2.as_bytes();
    let len1 = s1_bytes.len();
    let len2 = s2_bytes.len();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_bytes[i - 1] == s2_bytes[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = std::cmp::min(
                std::cmp::min(
                    matrix[i - 1][j] + 1, // deletion
                    matrix[i][j - 1] + 1, // insertion
                ),
                matrix[i - 1][j - 1] + cost, // substitution
            );
        }
    }

    matrix[len1][len2]
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new().expect("Failed to create CommandRegistry")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    // Import required types for testing
    use crate::commands::{CommandDefinition, CommandParameter, ExecutionMode, ParsedCommand};

    #[tokio::test]
    async fn test_register_command() {
        let registry = CommandRegistry::new().unwrap();

        let command = ParsedCommand {
            definition: CommandDefinition {
                name: "test".to_string(),
                description: "Test command".to_string(),
                execution_mode: ExecutionMode::Local,
                ..Default::default()
            },
            content: "Test content".to_string(),
            source_path: PathBuf::from("test.md"),
            modified: SystemTime::UNIX_EPOCH,
        };

        assert!(registry.register_command(command).await.is_ok());
        assert!(registry.get_command("test").await.is_some());
    }

    #[tokio::test]
    async fn test_duplicate_command() {
        let registry = CommandRegistry::new().unwrap();

        let command = ParsedCommand {
            definition: CommandDefinition {
                name: "test".to_string(),
                description: "Test command".to_string(),
                execution_mode: ExecutionMode::Local,
                ..Default::default()
            },
            content: "Test content".to_string(),
            source_path: PathBuf::from("test.md"),
            modified: SystemTime::UNIX_EPOCH,
        };

        assert!(registry.register_command(command.clone()).await.is_ok());
        assert!(registry.register_command(command).await.is_err());
    }

    #[tokio::test]
    async fn test_command_validation() {
        let registry = CommandRegistry::new().unwrap();

        let command = ParsedCommand {
            definition: CommandDefinition {
                name: "test".to_string(),
                description: "Test command".to_string(),
                execution_mode: ExecutionMode::Local,
                parameters: vec![CommandParameter {
                    name: "name".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: Some("Name parameter".to_string()),
                    ..Default::default()
                }],
                ..Default::default()
            },
            content: "Test content".to_string(),
            source_path: PathBuf::from("test.md"),
            modified: SystemTime::UNIX_EPOCH,
        };

        registry.register_command(command).await.unwrap();

        // Test valid parameters
        let mut params = HashMap::new();
        params.insert(
            "name".to_string(),
            serde_json::Value::String("test".to_string()),
        );
        assert!(registry.validate_parameters("test", &params).await.is_ok());

        // Test missing required parameter
        let empty_params = HashMap::new();
        assert!(registry
            .validate_parameters("test", &empty_params)
            .await
            .is_err());

        // Test invalid type
        let mut invalid_params = HashMap::new();
        invalid_params.insert("name".to_string(), serde_json::Value::Number(42.into()));
        assert!(registry
            .validate_parameters("test", &invalid_params)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_command_suggestions() {
        let registry = CommandRegistry::new().unwrap();

        // Register multiple commands
        let commands = vec![
            ("hello-world", "Say hello"),
            ("help-me", "Get help"),
            ("test", "Test command"),
        ];

        for (name, desc) in commands {
            let command = ParsedCommand {
                definition: CommandDefinition {
                    name: name.to_string(),
                    description: desc.to_string(),
                    execution_mode: ExecutionMode::Local,
                    ..Default::default()
                },
                content: "Content".to_string(),
                source_path: PathBuf::from(format!("{}.md", name)),
                modified: SystemTime::UNIX_EPOCH,
            };
            registry.register_command(command).await.unwrap();
        }

        // Test exact prefix match
        let suggestions = registry.suggest_commands("hello", None).await;
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].definition.name, "hello-world");

        // Test partial match
        let suggestions = registry.suggest_commands("hel", None).await;
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].definition.name, "hello-world");

        // Test fuzzy match
        let suggestions = registry.suggest_commands("hlp", None).await;
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].definition.name, "help-me");
    }
}
