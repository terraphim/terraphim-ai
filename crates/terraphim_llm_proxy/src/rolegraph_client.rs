//! RoleGraph client for knowledge graph-based routing
//!
//! Integrates Terraphim's RoleGraph and automata for intelligent pattern-based routing.
//! This is Phase 3 of the routing system.

use crate::{router::Priority, ProxyError, Result};
use aho_corasick::AhoCorasick;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

// ============================================================================
// Local Type Definitions
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMatch {
    pub concept: String,
    pub score: f64,
    pub priority: Priority,
    pub weighted_score: f64,
    pub provider: String,
    pub model: String,
    pub rule_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub id: String,
    pub concept: String,
    pub patterns: Vec<String>,
    pub provider: String,
    pub model: String,
    pub priority: Priority,
    pub metadata: Option<serde_json::Value>,
    pub enabled: bool,
    pub name: String,
}

impl RoutingRule {
    pub fn new(
        id: String,
        concept: String,
        patterns: Vec<String>,
        provider: String,
        model: String,
        priority: Priority,
    ) -> Self {
        Self {
            id: id.clone(),
            concept,
            patterns,
            provider,
            model,
            priority,
            metadata: None,
            enabled: true,
            name: id,
        }
    }
}

/// RoleGraph client for pattern-based routing
pub struct RoleGraphClient {
    /// Path to taxonomy directory
    taxonomy_path: PathBuf,

    /// Aho-Corasick automaton for pattern matching
    automaton: Option<AhoCorasick>,

    /// Pattern to concept mapping
    pattern_map: HashMap<usize, String>,

    /// Concept to (provider, model) routing directives parsed from markdown
    routing_map: HashMap<String, (String, String)>,

    /// Concept to priority mapping parsed from markdown
    priority_map: HashMap<String, Priority>,

    /// Routing rules with full metadata
    routing_rules: Vec<RoutingRule>,
}

impl RoleGraphClient {
    /// Create a new RoleGraph client
    pub fn new(taxonomy_path: impl AsRef<Path>) -> Result<Self> {
        let taxonomy_path = taxonomy_path.as_ref().to_path_buf();

        if !taxonomy_path.exists() {
            return Err(ProxyError::ConfigError(format!(
                "Taxonomy path does not exist: {}",
                taxonomy_path.display()
            )));
        }

        info!(
            path = %taxonomy_path.display(),
            "Initializing RoleGraph client"
        );

        Ok(Self {
            taxonomy_path,
            automaton: None,
            pattern_map: HashMap::new(),
            routing_map: HashMap::new(),
            priority_map: HashMap::new(),
            routing_rules: Vec::new(),
        })
    }

    /// Load taxonomy files and build automaton
    pub fn load_taxonomy(&mut self) -> Result<()> {
        info!("Loading taxonomy from {:?}", self.taxonomy_path);

        // Load INDEX.md to get taxonomy structure
        let index_path = self.taxonomy_path.join("INDEX.md");
        if !index_path.exists() {
            warn!("INDEX.md not found, will scan directory");
        }

        // Scan for taxonomy files
        let taxonomy_files = self.scan_taxonomy_files()?;
        info!("Found {} taxonomy files", taxonomy_files.len());

        // Parse each file and extract patterns
        let mut patterns = Vec::new();
        let mut pattern_concepts = Vec::new();
        self.routing_map.clear();
        self.priority_map.clear();
        self.routing_rules.clear();

        for file_path in taxonomy_files {
            if let Ok((concept, synonyms)) = self.parse_taxonomy_file(&file_path) {
                // Parse routing directives from the file as well
                if let Ok((provider, model)) = Self::parse_routing_directives(&file_path) {
                    self.routing_map
                        .insert(concept.clone(), (provider.clone(), model.clone()));

                    // Parse priority directive
                    let priority_value = Self::parse_priority_directive(&file_path).unwrap_or(50);
                    let priority = Priority::new(priority_value);
                    self.priority_map.insert(concept.clone(), priority);

                    // Create a routing rule
                    let rule = RoutingRule::new(
                        format!("rule-{}", concept),
                        concept.clone(),
                        vec![concept.clone()],
                        provider,
                        model,
                        priority,
                    );
                    self.routing_rules.push(rule);
                }

                // Add main concept as pattern
                patterns.push(concept.clone());
                pattern_concepts.push(concept.clone());

                // Add each synonym as pattern
                for synonym in synonyms {
                    patterns.push(synonym);
                    pattern_concepts.push(concept.clone());
                }
            }
        }

        // Build Aho-Corasick automaton
        if !patterns.is_empty() {
            let automaton = AhoCorasick::new(patterns.clone())
                .map_err(|e| ProxyError::Internal(format!("Failed to build automaton: {}", e)))?;

            // Build pattern ID to concept mapping
            for (idx, concept) in pattern_concepts.iter().enumerate() {
                self.pattern_map.insert(idx, concept.clone());
            }

            self.automaton = Some(automaton);
            info!("Built automaton with {} patterns", patterns.len());
        } else {
            warn!("No patterns found in taxonomy files");
        }

        Ok(())
    }

