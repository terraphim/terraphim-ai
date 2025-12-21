//! Pattern matcher implementation using Aho-Corasick algorithm
//!
//! This module provides efficient multi-pattern string matching to identify
//! tool usage in Bash commands.

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use anyhow::Result;
use std::collections::HashMap;

// Terraphim imports for knowledge graph automata
#[cfg(feature = "terraphim")]
use terraphim_automata::find_matches as terraphim_find_matches;
#[cfg(feature = "terraphim")]
use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

use super::loader::ToolPattern;

/// Trait for pattern matching implementations
pub trait PatternMatcher: Send + Sync {
    /// Initialize the matcher with tool patterns
    ///
    /// # Errors
    ///
    /// Returns an error if the automaton cannot be built from the patterns
    fn initialize(&mut self, patterns: &[ToolPattern]) -> Result<()>;

    /// Find all tool matches in the given text
    ///
    /// Returns matches ordered by position (leftmost-longest)
    fn find_matches<'a>(&self, text: &'a str) -> Vec<ToolMatch<'a>>;

    /// Get the matcher type identifier
    #[allow(dead_code)] // May be used for debugging
    fn matcher_type(&self) -> &'static str;
}

/// Represents a tool match found in text
#[derive(Debug, Clone, PartialEq)]
pub struct ToolMatch<'a> {
    /// The name of the matched tool
    pub tool_name: String,

    /// Start position in the text
    pub start: usize,

    /// End position in the text
    pub end: usize,

    /// The matched text
    pub text: &'a str,

    /// Category of the tool
    pub category: String,

    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
}

/// Aho-Corasick based pattern matcher
///
/// Uses efficient automaton-based matching for high performance
/// even with many patterns.
pub struct AhoCorasickMatcher {
    /// The Aho-Corasick automaton
    automaton: Option<AhoCorasick>,

    /// Mapping from pattern index to tool metadata
    pattern_to_tool: HashMap<usize, ToolInfo>,
}

#[derive(Debug, Clone)]
struct ToolInfo {
    name: String,
    category: String,
    confidence: f32,
}

impl Default for AhoCorasickMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl AhoCorasickMatcher {
    /// Create a new uninitialized matcher
    #[must_use]
    pub fn new() -> Self {
        Self {
            automaton: None,
            pattern_to_tool: HashMap::new(),
        }
    }

    /// Build the Aho-Corasick automaton from patterns
    fn build_automaton(&mut self, patterns: &[ToolPattern]) -> Result<()> {
        let mut all_patterns = Vec::new();
        self.pattern_to_tool.clear();

        for tool in patterns.iter() {
            for pattern in &tool.patterns {
                let pattern_idx = all_patterns.len();
                all_patterns.push(pattern.clone());

                self.pattern_to_tool.insert(
                    pattern_idx,
                    ToolInfo {
                        name: tool.name.clone(),
                        category: tool.metadata.category.clone(),
                        confidence: tool.metadata.confidence,
                    },
                );
            }
        }

        let automaton = AhoCorasickBuilder::new()
            .ascii_case_insensitive(true)
            .match_kind(MatchKind::LeftmostLongest)
            .build(&all_patterns)
            .map_err(|e| anyhow::anyhow!("Failed to build Aho-Corasick automaton: {e}"))?;

        self.automaton = Some(automaton);
        Ok(())
    }
}

impl PatternMatcher for AhoCorasickMatcher {
    fn initialize(&mut self, patterns: &[ToolPattern]) -> Result<()> {
        self.build_automaton(patterns)
    }

    fn find_matches<'a>(&self, text: &'a str) -> Vec<ToolMatch<'a>> {
        let Some(ref automaton) = self.automaton else {
            return Vec::new();
        };

        let mut matches = Vec::new();

        for mat in automaton.find_iter(text) {
            if let Some(tool_info) = self.pattern_to_tool.get(&mat.pattern().as_usize()) {
                matches.push(ToolMatch {
                    tool_name: tool_info.name.clone(),
                    start: mat.start(),
                    end: mat.end(),
                    text: &text[mat.start()..mat.end()],
                    category: tool_info.category.clone(),
                    confidence: tool_info.confidence,
                });
            }
        }

        matches
    }

    fn matcher_type(&self) -> &'static str {
        "aho-corasick"
    }
}

