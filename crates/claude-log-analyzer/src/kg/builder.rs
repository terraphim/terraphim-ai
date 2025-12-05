//! Knowledge Graph Builder using terraphim_automata
//!
//! This module constructs a knowledge graph from tool invocations,
//! extracting concepts and building a thesaurus for efficient pattern matching.

use crate::models::ToolInvocation;
use anyhow::Result;
use std::collections::HashMap;
use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

/// Knowledge graph builder that extracts concepts from tool invocations
#[derive(Debug, Clone)]
pub struct KnowledgeGraphBuilder {
    /// The terraphim thesaurus containing all concepts and patterns
    pub thesaurus: Thesaurus,

    /// Map from concept names to their associated patterns
    pub concept_map: HashMap<String, Vec<String>>,
}

impl KnowledgeGraphBuilder {
    /// Create a new empty knowledge graph builder
    #[must_use]
    pub fn new() -> Self {
        Self {
            thesaurus: Thesaurus::new("Tool Concepts".to_string()),
            concept_map: HashMap::new(),
        }
    }

    /// Build a knowledge graph from tool invocations
    ///
    /// Extracts concepts from tool names and command patterns, building
    /// a thesaurus that can be used for efficient concept matching.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use claude_log_analyzer::kg::KnowledgeGraphBuilder;
    /// use claude_log_analyzer::models::ToolInvocation;
    ///
    /// let tools = vec![/* tool invocations */];
    /// let builder = KnowledgeGraphBuilder::from_tool_invocations(&tools);
    /// ```
    #[must_use]
    pub fn from_tool_invocations(tools: &[ToolInvocation]) -> Self {
        let mut builder = Self::new();

        // Extract unique tool patterns
        let mut tool_patterns: HashMap<String, Vec<String>> = HashMap::new();

        for tool in tools {
            let tool_name = &tool.tool_name;
            let command = &tool.command_line;

            // Extract the base command (first few words)
            let base_pattern = extract_base_pattern(command);

            tool_patterns
                .entry(tool_name.clone())
                .or_default()
                .push(base_pattern);
        }

        // Build concepts from tool patterns
        builder.build_package_manager_concepts();
        builder.build_action_concepts();
        builder.build_tool_specific_concepts(&tool_patterns);

        builder
    }

    /// Add a concept with its associated patterns
    ///
    /// # Errors
    ///
    /// Returns an error if the concept cannot be added to the thesaurus
    pub fn add_concept(&mut self, concept: &str, patterns: Vec<String>) -> Result<()> {
        let concept_name = concept.to_uppercase();

        // Track in concept map
        self.concept_map
            .insert(concept_name.clone(), patterns.clone());

        // Add each pattern to the thesaurus
        for (idx, pattern) in patterns.iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let id = (self.thesaurus.len() + idx) as u64;
            let normalized_term = NormalizedTerm {
                id,
                value: NormalizedTermValue::from(concept_name.as_str()),
                url: Some(format!("concept://{concept_name}")),
            };

            self.thesaurus
                .insert(NormalizedTermValue::from(pattern.as_str()), normalized_term);
        }

