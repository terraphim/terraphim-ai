//! Pattern learning infrastructure for dynamically discovering new tool patterns
//!
//! This module implements a learning system that observes tool usage in Bash commands,
//! identifies patterns, and promotes frequently-seen patterns to learned patterns.
//!
//! ## Architecture
//!
//! - `PatternLearner`: Main learning system with voting-based promotion
//! - `CandidatePattern`: Tracks observations and category votes for unknown tools
//! - `LearnedPattern`: Promoted patterns with confidence scores
//!
//! ## Example
//!
//! ```rust
//! use claude_log_analyzer::patterns::knowledge_graph::{PatternLearner, LearnedPattern};
//! use claude_log_analyzer::models::ToolCategory;
//!
//! # fn main() -> anyhow::Result<()> {
//! let mut learner = PatternLearner::new();
//!
//! // Observe tool usage
//! learner.observe(
//!     "pytest".to_string(),
//!     "pytest tests/".to_string(),
//!     ToolCategory::Testing
//! );
//!
//! // After multiple observations, promote to learned patterns
//! let learned = learner.promote_candidates();
//! # Ok(())
//! # }
//! ```

use crate::models::ToolCategory;
#[cfg(feature = "terraphim")]
use crate::models::ToolChain;
use anyhow::{Context, Result};
use indexmap::IndexMap;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Learn new tool patterns from usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternLearner {
    /// Candidate patterns being tracked
    candidate_patterns: IndexMap<String, CandidatePattern>,

    /// Number of observations required before promoting a pattern
    promotion_threshold: u32,
}

/// A candidate pattern being observed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidatePattern {
    /// Name of the tool
    pub tool_name: String,

    /// Number of times this tool has been observed
    pub observations: u32,

    /// Commands where this tool appears (for context analysis)
    pub contexts: Vec<String>,

    /// Votes for which category this tool belongs to
    pub category_votes: HashMap<String, u32>,

    /// First time this tool was observed
    pub first_seen: Timestamp,

    /// Last time this tool was observed
    pub last_seen: Timestamp,
}

/// A learned pattern that has been promoted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    /// Name of the tool
    pub tool_name: String,

    /// Determined category based on voting
    pub category: ToolCategory,

    /// Confidence score (0.0-1.0) based on observation consistency
    pub confidence: f32,

    /// Total number of observations
    pub observations: u32,

    /// When this pattern was learned (promoted)
    pub learned_at: Timestamp,
}

impl Default for PatternLearner {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)] // Will be used in Phase 3 Part 3
impl PatternLearner {
    /// Create a new pattern learner with default threshold (3 observations)
    #[must_use]
    pub fn new() -> Self {
        Self {
            candidate_patterns: IndexMap::new(),
            promotion_threshold: 3,
        }
    }

    /// Create a new pattern learner with custom promotion threshold
    #[must_use]
    pub fn with_threshold(threshold: u32) -> Self {
        Self {
            candidate_patterns: IndexMap::new(),
            promotion_threshold: threshold,
        }
    }

    /// Observe a potential new tool pattern
    ///
    /// This method records an observation of a tool being used in a specific context.
    /// When a tool reaches the promotion threshold, it can be promoted to a learned pattern.
    pub fn observe(&mut self, tool_name: String, command: String, category: ToolCategory) {
        let category_str = category_to_string(&category);
        let now = Timestamp::now();

        self.candidate_patterns
            .entry(tool_name.clone())
            .and_modify(|candidate| {
                candidate.observations += 1;
                candidate.last_seen = now;

                // Add context if not already present and within limit
                if !candidate.contexts.contains(&command) && candidate.contexts.len() < 10 {
                    candidate.contexts.push(command.clone());
                }

                // Vote on category
                *candidate
                    .category_votes
                    .entry(category_str.clone())
                    .or_insert(0) += 1;
            })
            .or_insert_with(|| CandidatePattern {
                tool_name: tool_name.clone(),
                observations: 1,
                contexts: vec![command],
                category_votes: {
                    let mut votes = HashMap::new();
                    votes.insert(category_str, 1);
                    votes
                },
                first_seen: now,
                last_seen: now,
            });
    }

    /// Promote candidates that meet the observation threshold to learned patterns
    ///
    /// Returns a list of newly promoted patterns and removes them from candidates.
    pub fn promote_candidates(&mut self) -> Vec<LearnedPattern> {
        let mut promoted = Vec::new();
        let now = Timestamp::now();

        // Find candidates ready for promotion
        let candidates_to_promote: Vec<String> = self
            .candidate_patterns
            .iter()
            .filter(|(_, candidate)| candidate.observations >= self.promotion_threshold)
            .map(|(name, _)| name.clone())
            .collect();

        // Promote each candidate
        for tool_name in candidates_to_promote {
            if let Some(candidate) = self.candidate_patterns.shift_remove(&tool_name) {
                let category = determine_category(&candidate.category_votes, &candidate.contexts);
                let confidence =
                    calculate_confidence(&candidate.category_votes, candidate.observations);

                promoted.push(LearnedPattern {
                    tool_name: candidate.tool_name,
                    category,
                    confidence,
                    observations: candidate.observations,
                    learned_at: now,
                });
            }
        }

        promoted
    }

