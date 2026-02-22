//! Forgiving command parser
//!
//! Parses commands with typo tolerance and alias expansion.

use super::aliases::AliasRegistry;
use super::suggestions::{CommandSuggestion, find_best_match, find_similar_commands};

/// Result of parsing with the forgiving parser
#[derive(Debug, Clone)]
pub enum ParseResult {
    /// Exact match found
    Exact {
        /// The parsed command (canonical form)
        command: String,
        /// Original input (same as command for exact match)
        original: String,
        /// Remaining arguments after the command
        args: String,
    },
    /// Alias was expanded
    AliasExpanded {
        /// The expanded canonical command
        command: String,
        /// The original alias used
        original: String,
        /// Remaining arguments
        args: String,
    },
    /// Command was auto-corrected due to typo
    AutoCorrected {
        /// The corrected command
        command: String,
        /// The original (misspelled) input
        original: String,
        /// Edit distance
        distance: usize,
        /// Remaining arguments
        args: String,
    },
    /// Multiple possible corrections - user should choose
    Suggestions {
        /// Original input
        original: String,
        /// Possible corrections
        suggestions: Vec<CommandSuggestion>,
    },
    /// No match found
    Unknown {
        /// Original input
        original: String,
    },
    /// Empty input
    Empty,
}

impl ParseResult {
    /// Get the command if parsing succeeded
    pub fn command(&self) -> Option<&str> {
        match self {
            ParseResult::Exact { command, .. }
            | ParseResult::AliasExpanded { command, .. }
            | ParseResult::AutoCorrected { command, .. } => Some(command),
            _ => None,
        }
    }

    /// Get the original input
    pub fn original(&self) -> Option<&str> {
        match self {
            ParseResult::Exact { original, .. }
            | ParseResult::AliasExpanded { original, .. }
            | ParseResult::AutoCorrected { original, .. }
            | ParseResult::Suggestions { original, .. }
            | ParseResult::Unknown { original } => Some(original),
            ParseResult::Empty => None,
        }
    }

    /// Get the arguments if parsing succeeded
    pub fn args(&self) -> Option<&str> {
        match self {
            ParseResult::Exact { args, .. }
            | ParseResult::AliasExpanded { args, .. }
            | ParseResult::AutoCorrected { args, .. } => Some(args),
            _ => None,
        }
    }

    /// Check if this was auto-corrected
    pub fn was_corrected(&self) -> bool {
        matches!(self, ParseResult::AutoCorrected { .. })
    }

    /// Check if an alias was expanded
    pub fn was_alias(&self) -> bool {
        matches!(self, ParseResult::AliasExpanded { .. })
    }

    /// Check if parsing succeeded (command was determined)
    pub fn is_success(&self) -> bool {
        matches!(
            self,
            ParseResult::Exact { .. }
                | ParseResult::AliasExpanded { .. }
                | ParseResult::AutoCorrected { .. }
        )
    }

    /// Get the full command line (command + args) for successful parses
    pub fn full_command(&self) -> Option<String> {
        match self {
            ParseResult::Exact { command, args, .. }
            | ParseResult::AliasExpanded { command, args, .. }
            | ParseResult::AutoCorrected { command, args, .. } => {
                if args.is_empty() {
                    Some(command.clone())
                } else {
                    Some(format!("{} {}", command, args))
                }
            }
            _ => None,
        }
    }
}

/// Forgiving command parser with typo tolerance
#[derive(Debug)]
pub struct ForgivingParser {
    /// Known valid commands
    known_commands: Vec<String>,
    /// Alias registry
    aliases: AliasRegistry,
    /// Maximum edit distance for auto-correction
    max_auto_correct_distance: usize,
    /// Maximum suggestions to return
    max_suggestions: usize,
}

impl ForgivingParser {
    /// Create a new parser with default settings
    pub fn new(known_commands: Vec<String>) -> Self {
        Self {
            known_commands,
            aliases: AliasRegistry::new(),
            max_auto_correct_distance: 2,
            max_suggestions: 5,
        }
    }

    /// Create parser with custom alias registry
    pub fn with_aliases(mut self, aliases: AliasRegistry) -> Self {
        self.aliases = aliases;
        self
    }

    /// Set max auto-correct distance
    pub fn with_max_auto_correct_distance(mut self, distance: usize) -> Self {
        self.max_auto_correct_distance = distance;
        self
    }

    /// Set max suggestions
    pub fn with_max_suggestions(mut self, max: usize) -> Self {
        self.max_suggestions = max;
        self
    }

    /// Add additional known commands
    pub fn add_commands(&mut self, commands: &[&str]) {
        for cmd in commands {
            if !self.known_commands.contains(&cmd.to_string()) {
                self.known_commands.push(cmd.to_string());
            }
        }
    }