        Ok(())
    }

    /// Build package manager concepts (BUN, NPM, YARN, etc.)
    fn build_package_manager_concepts(&mut self) {
        // BUN concept
        let _ = self.add_concept(
            "BUN",
            vec![
                "bunx".to_string(),
                "bun install".to_string(),
                "bun add".to_string(),
                "bun run".to_string(),
                "bun test".to_string(),
                "bun build".to_string(),
            ],
        );

        // NPM concept
        let _ = self.add_concept(
            "NPM",
            vec![
                "npm".to_string(),
                "npx".to_string(),
                "npm install".to_string(),
                "npm test".to_string(),
                "npm run".to_string(),
                "npm build".to_string(),
            ],
        );

        // YARN concept
        let _ = self.add_concept(
            "YARN",
            vec![
                "yarn".to_string(),
                "yarn add".to_string(),
                "yarn install".to_string(),
                "yarn test".to_string(),
                "yarn build".to_string(),
            ],
        );

        // PNPM concept
        let _ = self.add_concept(
            "PNPM",
            vec![
                "pnpm".to_string(),
                "pnpm add".to_string(),
                "pnpm install".to_string(),
                "pnpm test".to_string(),
                "pnpm build".to_string(),
            ],
        );

        // CARGO concept
        let _ = self.add_concept(
            "CARGO",
            vec![
                "cargo".to_string(),
                "cargo build".to_string(),
                "cargo test".to_string(),
                "cargo run".to_string(),
                "cargo clippy".to_string(),
                "cargo install".to_string(),
            ],
        );
    }

    /// Build action concepts (install, deploy, test, etc.)
    fn build_action_concepts(&mut self) {
        // INSTALL concept
        let _ = self.add_concept(
            "INSTALL",
            vec![
                "install".to_string(),
                "npm install".to_string(),
                "yarn install".to_string(),
                "pnpm install".to_string(),
                "bun install".to_string(),
                "cargo install".to_string(),
            ],
        );

        // DEPLOY concept
        let _ = self.add_concept(
            "DEPLOY",
            vec![
                "deploy".to_string(),
                "wrangler deploy".to_string(),
                "vercel deploy".to_string(),
                "netlify deploy".to_string(),
                "npx wrangler deploy".to_string(),
                "bunx wrangler deploy".to_string(),
            ],
        );

        // TEST concept
        let _ = self.add_concept(
            "TEST",
            vec![
                "test".to_string(),
                "npm test".to_string(),
                "yarn test".to_string(),
                "cargo test".to_string(),
                "pytest".to_string(),
                "jest".to_string(),
            ],
        );

        // BUILD concept
        let _ = self.add_concept(
            "BUILD",
            vec![
                "build".to_string(),
                "npm run build".to_string(),
                "yarn build".to_string(),
                "cargo build".to_string(),
                "webpack".to_string(),
                "vite build".to_string(),
            ],
        );
    }

    /// Build tool-specific concepts from observed patterns
    fn build_tool_specific_concepts(&mut self, tool_patterns: &HashMap<String, Vec<String>>) {
        for (tool_name, patterns) in tool_patterns {
            // Skip if already covered by other concepts
            if tool_name.eq_ignore_ascii_case("npm")
                || tool_name.eq_ignore_ascii_case("yarn")
                || tool_name.eq_ignore_ascii_case("bun")
                || tool_name.eq_ignore_ascii_case("cargo")
            {
                continue;
            }

            // Create concept for specific tools (wrangler, git, etc.)
            let unique_patterns: Vec<String> = patterns
                .iter()
                .take(10) // Limit patterns per tool
                .cloned()
                .collect();

            if !unique_patterns.is_empty() {
                let _ = self.add_concept(tool_name, unique_patterns);
            }
        }
    }
}