    /// Get the current count of candidate patterns
    #[must_use]
    pub fn candidate_count(&self) -> usize {
        self.candidate_patterns.len()
    }

    /// Save learned patterns to cache directory
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be created or the file cannot be written
    pub fn save_to_cache(&self, learned_patterns: &[LearnedPattern]) -> Result<()> {
        let cache_path = get_cache_path()?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create cache directory: {}", parent.display())
            })?;
        }

        // Serialize and write patterns
        let json = serde_json::to_string_pretty(learned_patterns)
            .context("Failed to serialize learned patterns")?;

        std::fs::write(&cache_path, json).with_context(|| {
            format!(
                "Failed to write learned patterns to {}",
                cache_path.display()
            )
        })?;

        Ok(())
    }

    /// Load learned patterns from cache
    ///
    /// # Errors
    ///
    /// Returns an error if the cache file cannot be read or parsed
    pub fn load_from_cache() -> Result<Vec<LearnedPattern>> {
        let cache_path = get_cache_path()?;

        if !cache_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&cache_path)
            .with_context(|| format!("Failed to read cache file: {}", cache_path.display()))?;

        let patterns: Vec<LearnedPattern> = serde_json::from_str(&content)
            .context("Failed to parse learned patterns from cache")?;

        Ok(patterns)
    }

    /// Get all current candidate patterns (for debugging/inspection)
    #[must_use]
    pub fn get_candidates(&self) -> Vec<&CandidatePattern> {
        self.candidate_patterns.values().collect()
    }
}

/// Determine the category based on voting results and context analysis
#[allow(dead_code)] // Will be used in Phase 3 Part 3
fn determine_category(category_votes: &HashMap<String, u32>, contexts: &[String]) -> ToolCategory {
    // Find the category with the most votes
    let winner = category_votes
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(category, _)| category.as_str());

    if let Some(category_str) = winner {
        string_to_category(category_str)
    } else {
        // Fallback: infer from contexts
        infer_category_from_contexts(contexts)
    }
}

/// Calculate confidence score based on voting consistency
#[allow(dead_code)] // Used in tests
fn calculate_confidence(category_votes: &HashMap<String, u32>, total_observations: u32) -> f32 {
    if total_observations == 0 {
        return 0.0;
    }

    // Find the highest vote count
    let max_votes = category_votes.values().max().copied().unwrap_or(0);

    // Confidence is the proportion of votes for the winning category
    #[allow(clippy::cast_precision_loss)]
    let confidence = (max_votes as f32) / (total_observations as f32);

    // Clamp to valid range
    confidence.clamp(0.0, 1.0)
}

/// Infer category from tool name and command contexts using heuristics
#[allow(dead_code)] // Will be used in Phase 3 Part 3
pub fn infer_category_from_contexts(contexts: &[String]) -> ToolCategory {
    // Analyze the contexts to find common patterns
    let combined_context = contexts.join(" ").to_lowercase();

    // Testing tools
    if combined_context.contains("test")
        || combined_context.contains("spec")
        || combined_context.contains("jest")
        || combined_context.contains("pytest")
        || combined_context.contains("mocha")
    {
        return ToolCategory::Testing;
    }

    // Build tools
    if combined_context.contains("build")
        || combined_context.contains("webpack")
        || combined_context.contains("vite")
        || combined_context.contains("rollup")
        || combined_context.contains("esbuild")
    {
        return ToolCategory::BuildTool;
    }

    // Linting
    if combined_context.contains("lint")
        || combined_context.contains("eslint")
        || combined_context.contains("clippy")
        || combined_context.contains("pylint")
    {
        return ToolCategory::Linting;
    }

    // Git operations
    if combined_context.contains("git ")
        || combined_context.contains("commit")
        || combined_context.contains("push")
        || combined_context.contains("pull")
    {
        return ToolCategory::Git;
    }

    // Package managers
    if combined_context.contains("install")
        || combined_context.contains("npm ")
        || combined_context.contains("yarn ")
        || combined_context.contains("pnpm ")
        || combined_context.contains("cargo ")
        || combined_context.contains("pip ")
    {
        return ToolCategory::PackageManager;
    }

    // Cloud deployment
    if combined_context.contains("deploy")
        || combined_context.contains("publish")
        || combined_context.contains("wrangler")
        || combined_context.contains("vercel")
        || combined_context.contains("netlify")
    {
        return ToolCategory::CloudDeploy;
    }

    // Database
    if combined_context.contains("database")
        || combined_context.contains("migrate")
        || combined_context.contains("psql")
        || combined_context.contains("mysql")
    {
        return ToolCategory::Database;
    }

    // Default to Other
    ToolCategory::Other("unknown".to_string())
}