/// Terraphim-based pattern matcher using knowledge graph automata
///
/// This implementation uses the actual terraphim_automata library for pattern matching,
/// which provides knowledge graph-based semantic search capabilities.
#[cfg(feature = "terraphim")]
pub struct TerraphimMatcher {
    /// Thesaurus containing the pattern mappings
    thesaurus: Option<Thesaurus>,

    /// Mapping from tool name to metadata
    tool_metadata: HashMap<String, (String, f32)>, // (category, confidence)

    /// Fallback Aho-Corasick matcher for error cases
    fallback: AhoCorasickMatcher,
}

#[cfg(feature = "terraphim")]
impl Default for TerraphimMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "terraphim")]
impl TerraphimMatcher {
    /// Create a new uninitialized Terraphim matcher
    #[must_use]
    pub fn new() -> Self {
        Self {
            thesaurus: None,
            tool_metadata: HashMap::new(),
            fallback: AhoCorasickMatcher::new(),
        }
    }

    /// Build a Thesaurus from tool patterns
    fn build_thesaurus(&mut self, patterns: &[ToolPattern]) -> Result<()> {
        let mut thesaurus = Thesaurus::new("Tool Patterns".to_string());
        let mut pattern_id = 0u64;

        // Clear and rebuild metadata map
        self.tool_metadata.clear();

        for tool in patterns {
            // Store tool metadata
            self.tool_metadata.insert(
                tool.name.clone(),
                (tool.metadata.category.clone(), tool.metadata.confidence),
            );

            for pattern in &tool.patterns {
                pattern_id += 1;

                // Create a normalized term for this pattern
                let normalized_term = NormalizedTerm {
                    id: pattern_id,
                    value: NormalizedTermValue::from(tool.name.as_str()),
                    url: tool.metadata.description.as_ref().map(|d| d.to_string()),
                };

                // Insert the pattern -> normalized term mapping
                thesaurus.insert(NormalizedTermValue::from(pattern.as_str()), normalized_term);
            }
        }

        self.thesaurus = Some(thesaurus);
        Ok(())
    }
}

#[cfg(feature = "terraphim")]
impl PatternMatcher for TerraphimMatcher {
    fn initialize(&mut self, patterns: &[ToolPattern]) -> Result<()> {
        // Build the terraphim thesaurus
        self.build_thesaurus(patterns)?;

        // Also initialize fallback in case terraphim fails
        self.fallback.initialize(patterns)?;

        Ok(())
    }

    fn find_matches<'a>(&self, text: &'a str) -> Vec<ToolMatch<'a>> {
        // Use the actual terraphim_automata library
        let Some(ref thesaurus) = self.thesaurus else {
            // If thesaurus not initialized, use fallback
            return self.fallback.find_matches(text);
        };

        // Call the actual terraphim_automata find_matches function
        match terraphim_find_matches(text, thesaurus.clone(), true) {
            Ok(matches) => {
                // Convert terraphim matches to our ToolMatch format
                matches
                    .into_iter()
                    .filter_map(|m| {
                        let tool_name = m.normalized_term.value.to_string();

                        // Look up category and confidence from metadata
                        let (category, confidence) = self
                            .tool_metadata
                            .get(&tool_name)
                            .map(|(cat, conf)| (cat.clone(), *conf))
                            .unwrap_or_else(|| ("unknown".to_string(), 0.5));

                        // Extract position from the pos field
                        m.pos.map(|(start, end)| ToolMatch {
                            tool_name,
                            start,
                            end,
                            text: &text[start..end],
                            category,
                            confidence,
                        })
                    })
                    .collect()
            }
            Err(_) => {
                // If terraphim fails, fall back to aho-corasick
                self.fallback.find_matches(text)
            }
        }
    }

    fn matcher_type(&self) -> &'static str {
        if self.thesaurus.is_some() {
            "terraphim-automata"
        } else {
            "terraphim-automata (uninitialized)"
        }
    }
}

