//! Output Validator
//!
//! Validates TUI command output for correctness and expected behavior.
//! Provides pattern matching, content validation, and error detection.

use anyhow::{Result, anyhow};
use regex::Regex;
use std::collections::HashMap;

/// Validation result for command output
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub exit_code: Option<i32>,
}

/// Output Validator for TUI testing
pub struct OutputValidator {
    /// Validation patterns for different commands
    command_patterns: HashMap<String, Vec<ValidationPattern>>,
    /// Error patterns to detect
    error_patterns: Vec<Regex>,
    /// Success patterns to detect
    success_patterns: Vec<Regex>,
}

#[derive(Debug, Clone)]
struct ValidationPattern {
    /// Pattern to match
    pattern: Regex,
    /// Whether this pattern is required
    required: bool,
    /// Description for error messages
    description: String,
}

impl OutputValidator {
    /// Create a new output validator
    pub fn new() -> Self {
        let mut validator = Self {
            command_patterns: HashMap::new(),
            error_patterns: Vec::new(),
            success_patterns: Vec::new(),
        };

        validator.initialize_patterns();
        validator
    }

    /// Initialize validation patterns for different commands
    fn initialize_patterns(&mut self) {
        // Error patterns (generic)
        self.error_patterns = vec![
            Regex::new(r"(?i)error:").unwrap(),
            Regex::new(r"(?i)failed").unwrap(),
            Regex::new(r"(?i)panic").unwrap(),
            Regex::new(r"(?i)unreachable").unwrap(),
        ];

        // Success patterns (generic)
        self.success_patterns = vec![
            Regex::new(r"✅").unwrap(),
            Regex::new(r"(?i)success").unwrap(),
            Regex::new(r"(?i)ok").unwrap(),
        ];

        // Search command patterns
        self.command_patterns.insert(
            "search".to_string(),
            vec![
                ValidationPattern {
                    pattern: Regex::new(r"🔍 Searching for:").unwrap(),
                    required: true,
                    description: "Search command should display search indicator".to_string(),
                },
                ValidationPattern {
                    pattern: Regex::new(r"Found \d+ result").unwrap(),
                    required: false,
                    description: "Search should show result count".to_string(),
                },
                ValidationPattern {
                    pattern: Regex::new(r"No results found").unwrap(),
                    required: false,
                    description: "Search should handle no results gracefully".to_string(),
                },
            ],
        );

        // Config command patterns
        self.command_patterns.insert(
            "config".to_string(),
            vec![ValidationPattern {
                pattern: Regex::new(r"\{.*\}").unwrap(),
                required: true,
                description: "Config show should return JSON".to_string(),
            }],
        );

        // Role command patterns
        self.command_patterns.insert(
            "role".to_string(),
            vec![
                ValidationPattern {
                    pattern: Regex::new(r"Available roles:").unwrap(),
                    required: true,
                    description: "Role list should show available roles".to_string(),
                },
                ValidationPattern {
                    pattern: Regex::new(r"▶|Switched to role").unwrap(),
                    required: false,
                    description: "Role select should show confirmation".to_string(),
                },
            ],
        );

        // Graph command patterns
        self.command_patterns.insert(
            "graph".to_string(),
            vec![
                ValidationPattern {
                    pattern: Regex::new(r"📊 Top \d+ concepts:").unwrap(),
                    required: true,
                    description: "Graph command should show top concepts".to_string(),
                },
                ValidationPattern {
                    pattern: Regex::new(r"\d+\. \w+").unwrap(),
                    required: false,
                    description: "Graph should list concepts with rankings".to_string(),
                },
            ],
        );

        // Replace command patterns
        self.command_patterns.insert(
            "replace".to_string(),
            vec![ValidationPattern {
                pattern: Regex::new(r"✨ Replaced text:").unwrap(),
                required: true,
                description: "Replace command should show replaced text".to_string(),
            }],
        );

        // Find command patterns
        self.command_patterns.insert(
            "find".to_string(),
            vec![
                ValidationPattern {
                    pattern: Regex::new(r"🔍 Found \d+ match").unwrap(),
                    required: false,
                    description: "Find command should show match count".to_string(),
                },
                ValidationPattern {
                    pattern: Regex::new(r"No matches found").unwrap(),
                    required: false,
                    description: "Find should handle no matches gracefully".to_string(),
                },
                ValidationPattern {
                    pattern: Regex::new(r"Term.*Position.*Normalized").unwrap(),
                    required: false,
                    description: "Find should display results in table format".to_string(),
                },
            ],
        );

        // Thesaurus command patterns
        self.command_patterns.insert(
            "thesaurus".to_string(),
            vec![
                ValidationPattern {
                    pattern: Regex::new(r"📚 Thesaurus.*contains \d+ terms").unwrap(),
                    required: true,
                    description: "Thesaurus should show term count".to_string(),
                },
                ValidationPattern {
                    pattern: Regex::new(r"ID.*Term.*Normalized.*URL").unwrap(),
                    required: false,
                    description: "Thesaurus should display results in table format".to_string(),
                },
            ],
        );

        // Help command patterns
        self.command_patterns.insert(
            "help".to_string(),
            vec![
                ValidationPattern {
                    pattern: Regex::new(r"Available commands:").unwrap(),
                    required: true,
                    description: "Help should show available commands".to_string(),
                },
                ValidationPattern {
                    pattern: Regex::new(r"/\w+").unwrap(),
                    required: true,
                    description: "Help should show command syntax".to_string(),
                },
            ],
        );

        // Clear command patterns
        self.command_patterns.insert(
            "clear".to_string(),
            vec![ValidationPattern {
                pattern: Regex::new(r"\x1B\[2J\x1B\[1;1H").unwrap(),
                required: false,
                description: "Clear command should send ANSI clear sequence".to_string(),
            }],
        );
    }

