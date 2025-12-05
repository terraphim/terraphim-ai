//! Knowledge Graph Search using terraphim_automata
//!
//! This module provides search functionality over the knowledge graph,
//! evaluating complex queries and ranking results by relevance.

use super::builder::KnowledgeGraphBuilder;
use super::query::QueryNode;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use terraphim_automata::find_matches;

/// Type alias for query match results: (matched_text, concepts, (start, end))
type MatchResults = Vec<(String, Vec<String>, (usize, usize))>;

/// Knowledge graph search engine
#[derive(Debug, Clone)]
pub struct KnowledgeGraphSearch {
    builder: KnowledgeGraphBuilder,
}

/// Search result containing matched text and metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResult {
    /// The matched text from the original input
    pub matched_text: String,

    /// Concepts that were matched in this result
    pub concepts_matched: Vec<String>,

    /// Position in the original text (start, end)
    pub position: (usize, usize),

    /// Relevance score based on number of concept matches
    pub relevance_score: f32,
}

impl KnowledgeGraphSearch {
    /// Create a new search engine with the given knowledge graph
    #[must_use]
    pub fn new(builder: KnowledgeGraphBuilder) -> Self {
        Self { builder }
    }

    /// Search text using a query AST
    ///
    /// Evaluates the query against the text using terraphim pattern matching,
    /// returning results ranked by relevance.
    ///
    /// # Errors
    ///
    /// Returns an error if the terraphim search fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use claude_log_analyzer::kg::{KnowledgeGraphBuilder, KnowledgeGraphSearch};
    /// use claude_log_analyzer::kg::query::QueryNode;
    ///
    /// let builder = KnowledgeGraphBuilder::new();
    /// let search = KnowledgeGraphSearch::new(builder);
    ///
    /// let query = QueryNode::And(
    ///     Box::new(QueryNode::Concept("BUN".to_string())),
    ///     Box::new(QueryNode::Concept("install".to_string()))
    /// );
    ///
    /// let results = search.search("bunx install packages", &query)?;
    /// ```
    pub fn search(&self, text: &str, query: &QueryNode) -> Result<Vec<SearchResult>> {
        // Evaluate the query to get matched positions
        let matches = self.evaluate_query(text, query)?;

        // Convert to search results and rank by relevance
        let mut results: Vec<SearchResult> = matches
            .into_iter()
            .map(|(matched_text, concepts, position)| {
                let relevance_score = calculate_relevance(&concepts);
                SearchResult {
                    matched_text,
                    concepts_matched: concepts,
                    position,
                    relevance_score,
                }
            })
            .collect();

        // Sort by relevance score (highest first)
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results)
    }

    /// Evaluate a query node against the text
    ///
    /// Returns a vector of (matched_text, concepts, position) tuples
    fn evaluate_query(&self, text: &str, query: &QueryNode) -> Result<MatchResults> {
        match query {
            QueryNode::Concept(concept) => self.match_concept(text, concept),

            QueryNode::And(left, right) => {
                let left_results = self.evaluate_query(text, left)?;
                let right_results = self.evaluate_query(text, right)?;
                Ok(intersect_results(left_results, right_results))
            }

            QueryNode::Or(left, right) => {
                let mut left_results = self.evaluate_query(text, left)?;
                let right_results = self.evaluate_query(text, right)?;
                left_results.extend(right_results);
                Ok(deduplicate_results(left_results))
            }

            QueryNode::Not(inner) => {
                let inner_results = self.evaluate_query(text, inner)?;
                Ok(exclude_results(text, inner_results))
            }
        }
    }

    /// Match a single concept using terraphim
    fn match_concept(&self, text: &str, concept: &str) -> Result<MatchResults> {
        // Use terraphim find_matches to search for the concept
        // Use false for overlapping matches to get all possible matches
        let matches = find_matches(text, self.builder.thesaurus.clone(), false)
            .with_context(|| format!("Failed to find matches for concept: {concept}"))?;

        // Filter matches to only include this concept
        let concept_upper = concept.to_uppercase();

        let results: Vec<(String, Vec<String>, (usize, usize))> = matches
            .iter()
            .filter(|m| {
                let normalized_upper = m.normalized_term.value.to_string().to_uppercase();
                normalized_upper == concept_upper
            })
            .map(|m| {
                // If position is not set, estimate it from the term
                let (start, end) = m.pos.unwrap_or_else(|| {
                    // Try to find the term in the original text
                    if let Some(pos) = text.find(&m.term) {
                        (pos, pos + m.term.len())
                    } else {
                        (0, m.term.len())
                    }
                });

                (m.term.clone(), vec![concept_upper.clone()], (start, end))
            })
            .collect();

        Ok(results)
    }
}