/// Factory function to create a new pattern matcher
///
/// Returns Terraphim matcher if the feature is enabled,
/// otherwise returns the default Aho-Corasick implementation
#[must_use]
#[allow(dead_code)] // Used in doc examples
pub fn create_matcher() -> Box<dyn PatternMatcher> {
    #[cfg(feature = "terraphim")]
    {
        Box::new(TerraphimMatcher::new())
    }

    #[cfg(not(feature = "terraphim"))]
    {
        Box::new(AhoCorasickMatcher::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patterns::loader::ToolMetadata;

    fn create_test_patterns() -> Vec<ToolPattern> {
        vec![
            ToolPattern {
                name: "wrangler".to_string(),
                patterns: vec!["npx wrangler".to_string(), "bunx wrangler".to_string()],
                metadata: ToolMetadata {
                    category: "cloudflare".to_string(),
                    description: Some("Cloudflare Workers CLI".to_string()),
                    confidence: 0.95,
                },
            },
            ToolPattern {
                name: "npm".to_string(),
                patterns: vec!["npm ".to_string()],
                metadata: ToolMetadata {
                    category: "package-manager".to_string(),
                    description: Some("Node package manager".to_string()),
                    confidence: 0.9,
                },
            },
        ]
    }

    #[test]
    fn test_matcher_initialization() {
        let patterns = create_test_patterns();
        let mut matcher = AhoCorasickMatcher::new();

        let result = matcher.initialize(&patterns);
        assert!(result.is_ok());
        assert!(matcher.automaton.is_some());
    }

    #[test]
    fn test_find_matches_basic() {
        let patterns = create_test_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "npx wrangler deploy --env production";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].tool_name, "wrangler");
        assert_eq!(matches[0].text, "npx wrangler");
        assert_eq!(matches[0].category, "cloudflare");
    }

    #[test]
    fn test_find_matches_case_insensitive() {
        let patterns = create_test_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "NPX WRANGLER deploy";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].tool_name, "wrangler");
    }

    #[test]
    fn test_find_matches_multiple_tools() {
        let patterns = create_test_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "npm install && npx wrangler deploy";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].tool_name, "npm");
        assert_eq!(matches[1].tool_name, "wrangler");
    }

    #[test]
    fn test_find_matches_alternative_pattern() {
        let patterns = create_test_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "bunx wrangler dev";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].tool_name, "wrangler");
        assert_eq!(matches[0].text, "bunx wrangler");
    }

    #[test]
    fn test_find_matches_no_matches() {
        let patterns = create_test_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "echo hello world";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_matcher_type() {
        let matcher = AhoCorasickMatcher::new();
        assert_eq!(matcher.matcher_type(), "aho-corasick");
    }

    #[test]
    fn test_create_matcher_factory() {
        let matcher = create_matcher();

        // Factory returns different matchers based on features
        #[cfg(feature = "terraphim")]
        assert_eq!(matcher.matcher_type(), "terraphim-automata (uninitialized)");

        #[cfg(not(feature = "terraphim"))]
        assert_eq!(matcher.matcher_type(), "aho-corasick");
    }

    #[test]
    fn test_uninitialized_matcher() {
        let matcher = AhoCorasickMatcher::new();
        let matches = matcher.find_matches("npx wrangler deploy");
        assert_eq!(matches.len(), 0);
    }
}

#[cfg(test)]
mod wrangler_tests {
    use super::*;
    use crate::patterns::loader::ToolMetadata;

    /// Create comprehensive wrangler patterns with all package manager variants
    fn create_wrangler_patterns() -> Vec<ToolPattern> {
        vec![ToolPattern {
            name: "wrangler".to_string(),
            patterns: vec![
                "npx wrangler".to_string(),
                "bunx wrangler".to_string(),
                "pnpm wrangler".to_string(),
                "yarn wrangler".to_string(),
            ],
            metadata: ToolMetadata {
                category: "cloudflare".to_string(),
                description: Some("Cloudflare Workers CLI".to_string()),
                confidence: 0.95,
            },
        }]
    }