impl Default for KnowledgeGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract base pattern from a command line (first 2-3 words)
fn extract_base_pattern(command: &str) -> String {
    let words: Vec<&str> = command.split_whitespace().take(3).collect();

    // If it's a package manager invocation, include the next word
    if words.first().is_some_and(|w| {
        matches!(
            *w,
            "npm" | "npx" | "yarn" | "pnpm" | "bunx" | "bun" | "cargo"
        )
    }) && words.len() >= 2
    {
        words.join(" ")
    } else {
        words.first().map_or_else(String::new, |w| w.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ToolCategory;
    use jiff::Timestamp;

    fn create_test_tool(tool_name: &str, command: &str) -> ToolInvocation {
        ToolInvocation {
            timestamp: Timestamp::now(),
            tool_name: tool_name.to_string(),
            tool_category: ToolCategory::PackageManager,
            command_line: command.to_string(),
            arguments: vec![],
            flags: HashMap::new(),
            exit_code: Some(0),
            agent_context: None,
            session_id: "test-session".to_string(),
            message_id: "test-message".to_string(),
        }
    }

    #[test]
    fn test_new_builder() {
        let builder = KnowledgeGraphBuilder::new();
        assert_eq!(builder.thesaurus.name(), "Tool Concepts");
        assert!(builder.concept_map.is_empty());
    }

    #[test]
    fn test_add_concept() {
        let mut builder = KnowledgeGraphBuilder::new();

        let result = builder.add_concept(
            "TEST",
            vec!["npm test".to_string(), "yarn test".to_string()],
        );

        assert!(result.is_ok());
        assert!(builder.concept_map.contains_key("TEST"));
        assert_eq!(builder.concept_map["TEST"].len(), 2);
    }

    #[test]
    fn test_from_tool_invocations() {
        let tools = vec![
            create_test_tool("bun", "bunx wrangler deploy"),
            create_test_tool("npm", "npm install packages"),
            create_test_tool("cargo", "cargo build --release"),
        ];

        let builder = KnowledgeGraphBuilder::from_tool_invocations(&tools);

        // Should have built-in concepts
        assert!(builder.concept_map.contains_key("BUN"));
        assert!(builder.concept_map.contains_key("NPM"));
        assert!(builder.concept_map.contains_key("INSTALL"));
        assert!(builder.concept_map.contains_key("DEPLOY"));
    }

    #[test]
    fn test_extract_base_pattern_package_manager() {
        assert_eq!(
            extract_base_pattern("npm install packages"),
            "npm install packages"
        );
        assert_eq!(
            extract_base_pattern("bunx wrangler deploy"),
            "bunx wrangler deploy"
        );
        assert_eq!(
            extract_base_pattern("cargo build --release"),
            "cargo build --release"
        );
    }

    #[test]
    fn test_extract_base_pattern_simple_command() {
        assert_eq!(extract_base_pattern("git status"), "git");
        assert_eq!(extract_base_pattern("echo hello"), "echo");
    }

    #[test]
    fn test_package_manager_concepts() {
        let mut builder = KnowledgeGraphBuilder::new();
        builder.build_package_manager_concepts();

        // Verify BUN concept
        assert!(builder.concept_map.contains_key("BUN"));
        let bun_patterns = &builder.concept_map["BUN"];
        assert!(bun_patterns.contains(&"bunx".to_string()));
        assert!(bun_patterns.contains(&"bun install".to_string()));

        // Verify NPM concept
        assert!(builder.concept_map.contains_key("NPM"));
        let npm_patterns = &builder.concept_map["NPM"];
        assert!(npm_patterns.contains(&"npm".to_string()));
        assert!(npm_patterns.contains(&"npm install".to_string()));
    }

    #[test]
    fn test_action_concepts() {
        let mut builder = KnowledgeGraphBuilder::new();
        builder.build_action_concepts();

        // Verify INSTALL concept
        assert!(builder.concept_map.contains_key("INSTALL"));
        let install_patterns = &builder.concept_map["INSTALL"];
        assert!(install_patterns.contains(&"install".to_string()));
        assert!(install_patterns.contains(&"npm install".to_string()));

        // Verify DEPLOY concept
        assert!(builder.concept_map.contains_key("DEPLOY"));
        let deploy_patterns = &builder.concept_map["DEPLOY"];
        assert!(deploy_patterns.contains(&"deploy".to_string()));
        assert!(deploy_patterns.contains(&"wrangler deploy".to_string()));
    }

    #[test]
    fn test_thesaurus_not_empty() {
        let tools = vec![create_test_tool("npm", "npm install")];
        let builder = KnowledgeGraphBuilder::from_tool_invocations(&tools);

        // Thesaurus should contain patterns
        assert!(!builder.thesaurus.is_empty());
    }
}
