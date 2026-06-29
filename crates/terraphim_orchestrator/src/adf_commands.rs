//! ADF Command Parser using terraphim-automata
//!
//! Provides fast Aho-Corasick based parsing for @adf: commands in comments.
//! Uses terraphim-automata's matcher for multi-pattern term matching.
//! Supports special commands like @adf:compound-review, @adf:security-sentinel, etc.

use terraphim_automata::matcher::find_matches;
use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

/// ADF Command types that can be triggered via mentions
#[derive(Debug, Clone, PartialEq)]
pub enum AdfCommand {
    /// Trigger a compound review
    CompoundReview { issue_number: u64, comment_id: u64 },
    /// Spawn a specific agent
    SpawnAgent {
        agent_name: String,
        issue_number: u64,
        comment_id: u64,
        context: String,
    },
    /// Trigger a persona-based agent
    SpawnPersona {
        persona_name: String,
        issue_number: u64,
        comment_id: u64,
        context: String,
    },
    /// Unknown command
    Unknown { raw: String },
}

/// Parser for ADF commands using terraphim-automata
pub struct AdfCommandParser {
    /// Thesaurus containing all ADF command patterns
    thesaurus: Thesaurus,
    /// Map from normalized term to command type
    command_map: std::collections::HashMap<NormalizedTermValue, CommandType>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CommandType {
    CompoundReview,
    AgentSpawn,
    PersonaSpawn,
}

impl AdfCommandParser {
    /// Create a new ADF command parser with all known commands
    pub fn new(agent_names: &[String], persona_names: &[String]) -> Self {
        let mut thesaurus = Thesaurus::new("adf_commands".to_string());
        let mut command_map = std::collections::HashMap::new();
        let mut term_id = 1u64;

        // Special compound review command
        let compound_term = NormalizedTermValue::from("@adf:compound-review");
        thesaurus.insert(
            compound_term.clone(),
            NormalizedTerm::new(term_id, compound_term.clone()),
        );
        command_map.insert(compound_term, CommandType::CompoundReview);
        term_id += 1;

        // Alternative compound review trigger
        let alt_compound_term = NormalizedTermValue::from("@compound-review");
        thesaurus.insert(
            alt_compound_term.clone(),
            NormalizedTerm::new(term_id, alt_compound_term.clone()),
        );
        command_map.insert(alt_compound_term, CommandType::CompoundReview);
        term_id += 1;

        // Agent spawn commands: @adf:agent-name
        for agent in agent_names {
            let pattern = format!("@adf:{}", agent);
            let term = NormalizedTermValue::from(pattern.as_str());
            thesaurus.insert(term.clone(), NormalizedTerm::new(term_id, term.clone()));
            command_map.insert(term, CommandType::AgentSpawn);
            term_id += 1;
        }

        // Persona spawn commands: @adf:persona-name
        for persona in persona_names {
            let pattern = format!("@adf:{}", persona);
            let term = NormalizedTermValue::from(pattern.as_str());
            thesaurus.insert(term.clone(), NormalizedTerm::new(term_id, term.clone()));
            command_map.insert(term, CommandType::PersonaSpawn);
            term_id += 1;
        }

        Self {
            thesaurus,
            command_map,
        }
    }

    /// Parse commands from a comment body using terraphim-automata matcher
    ///
    /// Returns a list of commands found in the text, in order of appearance.
    pub fn parse_commands(
        &self,
        text: &str,
        issue_number: u64,
        comment_id: u64,
    ) -> Vec<AdfCommand> {
        let mut commands = vec![];

        // Use terraphim-automata's find_matches for Aho-Corasick matching
        let matches = match find_matches(text, self.thesaurus.clone(), true) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(error = %e, "ADF command matching failed");
                return commands;
            }
        };

        for matched in matches {
            let term_value = NormalizedTermValue::from(matched.term.as_str());
            let cmd_type = self.command_map.get(&term_value);

            // Extract matched position and context after the command
            let context = match matched.pos {
                Some((start, end)) => {
                    let _matched_text = &text[start..end]; // Exact matched text
                    extract_context(text, end)
                }
                None => String::new(),
            };

            let command = match cmd_type {
                Some(CommandType::CompoundReview) => AdfCommand::CompoundReview {
                    issue_number,
                    comment_id,
                },
                Some(CommandType::AgentSpawn) => {
                    let agent_name = matched.term.trim_start_matches("@adf:").to_string();
                    AdfCommand::SpawnAgent {
                        agent_name,
                        issue_number,
                        comment_id,
                        context,
                    }
                }
                Some(CommandType::PersonaSpawn) => {
                    let persona_name = matched.term.trim_start_matches("@adf:").to_string();
                    AdfCommand::SpawnPersona {
                        persona_name,
                        issue_number,
                        comment_id,
                        context,
                    }
                }
                None => AdfCommand::Unknown {
                    raw: matched.term.clone(),
                },
            };

            commands.push(command);
        }

        commands
    }

    /// Check if text contains any ADF commands
    pub fn has_commands(&self, text: &str) -> bool {
        match find_matches(text, self.thesaurus.clone(), false) {
            Ok(matches) => !matches.is_empty(),
            Err(_) => false,
        }
    }
}

/// Extract context after a command (rest of line or paragraph)
fn extract_context(text: &str, end_pos: usize) -> String {
    let after = &text[end_pos..];

    // Take until end of line or paragraph
    let context = after.lines().next().unwrap_or("").trim().to_string();

    // Limit context length
    if context.len() > 500 {
        format!("{}...", &context[..497])
    } else {
        context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_compound_review() {
        let parser = AdfCommandParser::new(&[], &[]);

        let commands = parser.parse_commands("Please run @adf:compound-review on this PR", 42, 123);

        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], AdfCommand::CompoundReview { .. }));
    }

    #[test]
    fn test_parse_agent_spawn() {
        let parser = AdfCommandParser::new(&["security-sentinel".to_string()], &[]);

        let commands = parser.parse_commands("@adf:security-sentinel check this code", 42, 123);

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            AdfCommand::SpawnAgent { agent_name, .. } => {
                assert_eq!(agent_name, "security-sentinel");
            }
            _ => panic!("Expected SpawnAgent command"),
        }
    }

    #[test]
    fn test_parse_multiple_commands() {
        let parser =
            AdfCommandParser::new(&["security-sentinel".to_string()], &["vigil".to_string()]);

        let commands = parser.parse_commands(
            "@adf:vigil please review, then @adf:compound-review",
            42,
            123,
        );

        assert_eq!(commands.len(), 2);
    }

    #[test]
    fn test_alternative_compound_trigger() {
        let parser = AdfCommandParser::new(&[], &[]);

        let commands = parser.parse_commands("Run @compound-review now", 42, 123);

        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], AdfCommand::CompoundReview { .. }));
    }

    #[test]
    fn test_case_insensitive() {
        let parser = AdfCommandParser::new(&["Security-Sentinel".to_string()], &[]);

        let commands = parser.parse_commands("@ADF:SECURITY-SENTINEL check this", 42, 123);

        assert_eq!(commands.len(), 1);
    }

    #[test]
    fn test_has_commands() {
        let parser = AdfCommandParser::new(&["agent".to_string()], &[]);

        assert!(parser.has_commands("@adf:agent do something"));
        assert!(!parser.has_commands("No commands here"));
    }
}