    #[test]
    fn test_wrangler_login_npx() {
        let patterns = create_wrangler_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "npx wrangler login";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].tool_name, "wrangler");
        assert_eq!(matches[0].text, "npx wrangler");
        assert_eq!(matches[0].category, "cloudflare");
        assert_eq!(matches[0].confidence, 0.95);
    }

    #[test]
    fn test_wrangler_login_bunx() {
        let patterns = create_wrangler_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "bunx wrangler login";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].tool_name, "wrangler");
        assert_eq!(matches[0].text, "bunx wrangler");
        assert_eq!(matches[0].category, "cloudflare");
    }

    #[test]
    fn test_wrangler_deploy_basic() {
        let patterns = create_wrangler_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "npx wrangler deploy";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].tool_name, "wrangler");
        assert_eq!(matches[0].text, "npx wrangler");
    }

    #[test]
    fn test_wrangler_deploy_with_env() {
        let patterns = create_wrangler_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        // Test npx variant
        let text = "npx wrangler deploy --env production";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].tool_name, "wrangler");
        assert_eq!(matches[0].text, "npx wrangler");

        // Test bunx variant
        let text = "bunx wrangler deploy --env staging";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].tool_name, "wrangler");
        assert_eq!(matches[0].text, "bunx wrangler");
    }

    #[test]
    fn test_wrangler_deploy_with_minify() {
        let patterns = create_wrangler_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "npx wrangler deploy --minify";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].tool_name, "wrangler");
        assert_eq!(matches[0].text, "npx wrangler");
    }

    #[test]
    fn test_wrangler_deploy_complex_flags() {
        let patterns = create_wrangler_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "npx wrangler deploy --env prod --minify --compatibility-date 2024-01-01";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].tool_name, "wrangler");
        assert_eq!(matches[0].text, "npx wrangler");
        assert_eq!(matches[0].start, 0);
        assert_eq!(matches[0].end, 12); // "npx wrangler" is 12 characters
    }

    #[test]
    fn test_wrangler_all_package_managers() {
        let patterns = create_wrangler_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        // Test all package manager variants
        let test_cases = vec![
            ("npx wrangler deploy", "npx wrangler"),
            ("bunx wrangler deploy", "bunx wrangler"),
            ("pnpm wrangler deploy", "pnpm wrangler"),
            ("yarn wrangler deploy", "yarn wrangler"),
        ];

        for (command, expected_text) in test_cases {
            let matches = matcher.find_matches(command);
            assert_eq!(matches.len(), 1, "Failed for command: {command}");
            assert_eq!(matches[0].tool_name, "wrangler");
            assert_eq!(matches[0].text, expected_text);
            assert_eq!(matches[0].category, "cloudflare");
        }
    }

    #[test]
    fn test_wrangler_publish() {
        let patterns = create_wrangler_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "npx wrangler publish";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].tool_name, "wrangler");
        assert_eq!(matches[0].text, "npx wrangler");
    }

    #[test]
    fn test_wrangler_dev() {
        let patterns = create_wrangler_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "bunx wrangler dev";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].tool_name, "wrangler");
        assert_eq!(matches[0].text, "bunx wrangler");
    }

    #[test]
    fn test_wrangler_tail() {
        let patterns = create_wrangler_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "npx wrangler tail";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].tool_name, "wrangler");
        assert_eq!(matches[0].text, "npx wrangler");
    }

    #[test]
    fn test_wrangler_case_insensitive() {
        let patterns = create_wrangler_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "NPX WRANGLER DEPLOY";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].tool_name, "wrangler");
    }

    #[test]
    fn test_wrangler_in_pipeline() {
        let patterns = create_wrangler_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "npm install && npx wrangler deploy && npm test";
        let matches = matcher.find_matches(text);

        // Should find wrangler
        let wrangler_matches: Vec<_> = matches
            .iter()
            .filter(|m| m.tool_name == "wrangler")
            .collect();
        assert_eq!(wrangler_matches.len(), 1);
        assert_eq!(wrangler_matches[0].text, "npx wrangler");
    }

    #[test]
    fn test_wrangler_multiple_commands() {
        let patterns = create_wrangler_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "npx wrangler login && bunx wrangler deploy";
        let matches = matcher.find_matches(text);

        // Should find both wrangler invocations
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].tool_name, "wrangler");
        assert_eq!(matches[0].text, "npx wrangler");
        assert_eq!(matches[1].tool_name, "wrangler");
        assert_eq!(matches[1].text, "bunx wrangler");
    }

    #[test]
    fn test_wrangler_with_output_redirection() {
        let patterns = create_wrangler_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "npx wrangler deploy > deploy.log 2>&1";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].tool_name, "wrangler");
        assert_eq!(matches[0].text, "npx wrangler");
    }

    #[test]
    fn test_wrangler_subcommands() {
        let patterns = create_wrangler_patterns();
        let mut matcher = AhoCorasickMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let subcommands = vec![
            "login",
            "deploy",
            "publish",
            "dev",
            "tail",
            "whoami",
            "init",
            "secret",
            "kv:namespace",
            "pages",
        ];

        for subcommand in subcommands {
            let text = format!("npx wrangler {subcommand}");
            let matches = matcher.find_matches(&text);

            assert_eq!(matches.len(), 1, "Failed for subcommand: {subcommand}");
            assert_eq!(matches[0].tool_name, "wrangler");
            assert_eq!(matches[0].text, "npx wrangler");
        }
    }
}