/// Convert ToolCategory to string for storage
#[allow(dead_code)] // Will be used in Phase 3 Part 3
fn category_to_string(category: &ToolCategory) -> String {
    match category {
        ToolCategory::PackageManager => "PackageManager".to_string(),
        ToolCategory::BuildTool => "BuildTool".to_string(),
        ToolCategory::Testing => "Testing".to_string(),
        ToolCategory::Linting => "Linting".to_string(),
        ToolCategory::Git => "Git".to_string(),
        ToolCategory::CloudDeploy => "CloudDeploy".to_string(),
        ToolCategory::Database => "Database".to_string(),
        ToolCategory::Other(s) => format!("Other({s})"),
    }
}

/// Convert string back to ToolCategory
#[allow(dead_code)] // Will be used in Phase 3 Part 3
fn string_to_category(s: &str) -> ToolCategory {
    match s {
        "PackageManager" => ToolCategory::PackageManager,
        "BuildTool" => ToolCategory::BuildTool,
        "Testing" => ToolCategory::Testing,
        "Linting" => ToolCategory::Linting,
        "Git" => ToolCategory::Git,
        "CloudDeploy" => ToolCategory::CloudDeploy,
        "Database" => ToolCategory::Database,
        s if s.starts_with("Other(") => {
            let inner = s.trim_start_matches("Other(").trim_end_matches(')');
            ToolCategory::Other(inner.to_string())
        }
        _ => ToolCategory::Other(s.to_string()),
    }
}

/// Get the path to the learned patterns cache file
///
/// # Errors
///
/// Returns an error if the home directory cannot be determined
#[allow(dead_code)] // Used in tests
fn get_cache_path() -> Result<PathBuf> {
    let home = home::home_dir().context("Could not find home directory")?;
    Ok(home
        .join(".config")
        .join("claude-log-analyzer")
        .join("learned_patterns.json"))
}

// ============================================================================
// Tool Relationship Models (Feature-gated for Terraphim)
// ============================================================================

/// Relationship between two tools indicating how they interact in workflows
#[cfg(feature = "terraphim")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(dead_code)] // Will be used in future terraphim integration
pub struct ToolRelationship {
    /// The source tool in the relationship
    pub from_tool: String,

    /// The target tool in the relationship
    pub to_tool: String,

    /// The type of relationship between the tools
    pub relationship_type: RelationType,

    /// Confidence score for this relationship (0.0-1.0)
    pub confidence: f32,
}

/// Types of relationships between tools
#[cfg(feature = "terraphim")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)] // Will be used in future terraphim integration
pub enum RelationType {
    /// Tool A requires Tool B to function (e.g., wrangler depends on npm build)
    DependsOn,

    /// Tool A is an alternative to Tool B (e.g., bunx replaces npx)
    Replaces,

    /// Tool A works well with Tool B (e.g., git works with npm)
    Complements,

    /// Tool A conflicts with Tool B
    Conflicts,
}

#[cfg(feature = "terraphim")]
#[allow(dead_code)] // Methods will be used in future terraphim integration
impl ToolRelationship {
    /// Infer relationships from tool chain patterns
    ///
    /// Analyzes a tool chain to identify potential relationships between tools.
    /// Sequential tools often have DependsOn relationships, while tools that appear
    /// in similar contexts might Complement each other.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use claude_log_analyzer::patterns::knowledge_graph::ToolRelationship;
    /// use claude_log_analyzer::models::ToolChain;
    ///
    /// let chain = ToolChain {
    ///     tools: vec!["npm".to_string(), "wrangler".to_string()],
    ///     frequency: 5,
    ///     average_time_between_ms: 1000,
    ///     typical_agent: Some("devops".to_string()),
    ///     success_rate: 0.95,
    /// };
    ///
    /// let relationships = ToolRelationship::infer_from_chain(&chain);
    /// // Expect npm -> wrangler DependsOn relationship
    /// ```
    #[must_use]
    pub fn infer_from_chain(chain: &ToolChain) -> Vec<Self> {
        let mut relationships = Vec::new();

        // Sequential tools suggest DependsOn relationships
        // Higher frequency and success rate increase confidence
        for i in 0..chain.tools.len().saturating_sub(1) {
            let from_tool = &chain.tools[i];
            let to_tool = &chain.tools[i + 1];

            // Base confidence on chain success rate and frequency
            #[allow(clippy::cast_precision_loss)]
            let frequency_factor = (chain.frequency.min(10) as f32) / 10.0;
            let base_confidence = chain.success_rate * frequency_factor;

            // Common dependency patterns get higher confidence
            let confidence = if is_known_dependency(from_tool, to_tool) {
                (base_confidence + 0.2).min(1.0)
            } else {
                base_confidence
            };

            relationships.push(ToolRelationship {
                from_tool: to_tool.clone(),
                to_tool: from_tool.clone(),
                relationship_type: RelationType::DependsOn,
                confidence,
            });
        }

        relationships
    }