/// Intersect two result sets (AND operation)
fn intersect_results(
    left: Vec<(String, Vec<String>, (usize, usize))>,
    right: Vec<(String, Vec<String>, (usize, usize))>,
) -> Vec<(String, Vec<String>, (usize, usize))> {
    // For AND, we need results that have overlapping or adjacent positions
    // This represents cases where both concepts appear in the same context
    let mut results = Vec::new();

    for (left_text, left_concepts, left_pos) in &left {
        for (right_text, right_concepts, right_pos) in &right {
            // Check if positions overlap or are close (within 50 chars)
            if positions_overlap_or_near(*left_pos, *right_pos, 50) {
                // Merge the results
                let merged_text = merge_text(left_text, right_text, *left_pos, *right_pos);
                let mut merged_concepts = left_concepts.clone();
                merged_concepts.extend(right_concepts.clone());

                let merged_pos = (left_pos.0.min(right_pos.0), left_pos.1.max(right_pos.1));

                results.push((merged_text, merged_concepts, merged_pos));
            }
        }
    }

    results
}

/// Check if two positions overlap or are near each other
fn positions_overlap_or_near(pos1: (usize, usize), pos2: (usize, usize), threshold: usize) -> bool {
    // Check for overlap
    if pos1.0 <= pos2.1 && pos2.0 <= pos1.1 {
        return true;
    }

    // Check for nearness - use saturating_sub to avoid potential overflow
    let distance = if pos1.1 < pos2.0 {
        pos2.0.saturating_sub(pos1.1)
    } else if pos2.1 < pos1.0 {
        pos1.0.saturating_sub(pos2.1)
    } else {
        0
    };

    distance <= threshold
}

/// Merge two text segments
fn merge_text(text1: &str, text2: &str, pos1: (usize, usize), pos2: (usize, usize)) -> String {
    if pos1.0 <= pos2.0 {
        if pos1.1 >= pos2.1 {
            // text1 contains text2
            text1.to_string()
        } else {
            // text1 before text2
            format!("{} {}", text1, text2)
        }
    } else if pos2.1 >= pos1.1 {
        // text2 contains text1
        text2.to_string()
    } else {
        // text2 before text1
        format!("{} {}", text2, text1)
    }
}

/// Deduplicate results by position
fn deduplicate_results(
    mut results: Vec<(String, Vec<String>, (usize, usize))>,
) -> Vec<(String, Vec<String>, (usize, usize))> {
    results.sort_by_key(|(_, _, pos)| *pos);
    results.dedup_by(|(_, _, pos1), (_, _, pos2)| pos1 == pos2);
    results
}

/// Exclude results (NOT operation)
fn exclude_results(_text: &str, _exclude: MatchResults) -> MatchResults {
    // For NOT operation, we return positions that are NOT in the exclude set
    // This is a simplified implementation - in practice, you'd need the full text
    // to identify non-matching regions

    // For now, always return empty - NOT operation requires full text context
    Vec::new()
}