#[cfg(all(test, feature = "terraphim"))]
mod terraphim_tests {
    use super::*;
    use crate::patterns::loader::ToolMetadata;

    fn create_test_patterns() -> Vec<ToolPattern> {
        vec![
            ToolPattern {
                name: "wrangler".to_string(),
                patterns: vec!["npx wrangler".to_string(), "bunx wrangler".to_string()],
                metadata: ToolMetadata {
                    category: "cloudflare".to_string(),
                    description: Some("Cloudflare Workers CLI".to_string()),
                    confidence: 0.95,
                },
            },
            ToolPattern {
                name: "npm".to_string(),
                patterns: vec!["npm ".to_string()],
                metadata: ToolMetadata {
                    category: "package-manager".to_string(),
                    description: Some("Node package manager".to_string()),
                    confidence: 0.9,
                },
            },
        ]
    }

    #[test]
    fn test_terraphim_matcher_initialization() {
        let patterns = create_test_patterns();
        let mut matcher = TerraphimMatcher::new();

        let result = matcher.initialize(&patterns);
        assert!(result.is_ok());
    }

    #[test]
    fn test_terraphim_find_matches_basic() {
        let patterns = create_test_patterns();
        let mut matcher = TerraphimMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "npx wrangler deploy --env production";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].tool_name, "wrangler");
        assert_eq!(matches[0].category, "cloudflare");
    }

    #[test]
    fn test_terraphim_find_matches_case_insensitive() {
        let patterns = create_test_patterns();
        let mut matcher = TerraphimMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "NPX WRANGLER deploy";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].tool_name, "wrangler");
    }

    #[test]
    fn test_terraphim_find_matches_multiple_tools() {
        let patterns = create_test_patterns();
        let mut matcher = TerraphimMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "npm install && npx wrangler deploy";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].tool_name, "npm");
        assert_eq!(matches[1].tool_name, "wrangler");
    }

    #[test]
    fn test_terraphim_find_matches_alternative_pattern() {
        let patterns = create_test_patterns();
        let mut matcher = TerraphimMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "bunx wrangler dev";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].tool_name, "wrangler");
    }

    #[test]
    fn test_terraphim_find_matches_no_matches() {
        let patterns = create_test_patterns();
        let mut matcher = TerraphimMatcher::new();
        matcher.initialize(&patterns).unwrap();

        let text = "echo hello world";
        let matches = matcher.find_matches(text);

        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_terraphim_matcher_type() {
        let matcher = TerraphimMatcher::new();
        assert_eq!(matcher.matcher_type(), "terraphim-automata (uninitialized)");

        // After initialization, should be terraphim-automata
        let patterns = create_test_patterns();
        let mut matcher = TerraphimMatcher::new();
        matcher.initialize(&patterns).unwrap();
        assert_eq!(matcher.matcher_type(), "terraphim-automata");
    }

    #[test]
    fn test_terraphim_create_matcher_factory() {
        let matcher = create_matcher();
        // Uninitialized matcher
        assert_eq!(matcher.matcher_type(), "terraphim-automata (uninitialized)");
    }

    #[test]
    fn test_terraphim_uninitialized_matcher() {
        let matcher = TerraphimMatcher::new();
        let matches = matcher.find_matches("npx wrangler deploy");
        assert_eq!(matches.len(), 0);
    }
}
