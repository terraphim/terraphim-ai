//! Claude Log Analyzer
//!
//! A high-performance Rust tool for analyzing Claude Code session logs
//! to identify which AI agents were used to build specific documents.
//!
//! ## Features
//!
//! - Parse JSONL session logs from `$HOME/.claude/projects/`
//! - Identify Task tool invocations with agent types
//! - Track file operations and attribute them to agents
//! - Generate rich terminal output with colored tables
//! - Export to JSON, CSV, and Markdown formats
//! - Timeline visualization and collaboration pattern detection
//! - Real-time session monitoring
//!
//! ## Example Usage
//!
//! ```rust
//! use terraphim_session_analyzer::{Analyzer, Reporter};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Analyze sessions from default location
//! let analyzer = Analyzer::from_default_location()?;
//! let analyses = analyzer.analyze(None)?;
//!
//! // Generate terminal report
//! let reporter = Reporter::new();
//! reporter.print_terminal(&analyses);
//!
//! // Export to JSON
//! let json = reporter.to_json(&analyses)?;
//! println!("{}", json);
//! # Ok(())
//! # }
//! ```

pub mod analyzer;
pub mod models;
pub mod parser;
pub mod patterns;
pub mod reporter;
pub mod tool_analyzer;

#[cfg(feature = "terraphim")]
pub mod kg;

pub mod connectors;

// Re-export main types for convenience
pub use analyzer::{Analyzer, SummaryStats};
pub use models::{
    AgentAttribution, AgentInvocation, AgentStatistics, AgentToolCorrelation, AnalyzerConfig,
    CollaborationPattern, FileOperation, SessionAnalysis, ToolAnalysis, ToolCategory, ToolChain,
    ToolInvocation, ToolStatistics, get_agent_category, normalize_agent_name,
};
pub use parser::{SessionParser, TimelineEvent, TimelineEventType};
pub use patterns::{
    AhoCorasickMatcher, PatternMatcher, ToolMatch, ToolMetadata, ToolPattern, create_matcher,
    load_patterns,
};
pub use reporter::Reporter;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default Claude session directory relative to home
pub const DEFAULT_SESSION_DIR: &str = ".claude/projects";

/// Common agent types used in Claude Code
pub const COMMON_AGENT_TYPES: &[&str] = &[
    "architect",
    "developer",
    "backend-architect",
    "frontend-developer",
    "rust-performance-expert",
    "rust-code-reviewer",
    "debugger",
    "technical-writer",
    "test-writer-fixer",
    "rapid-prototyper",
    "devops-automator",
    "overseer",
    "ai-engineer",
    "general-purpose",
];

/// Utility functions for common operations
pub mod utils {
    use crate::models::*;
    use std::path::Path;

    /// Get the default Claude session directory
    #[must_use]
    pub fn get_default_session_dir() -> Option<std::path::PathBuf> {
        home::home_dir().map(|home| home.join(crate::DEFAULT_SESSION_DIR))
    }

    /// Check if a path looks like a Claude session file
    pub fn is_session_file<P: AsRef<Path>>(path: P) -> bool {
        let path = path.as_ref();
        path.extension() == Some("jsonl".as_ref())
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.len() == 42) // UUID (36) + .jsonl (6) = 42
    }

    /// Extract project name from session path
    pub fn extract_project_name<P: AsRef<Path>>(session_path: P) -> Option<String> {
        let path = session_path.as_ref();
        path.parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            .map(|name| {
                // Convert encoded path back to readable format
                name.replace("-home-", "/home/").replace('-', "/")
            })
    }

    /// Filter analyses by project name
    #[must_use]
    pub fn filter_by_project<'a>(
        analyses: &'a [SessionAnalysis],
        project_filter: &str,
    ) -> Vec<&'a SessionAnalysis> {
        analyses
            .iter()
            .filter(|analysis| analysis.project_path.contains(project_filter))
            .collect()
    }

    /// Get unique agent types across all analyses
    #[must_use]
    pub fn get_unique_agents(analyses: &[SessionAnalysis]) -> Vec<String> {
        let mut agents: std::collections::HashSet<String> = std::collections::HashSet::new();

        for analysis in analyses {
            for agent in &analysis.agents {
                agents.insert(agent.agent_type.clone());
            }
        }

        let mut sorted: Vec<String> = agents.into_iter().collect();
        sorted.sort();
        sorted
    }

    /// Calculate total session time across analyses
    #[must_use]
    pub fn total_session_time(analyses: &[SessionAnalysis]) -> u64 {
        analyses.iter().map(|a| a.duration_ms).sum()
    }

    /// Find the most productive session (most files modified)
    #[must_use]
    pub fn most_productive_session(analyses: &[SessionAnalysis]) -> Option<&SessionAnalysis> {
        analyses.iter().max_by_key(|a| a.file_to_agents.len())
    }

    /// Find sessions that used a specific agent
    #[must_use]
    pub fn sessions_with_agent<'a>(
        analyses: &'a [SessionAnalysis],
        agent_type: &str,
    ) -> Vec<&'a SessionAnalysis> {
        analyses
            .iter()
            .filter(|analysis| {
                analysis
                    .agents
                    .iter()
                    .any(|agent| agent.agent_type == agent_type)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::*;

    #[test]
    fn test_is_session_file() {
        assert!(is_session_file(
            "b325985c-5c1c-48f1-97e2-e3185bb55886.jsonl"
        ));
        assert!(!is_session_file("regular-file.txt"));
        assert!(!is_session_file("short.jsonl"));
    }

    #[test]
    fn test_extract_project_name() {
        let path = "/home/alex/.claude/projects/-home-alex-projects-zestic-at-charm/session.jsonl";
        let project = extract_project_name(path);
        assert_eq!(
            project,
            Some("/home/alex/projects/zestic/at/charm".to_string())
        );
    }

    #[test]
    fn test_get_default_session_dir() {
        let dir = get_default_session_dir();
        assert!(dir.is_some());

        let path = dir.unwrap();
        assert!(path.to_string_lossy().contains(".claude"));
        assert!(path.to_string_lossy().contains("projects"));
    }
}