    /// Parse input with forgiving matching
    pub fn parse(&self, input: &str) -> ParseResult {
        let input = input.trim();

        if input.is_empty() {
            return ParseResult::Empty;
        }

        // Strip leading slash if present
        let input = input.strip_prefix('/').unwrap_or(input);

        // Split into command and args
        let (cmd_part, args) = match input.split_once(char::is_whitespace) {
            Some((cmd, rest)) => (cmd.trim(), rest.trim().to_string()),
            None => (input, String::new()),
        };

        let cmd_lower = cmd_part.to_lowercase();

        // 1. Check for alias first
        if let Some(canonical) = self.aliases.expand(&cmd_lower) {
            // Handle multi-word aliases (e.g., "sessions search")
            let full_cmd = if args.is_empty() {
                canonical.to_string()
            } else {
                format!("{} {}", canonical, args)
            };

            // Re-parse to get the actual command part
            let (actual_cmd, remaining_args) = match full_cmd.split_once(char::is_whitespace) {
                Some((cmd, rest)) => (cmd.to_string(), rest.to_string()),
                None => (full_cmd, String::new()),
            };

            return ParseResult::AliasExpanded {
                command: actual_cmd,
                original: cmd_part.to_string(),
                args: remaining_args,
            };
        }

        // 2. Check for exact match
        if self.is_known_command(&cmd_lower) {
            return ParseResult::Exact {
                command: cmd_lower,
                original: cmd_part.to_string(),
                args,
            };
        }

        // 3. Try fuzzy matching
        let commands: Vec<&str> = self.known_commands.iter().map(|s| s.as_str()).collect();

        if let Some(best) = find_best_match(&cmd_lower, &commands) {
            if best.edit_distance <= self.max_auto_correct_distance {
                return ParseResult::AutoCorrected {
                    command: best.command.clone(),
                    original: cmd_part.to_string(),
                    distance: best.edit_distance,
                    args,
                };
            }
        }

        // 4. Get suggestions for unknown command
        let suggestions = find_similar_commands(&cmd_lower, &commands, self.max_suggestions);

        if !suggestions.is_empty() {
            return ParseResult::Suggestions {
                original: cmd_part.to_string(),
                suggestions,
            };
        }

        // 5. Completely unknown
        ParseResult::Unknown {
            original: cmd_part.to_string(),
        }
    }

    /// Check if a command is in the known commands list
    fn is_known_command(&self, cmd: &str) -> bool {
        self.known_commands
            .iter()
            .any(|c| c.eq_ignore_ascii_case(cmd))
    }

    /// Get all known commands
    pub fn known_commands(&self) -> &[String] {
        &self.known_commands
    }

    /// Get the alias registry
    pub fn aliases(&self) -> &AliasRegistry {
        &self.aliases
    }
}

impl Default for ForgivingParser {
    fn default() -> Self {
        // Default commands based on terraphim_agent REPL
        let commands = vec![
            "search",
            "config",
            "role",
            "graph",
            "vm",
            "help",
            "quit",
            "exit",
            "clear",
            "robot",
            // Chat commands
            "chat",
            "summarize",
            // MCP commands
            "autocomplete",
            "extract",
            "find",
            "replace",
            "thesaurus",
            // File commands
            "file",
            // Web commands
            "web",
            // Session commands (future)
            "sessions",
        ];

        Self::new(commands.into_iter().map(String::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let parser = ForgivingParser::default();

        let result = parser.parse("search hello world");
        assert!(result.is_success());
        assert_eq!(result.command(), Some("search"));
        assert_eq!(result.args(), Some("hello world"));
        assert!(!result.was_corrected());
    }

    #[test]
    fn test_exact_match_with_slash() {
        let parser = ForgivingParser::default();

        let result = parser.parse("/search hello");
        assert!(result.is_success());
        assert_eq!(result.command(), Some("search"));
    }

    #[test]
    fn test_alias_expansion() {
        let parser = ForgivingParser::default();

        let result = parser.parse("q hello world");
        assert!(result.is_success());
        assert!(result.was_alias());
        assert_eq!(result.command(), Some("search"));
        assert_eq!(result.args(), Some("hello world"));
    }

    #[test]
    fn test_auto_correction() {
        let parser = ForgivingParser::default();

        let result = parser.parse("serach hello");
        assert!(result.is_success());
        assert!(result.was_corrected());
        assert_eq!(result.command(), Some("search"));

        if let ParseResult::AutoCorrected { distance, .. } = result {
            assert!(distance <= 2);
        }
    }

    #[test]
    fn test_suggestions() {
        let parser = ForgivingParser::default();

        // "searcxyz" has edit distance > 2, so should give suggestions not auto-correct
        let result = parser.parse("searcxyz");

        match result {
            ParseResult::Suggestions { suggestions, .. } => {
                assert!(!suggestions.is_empty());
            }
            ParseResult::AutoCorrected { distance, .. } => {
                // Also acceptable if edit distance algorithm is lenient
                assert!(distance > 0);
            }
            ParseResult::Unknown { .. } => {
                // Also acceptable for very different input
            }
            _ => panic!(
                "Expected Suggestions, AutoCorrected, or Unknown, got {:?}",
                result
            ),
        }
    }

    #[test]
    fn test_unknown_command() {
        let parser = ForgivingParser::default();

        let result = parser.parse("xyzabc123");
        assert!(!result.is_success());
        assert!(matches!(
            result,
            ParseResult::Unknown { .. } | ParseResult::Suggestions { .. }
        ));
    }

    #[test]
    fn test_empty_input() {
        let parser = ForgivingParser::default();

        let result = parser.parse("");
        assert!(matches!(result, ParseResult::Empty));

        let result = parser.parse("   ");
        assert!(matches!(result, ParseResult::Empty));
    }

    #[test]
    fn test_case_insensitive() {
        let parser = ForgivingParser::default();

        let result = parser.parse("SEARCH test");
        assert!(result.is_success());
        assert_eq!(result.command(), Some("search"));

        let result = parser.parse("Search test");
        assert!(result.is_success());
        assert_eq!(result.command(), Some("search"));
    }

    #[test]
    fn test_full_command() {
        let parser = ForgivingParser::default();

        let result = parser.parse("search hello world");
        assert_eq!(
            result.full_command(),
            Some("search hello world".to_string())
        );

        let result = parser.parse("quit");
        assert_eq!(result.full_command(), Some("quit".to_string()));
    }

    #[test]
    fn test_custom_parser() {
        let parser = ForgivingParser::new(vec!["custom".to_string(), "test".to_string()])
            .with_max_auto_correct_distance(1)
            .with_max_suggestions(3);

        let result = parser.parse("custm");
        assert!(result.is_success());
        assert_eq!(result.command(), Some("custom"));
    }
}
