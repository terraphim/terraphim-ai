//! Command suggestions based on similarity
//!
//! Uses string similarity algorithms to suggest commands when
//! the user types something unrecognized.

use strsim::{jaro_winkler, levenshtein};

/// A command suggestion with similarity score
#[derive(Debug, Clone, PartialEq)]
pub struct CommandSuggestion {
    /// The suggested command
    pub command: String,
    /// Edit distance from original input
    pub edit_distance: usize,
    /// Jaro-Winkler similarity (0.0 to 1.0)
    pub similarity: f64,
}

impl CommandSuggestion {
    /// Create a new suggestion
    pub fn new(command: impl Into<String>, input: &str) -> Self {
        let command = command.into();
        let edit_distance = levenshtein(input, &command);
        let similarity = jaro_winkler(input, &command);

        Self {
            command,
            edit_distance,
            similarity,
        }
    }

    /// Check if this is a high-confidence suggestion (likely what user meant)
    pub fn is_high_confidence(&self) -> bool {
        self.edit_distance <= 2 && self.similarity > 0.8
    }

    /// Check if this is worth showing as a suggestion
    pub fn is_reasonable(&self) -> bool {
        self.edit_distance <= 4 && self.similarity > 0.6
    }
}

/// Find similar commands from a list of known commands
pub fn find_similar_commands(
    input: &str,
    known_commands: &[&str],
    max_suggestions: usize,
) -> Vec<CommandSuggestion> {
    let input_lower = input.to_lowercase();

    let mut suggestions: Vec<CommandSuggestion> = known_commands
        .iter()
        .map(|cmd| CommandSuggestion::new(*cmd, &input_lower))
        .filter(|s| s.is_reasonable())
        .collect();

    // Sort by edit distance first, then by similarity (descending)
    suggestions.sort_by(|a, b| {
        a.edit_distance
            .cmp(&b.edit_distance)
            .then_with(|| b.similarity.partial_cmp(&a.similarity).unwrap())
    });

    suggestions.truncate(max_suggestions);
    suggestions
}

/// Find the best matching command if it's a high-confidence match
pub fn find_best_match(input: &str, known_commands: &[&str]) -> Option<CommandSuggestion> {
    let suggestions = find_similar_commands(input, known_commands, 1);

    suggestions.into_iter().find(|s| s.is_high_confidence())
}

/// Calculate edit distance between two strings
pub fn edit_distance(a: &str, b: &str) -> usize {
    levenshtein(a, b)
}

/// Calculate Jaro-Winkler similarity between two strings
pub fn similarity(a: &str, b: &str) -> f64 {
    jaro_winkler(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_suggestion() {
        let suggestion = CommandSuggestion::new("search", "serach");

        assert_eq!(suggestion.command, "search");
        assert_eq!(suggestion.edit_distance, 2);
        assert!(suggestion.similarity > 0.9);
        assert!(suggestion.is_high_confidence());
    }

    #[test]
    fn test_find_similar_commands() {
        let commands = vec!["search", "config", "role", "graph", "help", "quit"];

        let suggestions = find_similar_commands("serach", &commands, 3);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].command, "search");

        let suggestions = find_similar_commands("hlep", &commands, 3);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].command, "help");
    }

    #[test]
    fn test_find_best_match() {
        let commands = vec!["search", "config", "role", "graph", "help"];

        // Close match should be found
        let best = find_best_match("serach", &commands);
        assert!(best.is_some());
        assert_eq!(best.unwrap().command, "search");

        // Distant match should not be auto-corrected
        let best = find_best_match("xyz123", &commands);
        assert!(best.is_none());
    }

    #[test]
    fn test_edit_distance() {
        assert_eq!(edit_distance("search", "search"), 0);
        assert_eq!(edit_distance("search", "serach"), 2);
        assert_eq!(edit_distance("search", "find"), 6);
    }

    #[test]
    fn test_similarity() {
        let s1 = similarity("search", "search");
        assert!((s1 - 1.0).abs() < 0.001);

        let s2 = similarity("search", "serach");
        assert!(s2 > 0.9);

        let s3 = similarity("search", "xyz");
        assert!(s3 < 0.5);
    }

    #[test]
    fn test_case_insensitive_matching() {
        let commands = vec!["search", "config"];

        let suggestions = find_similar_commands("SEARCH", &commands, 3);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].command, "search");
    }
}