    /// Create a new tool relationship
    #[must_use]
    pub fn new(
        from_tool: String,
        to_tool: String,
        relationship_type: RelationType,
        confidence: f32,
    ) -> Self {
        Self {
            from_tool,
            to_tool,
            relationship_type,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }
}

/// Check if a tool dependency is well-known
#[cfg(feature = "terraphim")]
#[allow(dead_code)] // Used in inference and tests
fn is_known_dependency(dependency: &str, dependent: &str) -> bool {
    // Common dependency patterns
    matches!(
        (dependency, dependent),
        ("npm", "wrangler")
            | ("npm", "vercel")
            | ("npm", "netlify")
            | ("cargo", "clippy")
            | ("git", "npm")
            | ("git", "cargo")
            | ("npm", "npx")
            | ("yarn", "npx")
    )
}

/// Knowledge graph containing tool relationships
#[cfg(feature = "terraphim")]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(dead_code)] // Will be used in future terraphim integration
pub struct KnowledgeGraph {
    /// All known tool relationships
    pub relationships: Vec<ToolRelationship>,
}

#[cfg(feature = "terraphim")]
#[allow(dead_code)] // Methods will be used in future terraphim integration
impl KnowledgeGraph {
    /// Create a new empty knowledge graph
    #[must_use]
    pub fn new() -> Self {
        Self {
            relationships: Vec::new(),
        }
    }

    /// Build a knowledge graph from tool chains
    ///
    /// Analyzes all tool chains to infer relationships between tools.
    /// Common sequences suggest DependsOn relationships, while alternative
    /// patterns suggest Replaces relationships.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use claude_log_analyzer::patterns::knowledge_graph::KnowledgeGraph;
    /// use claude_log_analyzer::models::ToolChain;
    ///
    /// let chains = vec![
    ///     ToolChain {
    ///         tools: vec!["git".to_string(), "npm".to_string()],
    ///         frequency: 10,
    ///         average_time_between_ms: 500,
    ///         typical_agent: Some("developer".to_string()),
    ///         success_rate: 0.95,
    ///     },
    /// ];
    ///
    /// let graph = KnowledgeGraph::build_from_chains(&chains);
    /// ```
    #[must_use]
    pub fn build_from_chains(chains: &[ToolChain]) -> Self {
        let mut graph = Self::new();

        // Infer DependsOn relationships from sequential tool usage
        for chain in chains {
            let relationships = ToolRelationship::infer_from_chain(chain);
            for rel in relationships {
                graph.add_relationship(rel);
            }
        }

        // Infer Replaces relationships from alternative tool patterns
        graph.infer_replacement_relationships(chains);

        // Infer Complements relationships from co-occurrence
        graph.infer_complement_relationships(chains);

        graph
    }

    /// Add a relationship to the graph with deduplication
    ///
    /// If a relationship between the same tools already exists, the one with
    /// higher confidence is kept, or they are merged if they have the same type.
    pub fn add_relationship(&mut self, new_rel: ToolRelationship) {
        // Check for existing relationship between same tools
        if let Some(existing) = self.relationships.iter_mut().find(|r| {
            r.from_tool == new_rel.from_tool
                && r.to_tool == new_rel.to_tool
                && r.relationship_type == new_rel.relationship_type
        }) {
            // Merge by taking higher confidence and averaging
            existing.confidence = (existing.confidence + new_rel.confidence) / 2.0;
        } else {
            self.relationships.push(new_rel);
        }
    }