    /// Validate command output
    pub async fn validate_command_output(
        &self,
        command: &str,
        output: &str,
    ) -> Result<ValidationResult> {
        let mut result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            exit_code: None,
        };

        // Extract command base (remove leading slash and parameters)
        let command_base = self.extract_command_base(command);

        // Check for error patterns
        for error_pattern in &self.error_patterns {
            if error_pattern.is_match(output) {
                result.errors.push(format!(
                    "Error pattern detected: {}",
                    error_pattern.as_str()
                ));
                result.is_valid = false;
            }
        }

        // Check command-specific patterns
        if let Some(patterns) = self.command_patterns.get(&command_base) {
            for pattern in patterns {
                if pattern.required && !pattern.pattern.is_match(output) {
                    result
                        .errors
                        .push(format!("Required pattern missing: {}", pattern.description));
                    result.is_valid = false;
                }
            }
        }

        // Check for basic output (commands should produce some output)
        if output.trim().is_empty() && !command.contains("clear") {
            result.errors.push("Command produced no output".to_string());
            result.is_valid = false;
        }

        // Check for reasonable output length
        if output.len() > 1000000 {
            // 1MB limit
            result
                .warnings
                .push("Output is very large (>1MB)".to_string());
        }

        // Validate ANSI escape sequences are properly formed
        if let Err(e) = self.validate_ansi_sequences(output) {
            result
                .warnings
                .push(format!("ANSI sequence validation warning: {}", e));
        }

        // Validate table formatting for commands that should produce tables
        if self.should_have_table(&command_base) {
            if let Err(e) = self.validate_table_format(output) {
                result.warnings.push(format!("Table format warning: {}", e));
            }
        }