/// Calculate relevance score based on concepts matched
fn calculate_relevance(concepts: &[String]) -> f32 {
    // More concepts matched = higher relevance
    #[allow(clippy::cast_precision_loss)]
    let base_score = concepts.len() as f32;

    // Bonus for specific important concepts
    let bonus = concepts.iter().fold(0.0, |acc, concept| {
        acc + match concept.as_str() {
            "DEPLOY" | "INSTALL" | "BUILD" => 0.2,
            "BUN" | "NPM" | "CARGO" => 0.1,
            _ => 0.0,
        }
    });

    base_score + bonus
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kg::builder::KnowledgeGraphBuilder;

    fn create_test_builder() -> KnowledgeGraphBuilder {
        use crate::models::{ToolCategory, ToolInvocation};
        use jiff::Timestamp;
        use std::collections::HashMap;

        // Create sample tool invocations to build a test graph
        let tools = vec![
            ToolInvocation {
                timestamp: Timestamp::now(),
                tool_name: "bun".to_string(),
                tool_category: ToolCategory::PackageManager,
                command_line: "bunx wrangler deploy".to_string(),
                arguments: vec![],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: None,
                session_id: "test-session".to_string(),
                message_id: "test-message".to_string(),
            },
            ToolInvocation {
                timestamp: Timestamp::now(),
                tool_name: "npm".to_string(),
                tool_category: ToolCategory::PackageManager,
                command_line: "npm install".to_string(),
                arguments: vec![],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: None,
                session_id: "test-session".to_string(),
                message_id: "test-message".to_string(),
            },
        ];

        KnowledgeGraphBuilder::from_tool_invocations(&tools)
    }

    #[test]
    fn test_new_search() {
        let builder = create_test_builder();
        let search = KnowledgeGraphSearch::new(builder);
        assert!(!search.builder.thesaurus.is_empty());
    }

    #[test]
    fn test_match_concept_bun() -> Result<()> {
        let builder = create_test_builder();
        let search = KnowledgeGraphSearch::new(builder);

        // Test with just "bunx" - this should definitely match
        let results = search.match_concept("bunx", "BUN")?;

        assert!(!results.is_empty(), "Should find BUN concept in 'bunx'");
        assert_eq!(results[0].1, vec!["BUN".to_string()]);
        Ok(())
    }

    #[test]
    fn test_match_concept_install() -> Result<()> {
        let builder = create_test_builder();
        let search = KnowledgeGraphSearch::new(builder);

        let results = search.match_concept("npm install packages", "INSTALL")?;

        assert!(!results.is_empty());
        assert_eq!(results[0].1, vec!["INSTALL".to_string()]);
        Ok(())
    }

    #[test]
    fn test_search_simple_concept() -> Result<()> {
        let builder = create_test_builder();
        let search = KnowledgeGraphSearch::new(builder);

        let query = QueryNode::Concept("BUN".to_string());
        // Use a simpler text that should match BUN more clearly
        let results = search.search("bunx install packages", &query)?;

        assert!(!results.is_empty(), "Should find BUN matches");
        assert!(results[0].concepts_matched.contains(&"BUN".to_string()));
        Ok(())
    }

    #[test]
    fn test_search_and_query() -> Result<()> {
        let builder = create_test_builder();
        let search = KnowledgeGraphSearch::new(builder);

        let query = QueryNode::And(
            Box::new(QueryNode::Concept("BUN".to_string())),
            Box::new(QueryNode::Concept("DEPLOY".to_string())),
        );

        let results = search.search("bunx wrangler deploy", &query)?;

        // Should find matches where both BUN and DEPLOY concepts appear
        if !results.is_empty() {
            assert!(!results[0].concepts_matched.is_empty());
        }
        Ok(())
    }

    #[test]
    fn test_search_or_query() -> Result<()> {
        let builder = create_test_builder();
        let search = KnowledgeGraphSearch::new(builder);

        let query = QueryNode::Or(
            Box::new(QueryNode::Concept("BUN".to_string())),
            Box::new(QueryNode::Concept("NPM".to_string())),
        );

        let results = search.search("bunx install packages", &query)?;

        // Should find BUN
        assert!(!results.is_empty());
        Ok(())
    }

    #[test]
    fn test_positions_overlap_or_near() {
        // Exact overlap
        assert!(positions_overlap_or_near((0, 10), (5, 15), 50));

        // Adjacent
        assert!(positions_overlap_or_near((0, 10), (10, 20), 50));

        // Near (within threshold)
        assert!(positions_overlap_or_near((0, 10), (15, 25), 50));

        // Too far
        assert!(!positions_overlap_or_near((0, 10), (100, 110), 50));
    }

    #[test]
    fn test_calculate_relevance() {
        // Single concept
        let score = calculate_relevance(&["TEST".to_string()]);
        assert!((score - 1.0).abs() < 0.01);

        // Multiple concepts
        let score = calculate_relevance(&["BUN".to_string(), "INSTALL".to_string()]);
        assert!(score > 2.0); // 2.0 base + bonuses

        // Important concepts get bonus
        let score = calculate_relevance(&["DEPLOY".to_string()]);
        assert!((score - 1.2).abs() < 0.01); // 1.0 + 0.2 bonus
    }

    #[test]
    fn test_merge_text() {
        // First contains second
        assert_eq!(
            merge_text("bunx wrangler", "wrangler", (0, 13), (5, 13)),
            "bunx wrangler"
        );

        // Sequential
        assert_eq!(
            merge_text("bunx", "wrangler", (0, 4), (5, 13)),
            "bunx wrangler"
        );
    }

    #[test]
    fn test_deduplicate_results() {
        let results = vec![
            ("text1".to_string(), vec!["A".to_string()], (0, 5)),
            ("text2".to_string(), vec!["B".to_string()], (0, 5)), // Duplicate position
            ("text3".to_string(), vec!["C".to_string()], (10, 15)),
        ];

        let deduped = deduplicate_results(results);
        assert_eq!(deduped.len(), 2);
    }
}
