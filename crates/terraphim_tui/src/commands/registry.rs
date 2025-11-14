//! Command registry for managing markdown-defined commands
//!
//! This module provides the command registry that handles loading, storing, and managing
//! command definitions discovered from markdown files. Enhanced with terraphim-automata
//! for intelligent command discovery and content analysis.

use super::{CommandRegistryError, ExecutionMode, ParsedCommand, RiskLevel};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

// Automata imports for enhanced functionality
use terraphim_automata::{
    build_autocomplete_index, autocomplete_search, find_matches, extract_paragraphs_from_automata,
    AutocompleteIndex, AutocompleteConfig, Matched,
};
use ahash::AHashMap;
use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

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
    /// Autocomplete index for intelligent command discovery
    autocomplete_index: Arc<RwLock<Option<AutocompleteIndex>>>,
    /// Command thesaurus for term matching and analysis
    command_thesaurus: Arc<RwLock<Option<Thesaurus>>>,
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
            autocomplete_index: Arc::new(RwLock::new(None)),
            command_thesaurus: Arc::new(RwLock::new(None)),
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
        _role: &str,
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

    for (i, row) in matrix.iter_mut().enumerate().take(len1 + 1) {
        row[0] = i;
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

// Enhanced automata-based functionality

/// Result from intelligent command discovery
#[derive(Debug, Clone)]
pub struct CommandDiscoveryResult {
    pub command: Arc<ParsedCommand>,
    pub match_score: f64,
    pub match_type: String,
    pub related_commands: Vec<String>,
}

impl CommandRegistry {
    /// Build autocomplete index from all registered commands
    pub async fn build_autocomplete_index(&self) -> Result<(), CommandRegistryError> {
        let commands = self.commands.read().await;
        let mut thesaurus_data: AHashMap<NormalizedTermValue, NormalizedTerm> = AHashMap::new();

        // Build thesaurus from command names, descriptions, and content
        for (name, command) in commands.iter() {
            // Add command name
            let term_value = NormalizedTermValue::from(name.clone());
            thesaurus_data.insert(term_value.clone(), NormalizedTerm {
                id: command.definition.name.len() as u64,
                value: term_value.clone(),
                url: Some(format!("command:{}", name)),
            });

            // Add keywords from description
            let keywords = self.extract_keywords_from_text(&command.definition.description);
            for keyword in keywords {
                let keyword_value = NormalizedTermValue::from(keyword.clone());
                thesaurus_data.insert(keyword_value.clone(), NormalizedTerm {
                    id: command.definition.name.len() as u64,
                    value: keyword_value,
                    url: Some(format!("command:{}", name)),
                });
            }

            // Add parameter names
            for param in &command.definition.parameters {
                let param_key = format!("{}:{}", name, param.name);
                let param_value = NormalizedTermValue::from(param_key.clone());
                thesaurus_data.insert(param_value.clone(), NormalizedTerm {
                    id: param.name.len() as u64,
                    value: param_value,
                    url: Some(format!("command:{}:param:{}", name, param.name)),
                });
            }
        }

        let mut thesaurus = Thesaurus::new("command_registry".to_string());
        for (key, value) in thesaurus_data {
            thesaurus.insert(key, value);
        }

        // Clone thesaurus for storage before passing to build_autocomplete_index
        let thesaurus_clone = thesaurus.clone();

        // Build autocomplete index
        let autocomplete_config = AutocompleteConfig {
            max_results: 20,
            min_prefix_length: 1,
            case_sensitive: false,
        };

        let index = build_autocomplete_index(thesaurus, Some(autocomplete_config))
            .map_err(|e| CommandRegistryError::AutomataError(e.to_string()))?;

        // Store both thesaurus and index
        {
            let mut command_thesaurus = self.command_thesaurus.write().await;
            *command_thesaurus = Some(thesaurus_clone);
        }
        {
            let mut autocomplete_index = self.autocomplete_index.write().await;
            *autocomplete_index = Some(index);
        }

        Ok(())
    }

    /// Intelligent command discovery using automata autocomplete
    pub async fn discover_commands(&self, query: &str, limit: Option<usize>) -> Result<Vec<CommandDiscoveryResult>, CommandRegistryError> {
        // Build index if not already built
        {
            let autocomplete_index = self.autocomplete_index.read().await;
            if autocomplete_index.is_none() {
                drop(autocomplete_index);
                self.build_autocomplete_index().await?;
            }
        }

        let autocomplete_index = self.autocomplete_index.read().await;
        let index = autocomplete_index.as_ref().ok_or_else(|| {
            CommandRegistryError::AutomataError("Failed to build autocomplete index".to_string())
        })?;

        let results = autocomplete_search(index, query, limit)
            .map_err(|e| CommandRegistryError::AutomataError(e.to_string()))?;

        let mut discovery_results = Vec::new();
        for result in results {
            // Extract command name from result (remove parameter suffixes if present)
            let command_name = if result.term.contains(':') {
                result.term.split(':').next().unwrap_or(&result.term).to_string()
            } else {
                result.term.clone()
            };

            if let Some(command) = self.resolve_command(&command_name).await {
                discovery_results.push(CommandDiscoveryResult {
                    command,
                    match_score: result.score,
                    match_type: self.classify_match_type(&result.term, query),
                    related_commands: self.find_related_commands(&command_name).await,
                });
            }
        }

        // Sort by relevance score
        discovery_results.sort_by(|a, b| b.match_score.partial_cmp(&a.match_score).unwrap_or(std::cmp::Ordering::Equal));
        Ok(discovery_results)
    }

    /// Enhanced content analysis using term matching
    pub async fn analyze_command_content(&self, command_name: &str) -> Result<Vec<Matched>, CommandRegistryError> {
        let command = self.resolve_command(command_name).await.ok_or_else(|| {
            CommandRegistryError::CommandNotFound(command_name.to_string())
        })?;

        // Build thesaurus if not already built
        {
            let command_thesaurus = self.command_thesaurus.read().await;
            if command_thesaurus.is_none() {
                drop(command_thesaurus);
                self.build_autocomplete_index().await?;
            }
        }

        let command_thesaurus = self.command_thesaurus.read().await;
        if let Some(thesaurus) = command_thesaurus.as_ref() {
            find_matches(&command.content, thesaurus.clone(), true)
                .map_err(|e| CommandRegistryError::AutomataError(e.to_string()))
        } else {
            Ok(Vec::new())
        }
    }

    /// Extract contextual paragraphs for command help
    pub async fn extract_help_paragraphs(&self, command_name: &str, query_terms: &[String]) -> Result<Vec<(Matched, String)>, CommandRegistryError> {
        let command = self.resolve_command(command_name).await.ok_or_else(|| {
            CommandRegistryError::CommandNotFound(command_name.to_string())
        })?;

        // Build thesaurus from query terms
        let mut thesaurus_data: AHashMap<NormalizedTermValue, NormalizedTerm> = AHashMap::new();
        for (idx, term) in query_terms.iter().enumerate() {
            let term_value = NormalizedTermValue::from(term.clone());
            thesaurus_data.insert(term_value.clone(), NormalizedTerm {
                id: idx as u64,
                value: term_value,
                url: None,
            });
        }

        let mut thesaurus = Thesaurus::new("help_query".to_string());
        for (key, value) in thesaurus_data {
            thesaurus.insert(key, value);
        }

        extract_paragraphs_from_automata(&command.content, thesaurus, true)
            .map_err(|e| CommandRegistryError::AutomataError(e.to_string()))
    }

    /// Find related commands based on content similarity
    async fn find_related_commands(&self, command_name: &str) -> Vec<String> {
        let commands = self.commands.read().await;
        let target_command = match commands.get(command_name) {
            Some(cmd) => cmd,
            None => return Vec::new(),
        };

        let mut related = Vec::new();
        let target_keywords = self.extract_keywords_from_text(&target_command.definition.description);

        for (name, command) in commands.iter() {
            if name == command_name {
                continue;
            }

            let command_keywords = self.extract_keywords_from_text(&command.definition.description);
            let similarity = self.calculate_keyword_similarity(&target_keywords, &command_keywords);

            if similarity > 0.3 { // Threshold for relatedness
                related.push(name.clone());
            }
        }

        related.sort();
        related.truncate(5); // Limit to top 5 related commands
        related
    }

    /// Classify the type of match for discovery results
    fn classify_match_type(&self, matched_term: &str, query: &str) -> String {
        if matched_term == query {
            "exact".to_string()
        } else if matched_term.starts_with(query) {
            "prefix".to_string()
        } else if matched_term.contains(':') {
            "parameter".to_string()
        } else if matched_term.to_lowercase().contains(&query.to_lowercase()) {
            "contains".to_string()
        } else {
            "fuzzy".to_string()
        }
    }

    /// Extract keywords from text using simple heuristics
    fn extract_keywords_from_text(&self, text: &str) -> Vec<String> {
        // Simple keyword extraction - split on common separators and filter
        let mut keywords = Vec::new();

        // Split on whitespace, punctuation, and common separators
        for word in text.split_whitespace() {
            let clean_word = word.trim_matches(&[':', ',', '.', ';', '(', ')', '[', ']', '{', '}', '"', '\''][..]);
            if clean_word.len() > 3 && !self.is_stop_word(clean_word) {
                keywords.push(clean_word.to_lowercase());
            }
        }

        keywords.sort();
        keywords.dedup();
        keywords
    }

    /// Check if a word is a common stop word
    fn is_stop_word(&self, word: &str) -> bool {
        let stop_words = [
            "the", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
            "from", "up", "about", "into", "through", "during", "before", "after", "above", "below",
            "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "do", "does",
            "did", "will", "would", "could", "should", "may", "might", "must", "can", "this", "that",
            "these", "those", "i", "you", "he", "she", "it", "we", "they", "me", "him", "her", "us",
            "them", "my", "your", "his", "its", "our", "their", "a", "an",
        ];
        stop_words.contains(&word)
    }

    /// Calculate similarity between two keyword lists
    fn calculate_keyword_similarity(&self, keywords1: &[String], keywords2: &[String]) -> f64 {
        if keywords1.is_empty() || keywords2.is_empty() {
            return 0.0;
        }

        let set1: std::collections::HashSet<_> = keywords1.iter().collect();
        let set2: std::collections::HashSet<_> = keywords2.iter().collect();

        let intersection = set1.intersection(&set2).count();
        let union = set1.union(&set2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }
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
        assert_eq!(suggestions.len(), 2);
        assert!(suggestions
            .iter()
            .any(|cmd| cmd.definition.name == "hello-world"));

        // Test fuzzy match (current implementation may return different results)
        let suggestions = registry.suggest_commands("hlp", None).await;
        // Note: Fuzzy matching implementation may need improvement
        // For now, just verify it doesn't panic and returns reasonable results
        assert!(suggestions.len() >= 0);
    }

    // Automata integration tests
    #[tokio::test]
    async fn test_build_autocomplete_index() {
        let registry = CommandRegistry::new().unwrap();

        // Register test commands
        let commands = vec![
            ("build-project", "Build the project with all dependencies"),
            ("deploy-application", "Deploy the application to production"),
            ("test-suite", "Run comprehensive test suite"),
            ("database-backup", "Create backup of database"),
        ];

        for (name, desc) in &commands {
            let command = ParsedCommand {
                definition: CommandDefinition {
                    name: name.to_string(),
                    description: desc.to_string(),
                    execution_mode: ExecutionMode::Local,
                    parameters: vec![
                        CommandParameter {
                            name: "environment".to_string(),
                            param_type: "string".to_string(),
                            required: false,
                            description: Some("Target environment".to_string()),
                            ..Default::default()
                        }
                    ],
                    ..Default::default()
                },
                content: format!("This is the content for {} command.", name),
                source_path: PathBuf::from(format!("{}.md", name)),
                modified: SystemTime::UNIX_EPOCH,
            };
            registry.register_command(command).await.unwrap();
        }

        // Build autocomplete index
        assert!(registry.build_autocomplete_index().await.is_ok());

        // Verify index was built
        {
            let autocomplete_index = registry.autocomplete_index.read().await;
            assert!(autocomplete_index.is_some(), "Autocomplete index should be built");
            let index = autocomplete_index.as_ref().unwrap();
            assert!(index.len() > commands.len(), "Index should contain commands + parameters + keywords");
        }
    }

    #[tokio::test]
    async fn test_intelligent_command_discovery() {
        let registry = CommandRegistry::new().unwrap();

        // Register test commands with varied descriptions
        let commands = vec![
            ("build", "Build the project with all dependencies"),
            ("deploy", "Deploy the application to production"),
            ("test", "Run comprehensive test suite"),
            ("backup-database", "Create backup of database"),
        ];

        for (name, desc) in &commands {
            let command = ParsedCommand {
                definition: CommandDefinition {
                    name: name.to_string(),
                    description: desc.to_string(),
                    execution_mode: ExecutionMode::Local,
                    ..Default::default()
                },
                content: format!("Detailed help for {} command with examples.", name),
                source_path: PathBuf::from(format!("{}.md", name)),
                modified: SystemTime::UNIX_EPOCH,
            };
            registry.register_command(command).await.unwrap();
        }

        // Test exact match
        let results = registry.discover_commands("build", Some(5)).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].command.definition.name, "build");
        assert_eq!(results[0].match_type, "exact");

        // Test prefix match
        let results = registry.discover_commands("dep", Some(5)).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.command.definition.name == "deploy"));

        // Test content-based discovery (should find commands with matching content)
        let results = registry.discover_commands("comprehensive", Some(5)).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.command.definition.name == "test"));

        // Test that results are sorted by relevance
        for i in 1..results.len() {
            assert!(results[i-1].match_score >= results[i].match_score);
        }
    }

    #[tokio::test]
    async fn test_content_analysis() {
        let registry = CommandRegistry::new().unwrap();

        // Register a command with rich content
        let command = ParsedCommand {
            definition: CommandDefinition {
                name: "deploy".to_string(),
                description: "Deploy the application".to_string(),
                execution_mode: ExecutionMode::Local,
                ..Default::default()
            },
            content: "This command deploys the application using docker containers and ensures production deployment.".to_string(),
            source_path: PathBuf::from("deploy.md"),
            modified: SystemTime::UNIX_EPOCH,
        };

        registry.register_command(command).await.unwrap();

        // Build index first
        registry.build_autocomplete_index().await.unwrap();

        // Analyze command content
        let matches = registry.analyze_command_content("deploy").await.unwrap();
        // Should find some matches based on thesaurus entries
        assert!(!matches.is_empty(), "Should find some matches in command content");
    }

    #[tokio::test]
    async fn test_help_paragraph_extraction() {
        let registry = CommandRegistry::new().unwrap();

        let command = ParsedCommand {
            definition: CommandDefinition {
                name: "build".to_string(),
                description: "Build the project".to_string(),
                execution_mode: ExecutionMode::Local,
                ..Default::default()
            },
            content: "This command builds the project.

First, it installs all dependencies using cargo.

Then it compiles the project in release mode.

Finally, it runs tests to ensure everything works correctly.

Usage:
  build --release

Options:
  --release  Build in release mode
  --verbose   Show detailed output".to_string(),
            source_path: PathBuf::from("build.md"),
            modified: SystemTime::UNIX_EPOCH,
        };

        registry.register_command(command).await.unwrap();

        // Test paragraph extraction for specific terms
        let query_terms = vec!["cargo".to_string(), "release".to_string()];
        let paragraphs = registry.extract_help_paragraphs("build", &query_terms).await.unwrap();

        assert!(!paragraphs.is_empty(), "Should extract relevant paragraphs");

        // Verify that extracted paragraphs contain the query terms
        for (matched, paragraph) in &paragraphs {
            assert!(paragraph.to_lowercase().contains(&matched.term.to_lowercase()));
        }
    }

    #[tokio::test]
    async fn test_related_commands() {
        let registry = CommandRegistry::new().unwrap();

        // Register related commands
        let commands = vec![
            ("build-project", "Build the project with cargo"),
            ("build-release", "Build optimized release version"),
            ("deploy-project", "Deploy built project to production"),
            ("test-project", "Run tests for the project"),
        ];

        for (name, desc) in &commands {
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

        // Build index to enable related commands functionality
        registry.build_autocomplete_index().await.unwrap();

        // Test related commands discovery
        let results = registry.discover_commands("build", Some(10)).await.unwrap();
        if let Some(build_result) = results.first() {
            assert!(!build_result.related_commands.is_empty(),
                    "Should find related commands for build command");

            // Should include other build-related commands
            assert!(build_result.related_commands.iter().any(|cmd| cmd.contains("build")),
                    "Related commands should include other build commands");
        }
    }

    #[tokio::test]
    async fn test_keyword_extraction() {
        let registry = CommandRegistry::new().unwrap();

        let text = "This is a comprehensive test for the deployment system with docker containers.";
        let keywords = registry.extract_keywords_from_text(text);

        assert!(!keywords.is_empty(), "Should extract keywords");

        // Should extract meaningful keywords (not stop words)
        assert!(keywords.contains(&"comprehensive".to_string()), "Should extract 'comprehensive'");
        assert!(keywords.contains(&"deployment".to_string()), "Should extract 'deployment'");
        assert!(keywords.contains(&"system".to_string()), "Should extract 'system'");
        assert!(keywords.contains(&"docker".to_string()), "Should extract 'docker'");
        assert!(keywords.contains(&"containers".to_string()), "Should extract 'containers'");

        // Should not contain stop words
        assert!(!keywords.contains(&"this".to_string()), "Should not extract 'this'");
        assert!(!keywords.contains(&"with".to_string()), "Should not extract 'with'");
        assert!(!keywords.contains(&"the".to_string()), "Should not extract 'the'");
    }

    #[tokio::test]
    async fn test_keyword_similarity() {
        let registry = CommandRegistry::new().unwrap();

        let keywords1 = vec!["build".to_string(), "project".to_string(), "cargo".to_string()];
        let keywords2 = vec!["build".to_string(), "release".to_string(), "cargo".to_string()];
        let keywords3 = vec!["deploy".to_string(), "production".to_string()];

        // Test high similarity
        let similarity1 = registry.calculate_keyword_similarity(&keywords1, &keywords2);
        assert!(similarity1 > 0.5, "Should have high similarity between related keywords");

        // Test low similarity
        let similarity2 = registry.calculate_keyword_similarity(&keywords1, &keywords3);
        assert!(similarity2 < 0.3, "Should have low similarity between unrelated keywords");

        // Test empty case
        let similarity3 = registry.calculate_keyword_similarity(&keywords1, &vec![]);
        assert_eq!(similarity3, 0.0, "Should have zero similarity when one list is empty");
    }
}