        Ok(result)
    }

    /// Extract command base from full command string
    fn extract_command_base(&self, command: &str) -> String {
        let cmd = command.trim().strip_prefix('/').unwrap_or(command);
        cmd.split_whitespace().next().unwrap_or("").to_lowercase()
    }

    /// Check if command should produce table-formatted output
    fn should_have_table(&self, command: &str) -> bool {
        matches!(command, "search" | "find" | "thesaurus" | "role")
    }

    /// Validate ANSI escape sequences
    fn validate_ansi_sequences(&self, output: &str) -> Result<()> {
        let ansi_pattern = Regex::new(r"\x1B\[[0-9;]*[A-Za-z]").unwrap();
        let invalid_pattern = Regex::new(r"\x1B\[[0-9;]*$").unwrap();

        // Check for incomplete ANSI sequences at end of output
        if invalid_pattern.is_match(output) {
            return Err(anyhow!("Incomplete ANSI escape sequence detected"));
        }

        // Basic validation - all ANSI sequences should be properly formed
        let sequences: Vec<_> = ansi_pattern.find_iter(output).collect();
        for seq_match in sequences {
            let seq = seq_match.as_str();
            // Check that sequence ends with a valid terminator
            if !seq
                .chars()
                .last()
                .map(|c| c.is_ascii_alphabetic())
                .unwrap_or(false)
            {
                return Err(anyhow!("Invalid ANSI escape sequence: {}", seq));
            }
        }

        Ok(())
    }

    /// Validate table formatting in output
    fn validate_table_format(&self, output: &str) -> Result<()> {
        // Look for common table indicators
        let has_borders = output.contains('┌') || output.contains('─') || output.contains('└');
        let has_separators = output.contains('|') || output.contains('+');

        if has_borders || has_separators {
            // If we detect table formatting, check for basic structure
            let lines: Vec<&str> = output.lines().collect();

            // Should have at least header + separator + data
            if lines.len() < 3 {
                return Err(anyhow!("Table appears too short"));
            }

            // Check for consistent column structure
            let mut column_counts = Vec::new();
            for line in &lines {
                if line.contains('|') {
                    let columns: Vec<&str> = line.split('|').collect();
                    column_counts.push(columns.len());
                }
            }

            if column_counts.len() > 1 {
                let first_count = column_counts[0];
                for &count in &column_counts {
                    if count != first_count {
                        return Err(anyhow!(
                            "Inconsistent column count in table: {} vs {}",
                            first_count,
                            count
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate command syntax
    pub fn validate_command_syntax(&self, command: &str) -> Result<()> {
        if command.trim().is_empty() {
            return Err(anyhow!("Empty command"));
        }

        // Check for valid command prefix
        if !command.starts_with('/')
            && !command.starts_with("search")
            && !command.starts_with("help")
        {
            // Allow some commands without slash
            return Ok(());
        }

        let cmd = command.strip_prefix('/').unwrap_or(command);

        // Basic syntax validation
        match cmd.split_whitespace().next() {
            Some("search") => self.validate_search_syntax(cmd)?,
            Some("config") => self.validate_config_syntax(cmd)?,
            Some("role") => self.validate_role_syntax(cmd)?,
            Some("graph") => self.validate_graph_syntax(cmd)?,
            Some("replace") => self.validate_replace_syntax(cmd)?,
            Some("find") => self.validate_find_syntax(cmd)?,
            Some("thesaurus") => self.validate_thesaurus_syntax(cmd)?,
            Some("help") => self.validate_help_syntax(cmd)?,
            Some("clear") | Some("quit") | Some("exit") | Some("q") => {} // No additional validation
            _ => {
                return Err(anyhow!(
                    "Unknown command: {}",
                    cmd.split_whitespace().next().unwrap()
                ));
            }
        }

        Ok(())
    }

    fn validate_search_syntax(&self, cmd: &str) -> Result<()> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(anyhow!("Search command requires a query"));
        }

        // Check for valid flags
        let mut i = 1;
        while i < parts.len() {
            match parts[i] {
                "--role" | "--limit" => {
                    if i + 1 >= parts.len() {
                        return Err(anyhow!("{} requires a value", parts[i]));
                    }
                    i += 2;
                }
                _ => {
                    // This should be part of the query
                    break;
                }
            }
        }

        Ok(())
    }

    fn validate_config_syntax(&self, cmd: &str) -> Result<()> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.len() > 2 {
            return Err(anyhow!("Config command syntax: config [show]"));
        }
        if parts.len() == 2 && parts[1] != "show" {
            return Err(anyhow!("Invalid config subcommand: {}", parts[1]));
        }
        Ok(())
    }

    fn validate_role_syntax(&self, cmd: &str) -> Result<()> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(anyhow!("Role command requires a subcommand"));
        }
        match parts[1] {
            "list" => {
                if parts.len() != 2 {
                    return Err(anyhow!("Role list takes no arguments"));
                }
            }
            "select" => {
                if parts.len() < 3 {
                    return Err(anyhow!("Role select requires a role name"));
                }
            }
            _ => return Err(anyhow!("Invalid role subcommand: {}", parts[1])),
        }
        Ok(())
    }

    fn validate_graph_syntax(&self, cmd: &str) -> Result<()> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let mut i = 1;
        while i < parts.len() {
            if parts[i] == "--top-k" {
                if i + 1 >= parts.len() {
                    return Err(anyhow!("--top-k requires a value"));
                }
                if parts[i + 1].parse::<usize>().is_err() {
                    return Err(anyhow!("--top-k value must be a number"));
                }
                i += 2;
            } else {
                return Err(anyhow!("Unknown graph option: {}", parts[i]));
            }
        }
        Ok(())
    }

    fn validate_replace_syntax(&self, cmd: &str) -> Result<()> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(anyhow!("Replace command requires text"));
        }

        let mut i = 1;
        while i < parts.len() {
            if parts[i] == "--format" {
                if i + 1 >= parts.len() {
                    return Err(anyhow!("--format requires a value"));
                }
                let format_val = parts[i + 1];
                if !matches!(format_val, "markdown" | "html" | "wiki" | "plain") {
                    return Err(anyhow!("Invalid format: {}", format_val));
                }
                i += 2;
            } else {
                break;
            }
        }

        Ok(())
    }

    fn validate_find_syntax(&self, cmd: &str) -> Result<()> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(anyhow!("Find command requires text"));
        }
        Ok(())
    }

    fn validate_thesaurus_syntax(&self, cmd: &str) -> Result<()> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let mut i = 1;
        while i < parts.len() {
            if parts[i] == "--role" {
                if i + 1 >= parts.len() {
                    return Err(anyhow!("--role requires a value"));
                }
                i += 2;
            } else {
                return Err(anyhow!("Unknown thesaurus option: {}", parts[i]));
            }
        }
        Ok(())
    }

    fn validate_help_syntax(&self, cmd: &str) -> Result<()> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.len() > 2 {
            return Err(anyhow!("Help syntax: help [command]"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_creation() {
        let validator = OutputValidator::new();
        assert!(!validator.command_patterns.is_empty());
    }

    #[tokio::test]
    async fn test_search_validation() {
        let validator = OutputValidator::new();

        // Valid search output
        let valid_output = "🔍 Searching for: 'test query'\nFound 5 result(s):";
        let result = validator
            .validate_command_output("/search test query", valid_output)
            .await
            .unwrap();
        assert!(result.is_valid);

        // Invalid search output (missing search indicator)
        let invalid_output = "Some random output";
        let result = validator
            .validate_command_output("/search test", invalid_output)
            .await
            .unwrap();
        assert!(!result.is_valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("Required pattern missing"))
        );
    }

    #[tokio::test]
    async fn test_config_validation() {
        let validator = OutputValidator::new();

        // Valid config output
        let valid_output = r#"{"selected_role": "Default", "some_setting": true}"#;
        let result = validator
            .validate_command_output("/config show", valid_output)
            .await
            .unwrap();
        assert!(result.is_valid);

        // Invalid config output (no JSON)
        let invalid_output = "Not JSON";
        let result = validator
            .validate_command_output("/config show", invalid_output)
            .await
            .unwrap();
        assert!(!result.is_valid);
    }

    #[test]
    fn test_syntax_validation() {
        let validator = OutputValidator::new();

        // Valid commands
        assert!(
            validator
                .validate_command_syntax("/search rust async")
                .is_ok()
        );
        assert!(validator.validate_command_syntax("/config show").is_ok());
        assert!(validator.validate_command_syntax("/role list").is_ok());
        assert!(validator.validate_command_syntax("/help").is_ok());

        // Invalid commands
        assert!(validator.validate_command_syntax("/search").is_err()); // Missing query
        assert!(
            validator
                .validate_command_syntax("/config invalid")
                .is_err()
        ); // Invalid subcommand
        assert!(validator.validate_command_syntax("/role").is_err()); // Missing subcommand
    }

    #[test]
    fn test_ansi_validation() {
        let validator = OutputValidator::new();

        // Valid ANSI sequences
        let valid_output = "Normal text\x1b[31mRed text\x1b[0mNormal again\x1b[2J";
        assert!(validator.validate_ansi_sequences(valid_output).is_ok());

        // Invalid ANSI sequence (incomplete)
        let invalid_output = "Text with incomplete escape\x1b[31";
        assert!(validator.validate_ansi_sequences(invalid_output).is_err());
    }

    #[test]
    fn test_command_base_extraction() {
        let validator = OutputValidator::new();

        assert_eq!(validator.extract_command_base("/search query"), "search");
        assert_eq!(validator.extract_command_base("search query"), "search");
        assert_eq!(validator.extract_command_base("/config show"), "config");
        assert_eq!(validator.extract_command_base("help"), "help");
    }
}