    /// Infer replacement relationships from alternative tool usage patterns
    fn infer_replacement_relationships(&mut self, chains: &[ToolChain]) {
        // Build tool position map - tools that appear in the same position
        let mut position_map: HashMap<usize, HashMap<String, u32>> = HashMap::new();

        for chain in chains {
            for (pos, tool) in chain.tools.iter().enumerate() {
                *position_map
                    .entry(pos)
                    .or_default()
                    .entry(tool.clone())
                    .or_insert(0) += chain.frequency;
            }
        }

        // Find tools that appear in the same position (potential replacements)
        for tools_at_position in position_map.values() {
            let tools: Vec<(&String, &u32)> = tools_at_position.iter().collect();

            for i in 0..tools.len() {
                for j in (i + 1)..tools.len() {
                    let (tool1, freq1) = tools[i];
                    let (tool2, freq2) = tools[j];

                    // Check if these are known alternatives
                    if are_known_alternatives(tool1, tool2) {
                        #[allow(clippy::cast_precision_loss)]
                        let total = (freq1 + freq2) as f32;
                        #[allow(clippy::cast_precision_loss)]
                        let confidence = (*freq1.min(freq2) as f32 / total) * 0.8;

                        self.add_relationship(ToolRelationship::new(
                            tool1.clone(),
                            tool2.clone(),
                            RelationType::Replaces,
                            confidence,
                        ));
                    }
                }
            }
        }
    }

    /// Infer complement relationships from co-occurrence patterns
    fn infer_complement_relationships(&mut self, chains: &[ToolChain]) {
        // Count co-occurrences of tool pairs (not necessarily sequential)
        let mut cooccurrence: HashMap<(String, String), u32> = HashMap::new();

        for chain in chains {
            // For each pair of tools in the chain (not just sequential)
            for i in 0..chain.tools.len() {
                for j in (i + 1)..chain.tools.len() {
                    let tool1 = &chain.tools[i];
                    let tool2 = &chain.tools[j];

                    // Skip if they're already connected as dependencies
                    if self.has_relationship(tool1, tool2, &RelationType::DependsOn) {
                        continue;
                    }

                    let key = if tool1 < tool2 {
                        (tool1.clone(), tool2.clone())
                    } else {
                        (tool2.clone(), tool1.clone())
                    };

                    *cooccurrence.entry(key).or_insert(0) += chain.frequency;
                }
            }
        }

        // Convert frequent co-occurrences to Complements relationships
        for ((tool1, tool2), count) in cooccurrence {
            if count >= 3 {
                // Require at least 3 co-occurrences
                #[allow(clippy::cast_precision_loss)]
                let confidence = ((count.min(10) as f32) / 10.0) * 0.6;

                self.add_relationship(ToolRelationship::new(
                    tool1,
                    tool2,
                    RelationType::Complements,
                    confidence,
                ));
            }
        }
    }

    /// Check if a specific relationship exists
    fn has_relationship(&self, from: &str, to: &str, rel_type: &RelationType) -> bool {
        self.relationships.iter().any(|r| {
            ((r.from_tool == from && r.to_tool == to) || (r.from_tool == to && r.to_tool == from))
                && r.relationship_type == *rel_type
        })
    }