    /// Scan taxonomy directory for markdown files
    pub fn scan_taxonomy_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        // Scan subdirectories
        for subdir in &[
            "routing_scenarios",
            "providers",
            "transformers",
            "configuration",
            "operations",
            "technical",
        ] {
            let dir_path = self.taxonomy_path.join(subdir);
            if dir_path.exists() && dir_path.is_dir() {
                for entry in std::fs::read_dir(&dir_path)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "md") {
                        files.push(path);
                    }
                }
            }
        }

        Ok(files)
    }

    /// Parse taxonomy markdown file
    pub fn parse_taxonomy_file(&self, path: &Path) -> Result<(String, Vec<String>)> {
        let content = std::fs::read_to_string(path)?;

        // Extract concept name from first heading
        let concept = content
            .lines()
            .find(|line| line.starts_with("# "))
            .map(|line| line.trim_start_matches("# ").trim().to_string())
            .ok_or_else(|| {
                ProxyError::ConfigError(format!("No heading found in {}", path.display()))
            })?;

        // Extract synonyms from "synonyms::" line
        let synonyms = content
            .lines()
            .find(|line| line.starts_with("synonyms::"))
            .map(|line| {
                line.trim_start_matches("synonyms::")
                    .split(',')
                    .map(|s| s.trim().to_lowercase().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        debug!(
            concept = %concept,
            synonyms = synonyms.len(),
            "Parsed taxonomy file"
        );

        Ok((concept.to_lowercase().replace(" ", "_"), synonyms))
    }

    /// Parse routing directives from a taxonomy markdown file.
    ///
    /// Supported format (anywhere in the file, first match wins):
    ///   route:: provider_name, model_name
    /// Also accepts the alias:
    ///   routing:: provider_name, model_name
    fn parse_routing_directives(path: &Path) -> Result<(String, String)> {
        let content = std::fs::read_to_string(path)?;

        // Find the first line starting with route:: or routing::
        let line_opt = content.lines().find(|line| {
            line.trim_start().starts_with("route::") || line.trim_start().starts_with("routing::")
        });

        let line = match line_opt {
            Some(l) => l.trim_start(),
            None => {
                return Err(ProxyError::ConfigError(format!(
                    "No routing directive found in {}",
                    path.display()
                )));
            }
        };

        // Normalize prefix
        let rest = if let Some(rest) = line.strip_prefix("route::") {
            rest
        } else {
            line.trim_start_matches("routing::")
        };

        // Expect "provider, model"
        let parts: Vec<&str> = rest.split(',').collect();
        if parts.len() != 2 {
            return Err(ProxyError::ConfigError(format!(
                "Invalid routing directive format in {}: '{}'",
                path.display(),
                line
            )));
        }

        let provider = parts[0].trim().to_lowercase();
        let model = parts[1].trim().to_string();

        debug!(
            provider = %provider,
            model = %model,
            file = %path.display(),
            "Parsed routing directive"
        );

        Ok((provider, model))
    }

    /// Parse priority directive from a taxonomy markdown file.
    ///
    /// Supported format (anywhere in the file, first match wins):
    ///   priority:: 0-100
    /// Default priority is 50 (medium) if not specified.
    fn parse_priority_directive(path: &Path) -> Result<u8> {
        let content = std::fs::read_to_string(path)?;

        // Find the first line starting with priority::
        let line_opt = content
            .lines()
            .find(|line| line.trim_start().starts_with("priority::"));

        if let Some(line) = line_opt {
            let priority_str = line.trim_start().trim_start_matches("priority::").trim();

            // Parse priority value
            match priority_str.parse::<u8>() {
                Ok(value) => {
                    debug!(
                        priority = value,
                        file = %path.display(),
                        "Parsed priority directive"
                    );
                    Ok(value.clamp(0, 100))
                }
                Err(_) => {
                    warn!(
                        priority_str = %priority_str,
                        file = %path.display(),
                        "Invalid priority value, using default"
                    );
                    Ok(50) // Default medium priority
                }
            }
        } else {
            // No priority directive found, use default
            debug!(
                file = %path.display(),
                "No priority directive found, using default"
            );
            Ok(50) // Default medium priority
        }
    }

    /// Match patterns in query text
    pub fn match_patterns(&self, query: &str) -> Vec<PatternMatch> {
        let mut matches = Vec::new();

        if let Some(automaton) = &self.automaton {
            let query_lower = query.to_lowercase();

            for mat in automaton.find_iter(&query_lower) {
                let pattern_id = mat.pattern().as_usize();

                if let Some(concept) = self.pattern_map.get(&pattern_id) {
                    // Calculate score based on match length and position
                    let score = self.calculate_match_score(mat.start(), mat.end(), query.len());

                    // Look up routing for this concept
                    if let Some((provider, model)) = self.get_routing_for_concept(concept) {
                        let priority = self
                            .priority_map
                            .get(concept)
                            .copied()
                            .unwrap_or(Priority::Medium);
                        matches.push(PatternMatch {
                            concept: concept.clone(),
                            score,
                            priority,
                            weighted_score: score * priority.value() as f64,
                            provider,
                            model,
                            rule_id: format!("rule-{}", concept),
                        });
                    }
                }
            }
        }

        // Sort by score (descending)
        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        matches
    }

    /// Get all routing rules (for priority-based routing)
    pub fn get_routing_rules(&self) -> &[RoutingRule] {
        &self.routing_rules
    }

    /// Get enabled routing rules sorted by priority (descending)
    pub fn get_enabled_routing_rules(&self) -> Vec<&RoutingRule> {
        let mut rules: Vec<&RoutingRule> = self
            .routing_rules
            .iter()
            .filter(|rule| rule.enabled)
            .collect();

        // Sort by priority (descending), then by name for consistency
        rules.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.name.cmp(&b.name))
        });

        rules
    }

    /// Priority-aware pattern matching that returns terraphim_types PatternMatch
    pub fn query_routing_priority(&self, query: &str) -> Option<PatternMatch> {
        let matches = self.match_patterns(query);

        if let Some(best_match) = matches.first() {
            let priority = self
                .priority_map
                .get(&best_match.concept)
                .copied()
                .unwrap_or(Priority::Medium);

            Some(PatternMatch {
                concept: best_match.concept.clone(),
                score: best_match.score,
                priority,
                weighted_score: best_match.score * priority.value() as f64,
                provider: best_match.provider.clone(),
                model: best_match.model.clone(),
                rule_id: format!("rule-{}", best_match.concept),
            })
        } else {
            None
        }
    }

    /// Calculate match score based on position and length
    fn calculate_match_score(&self, start: usize, end: usize, query_len: usize) -> f64 {
        let length = end - start;

        // Longer matches are better
        let length_score = length as f64 / query_len as f64;

        // Earlier matches are slightly better
        let position_score = 1.0 - (start as f64 / query_len as f64) * 0.1;

        length_score * position_score
    }

    /// Get routing decision for a concept
    fn get_routing_for_concept(&self, concept: &str) -> Option<(String, String)> {
        if let Some((provider, model)) = self.routing_map.get(concept) {
            return Some((provider.clone(), model.clone()));
        }

        // Backward-compatible heuristics if no directive present
        let routing = match concept {
            s if s.contains("background") => {
                Some(("ollama".to_string(), "qwen2.5-coder:latest".to_string()))
            }
            s if s.contains("think") || s.contains("reason") => {
                Some(("deepseek".to_string(), "deepseek-reasoner".to_string()))
            }
            s if s.contains("search") || s.contains("web") => Some((
                "openrouter".to_string(),
                "perplexity/llama-3.1-sonar-large-128k-online".to_string(),
            )),
            s if s.contains("long_context") => Some((
                "openrouter".to_string(),
                "google/gemini-2.5-flash-preview-09-2025".to_string(),
            )),
            s if s.contains("image") => Some((
                "openrouter".to_string(),
                "anthropic/claude-sonnet-4.5".to_string(),
            )),
            s if s.contains("default") => {
                Some(("deepseek".to_string(), "deepseek-chat".to_string()))
            }
            _ => None,
        };

        if routing.is_some() {
            debug!(concept = %concept, "Found routing for concept (fallback)");
        }

        routing
    }

    /// Query RoleGraph for best routing decision
    pub fn query_routing(&self, query: &str) -> Option<PatternMatch> {
        let matches = self.match_patterns(query);

        // Return best match (highest score)
        matches.into_iter().next()
    }

    /// Get the number of patterns loaded
    pub fn pattern_count(&self) -> usize {
        self.pattern_map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_taxonomy() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let taxonomy_dir = temp_dir.path();

        // Create routing_scenarios directory
        let scenarios_dir = taxonomy_dir.join("routing_scenarios");
        fs::create_dir_all(&scenarios_dir).unwrap();

        // Create sample taxonomy file
        let think_file = scenarios_dir.join("think_routing.md");
        fs::write(
            &think_file,
            r#"# Think Routing

Routing for complex reasoning and planning tasks.

synonyms:: think, reason, plan, analyze deeply, complex reasoning
"#,
        )
        .unwrap();

        // Create another file
        let background_file = scenarios_dir.join("background_routing.md");
        fs::write(
            &background_file,
            r#"# Background Routing

Routing for background tasks.

synonyms:: background, index, scan, batch process
"#,
        )
        .unwrap();

        temp_dir
    }

    #[test]
    fn test_create_client() {
        let temp_dir = create_test_taxonomy();
        let client = RoleGraphClient::new(temp_dir.path());
        assert!(client.is_ok());
    }

    #[test]
    fn test_load_taxonomy() {
        let temp_dir = create_test_taxonomy();
        let mut client = RoleGraphClient::new(temp_dir.path()).unwrap();

        let result = client.load_taxonomy();
        assert!(result.is_ok());
        assert!(client.automaton.is_some());
    }

    #[test]
    fn test_pattern_matching() {
        let temp_dir = create_test_taxonomy();
        let mut client = RoleGraphClient::new(temp_dir.path()).unwrap();
        client.load_taxonomy().unwrap();

        let query = "I need to think about this problem carefully";
        let matches = client.match_patterns(query);

        assert!(!matches.is_empty());
        assert_eq!(matches[0].concept, "think_routing");
    }

    #[test]
    fn test_query_routing() {
        let temp_dir = create_test_taxonomy();
        let mut client = RoleGraphClient::new(temp_dir.path()).unwrap();
        client.load_taxonomy().unwrap();

        let query = "Run this as a background task";
        let routing = client.query_routing(query);

        assert!(routing.is_some());
        let routing = routing.unwrap();
        assert_eq!(routing.concept, "background_routing");
        assert_eq!(routing.provider, "ollama");
    }

    #[test]
    fn test_no_match_returns_none() {
        let temp_dir = create_test_taxonomy();
        let mut client = RoleGraphClient::new(temp_dir.path()).unwrap();
        client.load_taxonomy().unwrap();

        let query = "xyz unrelated query abc";
        let routing = client.query_routing(query);

        // No patterns match, should return None
        // (In practice, would fall back to Phase 1 or default)
        assert!(routing.is_none());
    }

    #[test]
    fn test_parse_routing_directives_from_markdown() {
        use std::fs;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("example.md");

        let content = r#"# Example Concept

Some description text.

synonyms:: example, sample

route:: deepseek, deepseek-reasoner
"#;

        fs::write(&file_path, content).unwrap();

        let (provider, model) = RoleGraphClient::parse_routing_directives(&file_path).unwrap();
        assert_eq!(provider, "deepseek");
        assert_eq!(model, "deepseek-reasoner");

        // Also test the alias 'routing::'
        let file_path2 = temp_dir.path().join("example2.md");
        let content2 = r#"# Another Concept

routing:: openrouter, anthropic/claude-3.5-sonnet
"#;
        fs::write(&file_path2, content2).unwrap();

        let (provider2, model2) = RoleGraphClient::parse_routing_directives(&file_path2).unwrap();
        assert_eq!(provider2, "openrouter");
        assert_eq!(model2, "anthropic/claude-3.5-sonnet");
    }
}