    /// Get all relationships for a specific tool
    #[must_use]
    pub fn get_relationships_for_tool(&self, tool_name: &str) -> Vec<&ToolRelationship> {
        self.relationships
            .iter()
            .filter(|r| r.from_tool == tool_name || r.to_tool == tool_name)
            .collect()
    }
}

/// Check if two tools are known alternatives
#[cfg(feature = "terraphim")]
#[allow(dead_code)] // Used in inference and tests
fn are_known_alternatives(tool1: &str, tool2: &str) -> bool {
    let alternatives = [
        ("npm", "yarn"),
        ("npm", "pnpm"),
        ("yarn", "pnpm"),
        ("npx", "bunx"),
        ("webpack", "vite"),
        ("webpack", "rollup"),
        ("jest", "vitest"),
        ("mocha", "jest"),
        ("eslint", "biome"),
    ];

    alternatives
        .iter()
        .any(|(a, b)| (tool1 == *a && tool2 == *b) || (tool1 == *b && tool2 == *a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_learner_new() {
        let learner = PatternLearner::new();
        assert_eq!(learner.promotion_threshold, 3);
        assert_eq!(learner.candidate_count(), 0);
    }

    #[test]
    fn test_pattern_learner_with_threshold() {
        let learner = PatternLearner::with_threshold(5);
        assert_eq!(learner.promotion_threshold, 5);
    }

    #[test]
    fn test_observe_single_tool() {
        let mut learner = PatternLearner::new();

        learner.observe(
            "pytest".to_string(),
            "pytest tests/".to_string(),
            ToolCategory::Testing,
        );

        assert_eq!(learner.candidate_count(), 1);

        let candidates = learner.get_candidates();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].tool_name, "pytest");
        assert_eq!(candidates[0].observations, 1);
    }

    #[test]
    fn test_observe_multiple_times() {
        let mut learner = PatternLearner::new();

        for i in 0..5 {
            learner.observe(
                "pytest".to_string(),
                format!("pytest tests/test_{i}.py"),
                ToolCategory::Testing,
            );
        }

        assert_eq!(learner.candidate_count(), 1);

        let candidates = learner.get_candidates();
        assert_eq!(candidates[0].observations, 5);
        assert!(candidates[0].contexts.len() <= 10); // Respects limit
    }

    #[test]
    fn test_promote_candidates_threshold_met() {
        let mut learner = PatternLearner::new();

        // Observe 3 times (meets default threshold)
        for i in 0..3 {
            learner.observe(
                "pytest".to_string(),
                format!("pytest tests/test_{i}.py"),
                ToolCategory::Testing,
            );
        }

        let promoted = learner.promote_candidates();

        assert_eq!(promoted.len(), 1);
        assert_eq!(promoted[0].tool_name, "pytest");
        assert_eq!(promoted[0].observations, 3);
        assert!(matches!(promoted[0].category, ToolCategory::Testing));
        assert_eq!(learner.candidate_count(), 0); // Removed after promotion
    }

    #[test]
    fn test_promote_candidates_threshold_not_met() {
        let mut learner = PatternLearner::new();

        // Observe only 2 times (below threshold)
        for i in 0..2 {
            learner.observe(
                "pytest".to_string(),
                format!("pytest tests/test_{i}.py"),
                ToolCategory::Testing,
            );
        }

        let promoted = learner.promote_candidates();

        assert_eq!(promoted.len(), 0);
        assert_eq!(learner.candidate_count(), 1); // Still a candidate
    }

    #[test]
    fn test_category_voting() {
        let mut learner = PatternLearner::new();

        // Vote for Testing twice, BuildTool once
        learner.observe(
            "tool".to_string(),
            "tool test".to_string(),
            ToolCategory::Testing,
        );
        learner.observe(
            "tool".to_string(),
            "tool test2".to_string(),
            ToolCategory::Testing,
        );
        learner.observe(
            "tool".to_string(),
            "tool build".to_string(),
            ToolCategory::BuildTool,
        );

        let promoted = learner.promote_candidates();
        assert_eq!(promoted.len(), 1);
        // Should choose Testing (majority vote)
        assert!(matches!(promoted[0].category, ToolCategory::Testing));
    }

    #[test]
    fn test_confidence_calculation() {
        let mut votes = HashMap::new();
        votes.insert("Testing".to_string(), 3);
        votes.insert("BuildTool".to_string(), 1);

        let confidence = calculate_confidence(&votes, 4);
        assert!((confidence - 0.75).abs() < 0.01); // 3/4 = 0.75
    }

    #[test]
    fn test_infer_category_testing() {
        let contexts = vec!["pytest tests/".to_string(), "pytest --verbose".to_string()];

        let category = infer_category_from_contexts(&contexts);
        assert!(matches!(category, ToolCategory::Testing));
    }

    #[test]
    fn test_infer_category_build_tool() {
        let contexts = vec!["webpack build".to_string(), "vite build".to_string()];

        let category = infer_category_from_contexts(&contexts);
        assert!(matches!(category, ToolCategory::BuildTool));
    }

    #[test]
    fn test_infer_category_linting() {
        let contexts = vec!["eslint src/".to_string(), "cargo clippy".to_string()];

        let category = infer_category_from_contexts(&contexts);
        assert!(matches!(category, ToolCategory::Linting));
    }

    #[test]
    fn test_infer_category_git() {
        let contexts = vec!["git commit".to_string(), "git push".to_string()];

        let category = infer_category_from_contexts(&contexts);
        assert!(matches!(category, ToolCategory::Git));
    }

    #[test]
    fn test_infer_category_package_manager() {
        let contexts = vec!["npm install".to_string(), "yarn add".to_string()];

        let category = infer_category_from_contexts(&contexts);
        assert!(matches!(category, ToolCategory::PackageManager));
    }

    #[test]
    fn test_category_roundtrip() {
        let categories = vec![
            ToolCategory::PackageManager,
            ToolCategory::BuildTool,
            ToolCategory::Testing,
            ToolCategory::Linting,
            ToolCategory::Git,
            ToolCategory::CloudDeploy,
            ToolCategory::Database,
            ToolCategory::Other("custom".to_string()),
        ];

        for category in categories {
            let s = category_to_string(&category);
            let parsed = string_to_category(&s);
            assert_eq!(
                std::mem::discriminant(&category),
                std::mem::discriminant(&parsed)
            );
        }
    }

    #[test]
    fn test_get_cache_path() {
        let path = get_cache_path();
        assert!(path.is_ok());

        let path_buf = path.unwrap();
        assert!(path_buf.to_string_lossy().contains(".config"));
        assert!(path_buf.to_string_lossy().contains("claude-log-analyzer"));
        assert!(path_buf.to_string_lossy().contains("learned_patterns.json"));
    }

    mod proptest_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn test_observe_properties(
                tool_name in "[a-z]{3,15}",
                command in "[a-z ]{5,30}",
                observation_count in 1u32..10
            ) {
                let mut learner = PatternLearner::new();

                for _ in 0..observation_count {
                    learner.observe(
                        tool_name.clone(),
                        command.clone(),
                        ToolCategory::Testing
                    );
                }

                // Property 1: Should always have exactly one candidate for one tool
                prop_assert_eq!(learner.candidate_count(), 1);

                // Property 2: Observation count should match
                let candidates = learner.get_candidates();
                prop_assert_eq!(candidates[0].observations, observation_count);

                // Property 3: Tool name should be preserved
                prop_assert_eq!(&candidates[0].tool_name, &tool_name);
            }

            #[test]
            fn test_promotion_threshold_properties(
                threshold in 1u32..20,
                observations in 1u32..20
            ) {
                let mut learner = PatternLearner::with_threshold(threshold);

                for _ in 0..observations {
                    learner.observe(
                        "tool".to_string(),
                        "command".to_string(),
                        ToolCategory::Testing
                    );
                }

                let promoted = learner.promote_candidates();

                // Property: Promotion happens if and only if observations >= threshold
                if observations >= threshold {
                    prop_assert_eq!(promoted.len(), 1);
                    prop_assert_eq!(learner.candidate_count(), 0);
                } else {
                    prop_assert_eq!(promoted.len(), 0);
                    prop_assert_eq!(learner.candidate_count(), 1);
                }
            }

            #[test]
            fn test_confidence_properties(
                winning_votes in 1u32..100,
                losing_votes in 0u32..100
            ) {
                let total = winning_votes + losing_votes;
                if total == 0 {
                    return Ok(());
                }

                let mut votes = HashMap::new();
                votes.insert("Category1".to_string(), winning_votes);
                if losing_votes > 0 {
                    votes.insert("Category2".to_string(), losing_votes);
                }

                let confidence = calculate_confidence(&votes, total);

                // Property 1: Confidence should be in valid range
                prop_assert!((0.0..=1.0).contains(&confidence));

                // Property 2: Confidence should match the max vote proportion
                #[allow(clippy::cast_precision_loss)]
                let max_votes = winning_votes.max(losing_votes);
                let expected = (max_votes as f32) / (total as f32);
                prop_assert!((confidence - expected).abs() < 0.01);
            }
        }
    }

    // ============================================================================
    // Terraphim Feature Tests
    // ============================================================================

    #[cfg(feature = "terraphim")]
    mod terraphim_tests {
        use super::*;

        #[test]
        fn test_tool_relationship_new() {
            let rel = ToolRelationship::new(
                "npm".to_string(),
                "wrangler".to_string(),
                RelationType::DependsOn,
                0.8,
            );

            assert_eq!(rel.from_tool, "npm");
            assert_eq!(rel.to_tool, "wrangler");
            assert_eq!(rel.relationship_type, RelationType::DependsOn);
            assert!((rel.confidence - 0.8).abs() < 0.01);
        }

        #[test]
        fn test_tool_relationship_confidence_clamp() {
            // Test upper bound
            let rel = ToolRelationship::new(
                "npm".to_string(),
                "wrangler".to_string(),
                RelationType::DependsOn,
                1.5,
            );
            assert!((rel.confidence - 1.0).abs() < 0.01);

            // Test lower bound
            let rel = ToolRelationship::new(
                "npm".to_string(),
                "wrangler".to_string(),
                RelationType::DependsOn,
                -0.5,
            );
            assert!((rel.confidence - 0.0).abs() < 0.01);
        }

        #[test]
        fn test_infer_from_chain_sequential_tools() {
            let chain = ToolChain {
                tools: vec!["git".to_string(), "npm".to_string(), "wrangler".to_string()],
                frequency: 5,
                average_time_between_ms: 1000,
                typical_agent: Some("devops".to_string()),
                success_rate: 0.9,
            };

            let relationships = ToolRelationship::infer_from_chain(&chain);

            // Should create 2 relationships (git->npm, npm->wrangler)
            assert_eq!(relationships.len(), 2);

            // All should be DependsOn type
            for rel in &relationships {
                assert_eq!(rel.relationship_type, RelationType::DependsOn);
                assert!(rel.confidence > 0.0);
                assert!(rel.confidence <= 1.0);
            }
        }

        #[test]
        fn test_infer_from_chain_known_dependency() {
            let chain = ToolChain {
                tools: vec!["npm".to_string(), "wrangler".to_string()],
                frequency: 10,
                average_time_between_ms: 500,
                typical_agent: Some("devops".to_string()),
                success_rate: 1.0,
            };

            let relationships = ToolRelationship::infer_from_chain(&chain);

            assert_eq!(relationships.len(), 1);
            let rel = &relationships[0];

            // Known dependency should have boosted confidence
            assert!(rel.confidence > 0.9);
        }

        #[test]
        fn test_knowledge_graph_new() {
            let graph = KnowledgeGraph::new();
            assert_eq!(graph.relationships.len(), 0);
        }

        #[test]
        fn test_knowledge_graph_add_relationship() {
            let mut graph = KnowledgeGraph::new();

            let rel = ToolRelationship::new(
                "npm".to_string(),
                "wrangler".to_string(),
                RelationType::DependsOn,
                0.8,
            );

            graph.add_relationship(rel);
            assert_eq!(graph.relationships.len(), 1);
        }

        #[test]
        fn test_knowledge_graph_deduplication() {
            let mut graph = KnowledgeGraph::new();

            // Add same relationship twice with different confidence
            let rel1 = ToolRelationship::new(
                "npm".to_string(),
                "wrangler".to_string(),
                RelationType::DependsOn,
                0.6,
            );
            let rel2 = ToolRelationship::new(
                "npm".to_string(),
                "wrangler".to_string(),
                RelationType::DependsOn,
                0.8,
            );

            graph.add_relationship(rel1);
            graph.add_relationship(rel2);

            // Should have only one relationship (deduplicated)
            assert_eq!(graph.relationships.len(), 1);

            // Confidence should be averaged
            let rel = &graph.relationships[0];
            assert!((rel.confidence - 0.7).abs() < 0.01);
        }

        #[test]
        fn test_knowledge_graph_build_from_chains() {
            let chains = vec![
                ToolChain {
                    tools: vec!["git".to_string(), "npm".to_string()],
                    frequency: 10,
                    average_time_between_ms: 500,
                    typical_agent: Some("developer".to_string()),
                    success_rate: 0.95,
                },
                ToolChain {
                    tools: vec!["npm".to_string(), "wrangler".to_string()],
                    frequency: 8,
                    average_time_between_ms: 1000,
                    typical_agent: Some("devops".to_string()),
                    success_rate: 0.9,
                },
            ];

            let graph = KnowledgeGraph::build_from_chains(&chains);

            // Should have DependsOn relationships from both chains
            assert!(!graph.relationships.is_empty());

            // Check that DependsOn relationships exist
            let depends_on_count = graph
                .relationships
                .iter()
                .filter(|r| r.relationship_type == RelationType::DependsOn)
                .count();
            assert!(depends_on_count >= 2);
        }

        #[test]
        fn test_knowledge_graph_replacement_relationships() {
            let chains = vec![
                ToolChain {
                    tools: vec!["npm".to_string(), "build".to_string()],
                    frequency: 5,
                    average_time_between_ms: 1000,
                    typical_agent: Some("developer".to_string()),
                    success_rate: 0.9,
                },
                ToolChain {
                    tools: vec!["yarn".to_string(), "build".to_string()],
                    frequency: 5,
                    average_time_between_ms: 1000,
                    typical_agent: Some("developer".to_string()),
                    success_rate: 0.9,
                },
            ];

            let graph = KnowledgeGraph::build_from_chains(&chains);

            // Should identify npm and yarn as alternatives (Replaces relationship)
            let replaces_count = graph
                .relationships
                .iter()
                .filter(|r| r.relationship_type == RelationType::Replaces)
                .count();
            assert!(replaces_count > 0);
        }

        #[test]
        fn test_knowledge_graph_get_relationships_for_tool() {
            let mut graph = KnowledgeGraph::new();

            graph.add_relationship(ToolRelationship::new(
                "npm".to_string(),
                "wrangler".to_string(),
                RelationType::DependsOn,
                0.8,
            ));
            graph.add_relationship(ToolRelationship::new(
                "git".to_string(),
                "npm".to_string(),
                RelationType::Complements,
                0.7,
            ));

            let npm_rels = graph.get_relationships_for_tool("npm");

            // npm should have 2 relationships
            assert_eq!(npm_rels.len(), 2);
        }

        #[test]
        fn test_are_known_alternatives() {
            assert!(are_known_alternatives("npm", "yarn"));
            assert!(are_known_alternatives("yarn", "npm"));
            assert!(are_known_alternatives("npx", "bunx"));
            assert!(are_known_alternatives("webpack", "vite"));
            assert!(!are_known_alternatives("npm", "cargo"));
        }

        #[test]
        fn test_is_known_dependency() {
            assert!(is_known_dependency("npm", "wrangler"));
            assert!(is_known_dependency("cargo", "clippy"));
            assert!(is_known_dependency("git", "npm"));
            assert!(!is_known_dependency("random", "tool"));
        }
    }
}
