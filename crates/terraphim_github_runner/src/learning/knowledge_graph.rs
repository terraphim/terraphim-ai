//! Knowledge graph integration for command pattern learning
//!
//! This module provides:
//! - `CommandKnowledgeGraph` - wrapper around RoleGraph for command sequences
//! - Edge types via document ID prefixes (success, failure, workflow)
//! - Command-to-node ID mapping with persistent storage

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use terraphim_rolegraph::{magic_pair, RoleGraph};
use tokio::sync::RwLock;

use super::thesaurus::{build_command_thesaurus, get_command_id, normalize_command};
use crate::Result;

/// Edge type prefixes for document IDs
const SUCCESS_PREFIX: &str = "success:";
const FAILURE_PREFIX: &str = "failure:";
const WORKFLOW_PREFIX: &str = "workflow:";

/// Knowledge graph for learning command execution patterns
///
/// Wraps a separate RoleGraph instance dedicated to command patterns,
/// distinct from the main document graph.
pub struct CommandKnowledgeGraph {
    /// The underlying RoleGraph for command relationships
    graph: Arc<RwLock<RoleGraph>>,
    /// Mapping from normalized command strings to node IDs
    command_to_node: DashMap<String, u64>,
    /// Statistics tracker
    stats: Arc<RwLock<CommandGraphStats>>,
}

/// Statistics about the command knowledge graph
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommandGraphStats {
    /// Total success edges recorded
    pub success_edges: u64,
    /// Total failure edges recorded
    pub failure_edges: u64,
    /// Total workflow edges recorded
    pub workflow_edges: u64,
    /// Unique commands tracked
    pub unique_commands: usize,
}

impl CommandKnowledgeGraph {
    /// Create a new command knowledge graph
    pub async fn new() -> Result<Self> {
        let thesaurus = build_command_thesaurus();
        let role_name = "command-patterns".to_string();

        let graph = RoleGraph::new(role_name.into(), thesaurus)
            .await
            .map_err(|e| {
                crate::error::GitHubRunnerError::Internal(format!(
                    "Failed to create RoleGraph: {}",
                    e
                ))
            })?;

        Ok(Self {
            graph: Arc::new(RwLock::new(graph)),
            command_to_node: DashMap::new(),
            stats: Arc::new(RwLock::new(CommandGraphStats::default())),
        })
    }

    /// Get or create a node ID for a command
    ///
    /// Uses the existing mapping if available, otherwise creates a new ID
    /// and adds to the mapping.
    pub fn get_or_create_node_id(&self, command: &str) -> u64 {
        let normalized = normalize_command(command);

        *self
            .command_to_node
            .entry(normalized)
            .or_insert_with(get_command_id)
    }

    /// Record a successful command sequence
    ///
    /// Creates an edge between two commands indicating that cmd2
    /// successfully followed cmd1 in execution order.
    pub async fn record_success_sequence(
        &self,
        cmd1: &str,
        cmd2: &str,
        context_id: &str,
    ) -> Result<()> {
        let node1 = self.get_or_create_node_id(cmd1);
        let node2 = self.get_or_create_node_id(cmd2);

        // Create document ID with success prefix
        let doc_id = format!(
            "{}{}:{}:{}",
            SUCCESS_PREFIX,
            normalize_command(cmd1),
            normalize_command(cmd2),
            context_id
        );

        // Add edge to graph
        let mut graph = self.graph.write().await;
        graph.add_or_update_document(&doc_id, node1, node2);

        // Update stats
        let mut stats = self.stats.write().await;
        stats.success_edges += 1;
        stats.unique_commands = self.command_to_node.len();

        log::debug!(
            "Recorded success edge: {} -> {} (nodes: {} -> {})",
            cmd1,
            cmd2,
            node1,
            node2
        );

        Ok(())
    }

    /// Record a command failure
    ///
    /// Creates a failure edge linking the command to its error signature.
    /// Error signatures are also treated as nodes for pattern matching.
    pub async fn record_failure(
        &self,
        command: &str,
        error_signature: &str,
        context_id: &str,
    ) -> Result<()> {
        let cmd_node = self.get_or_create_node_id(command);

        // Create a pseudo-node for the error signature
        let error_key = format!("error:{}", truncate_error(error_signature));
        let error_node = self.get_or_create_node_id(&error_key);

        // Create document ID with failure prefix
        let doc_id = format!(
            "{}{}:{}:{}",
            FAILURE_PREFIX,
            normalize_command(command),
            truncate_error(error_signature),
            context_id
        );

        // Add edge to graph
        let mut graph = self.graph.write().await;
        graph.add_or_update_document(&doc_id, cmd_node, error_node);

        // Update stats
        let mut stats = self.stats.write().await;
        stats.failure_edges += 1;
        stats.unique_commands = self.command_to_node.len();

        log::debug!(
            "Recorded failure edge: {} -> error:{} (nodes: {} -> {})",
            command,
            truncate_error(error_signature),
            cmd_node,
            error_node
        );

        Ok(())
    }

    /// Record all commands in a workflow as related
    ///
    /// Creates edges between all pairs of commands in the workflow,
    /// indicating they belong to the same execution context.
    pub async fn record_workflow(&self, commands: &[String], session_id: &str) -> Result<()> {
        if commands.len() < 2 {
            return Ok(());
        }

        let doc_id = format!("{}{}", WORKFLOW_PREFIX, session_id);
        let mut graph = self.graph.write().await;

        // Create edges between consecutive commands
        for window in commands.windows(2) {
            let node1 = self.get_or_create_node_id(&window[0]);
            let node2 = self.get_or_create_node_id(&window[1]);
            graph.add_or_update_document(&doc_id, node1, node2);
        }

        // Update stats
        let mut stats = self.stats.write().await;
        stats.workflow_edges += commands.len().saturating_sub(1) as u64;
        stats.unique_commands = self.command_to_node.len();

        log::debug!(
            "Recorded workflow with {} commands (session: {})",
            commands.len(),
            session_id
        );

        Ok(())
    }

    /// Find commands related to the given command
    ///
    /// Returns commands that have been executed in sequence with the given command.
    pub async fn find_related_commands(&self, command: &str, limit: usize) -> Result<Vec<String>> {
        let normalized = normalize_command(command);
        let graph = self.graph.read().await;

        // Query the graph for related documents
        let results = graph
            .query_graph(&normalized, Some(0), Some(limit))
            .map_err(|e| {
                crate::error::GitHubRunnerError::Internal(format!("Graph query failed: {}", e))
            })?;

        // Extract command names from document IDs
        let related: Vec<String> = results
            .into_iter()
            .filter_map(|(doc_id, _)| extract_command_from_doc_id(&doc_id))
            .collect();

        Ok(related)
    }

    /// Predict success probability for a command sequence
    ///
    /// Returns a confidence score (0.0-1.0) based on historical success/failure
    /// edges between the two commands.
    pub async fn predict_success(&self, cmd1: &str, cmd2: &str) -> f64 {
        let node1 = self.get_or_create_node_id(cmd1);
        let node2 = self.get_or_create_node_id(cmd2);

        let edge_id = magic_pair(node1, node2);
        let graph = self.graph.read().await;

        // Check if edge exists
        if let Some(edge) = graph.edges_map().get(&edge_id) {
            // Count success vs failure documents
            let mut success_count = 0u32;
            let mut failure_count = 0u32;

            for doc_id in edge.doc_hash.keys() {
                if doc_id.starts_with(SUCCESS_PREFIX) {
                    success_count += 1;
                } else if doc_id.starts_with(FAILURE_PREFIX) {
                    failure_count += 1;
                }
            }

            let total = success_count + failure_count;
            if total > 0 {
                return success_count as f64 / total as f64;
            }
        }

        // No historical data, return neutral probability
        0.5
    }

    /// Get statistics about the command graph
    pub async fn get_stats(&self) -> CommandGraphStats {
        let stats = self.stats.read().await;
        CommandGraphStats {
            unique_commands: self.command_to_node.len(),
            ..*stats
        }
    }

    /// Get the number of unique commands tracked
    pub fn command_count(&self) -> usize {
        self.command_to_node.len()
    }

    /// Check if a command has been seen before
    pub fn has_command(&self, command: &str) -> bool {
        let normalized = normalize_command(command);
        self.command_to_node.contains_key(&normalized)
    }
}

/// Truncate error message to a reasonable signature length
fn truncate_error(error: &str) -> String {
    let first_line = error.lines().next().unwrap_or(error);
    if first_line.len() > 50 {
        format!("{}...", &first_line[..50])
    } else {
        first_line.to_string()
    }
}

/// Extract command name from document ID
fn extract_command_from_doc_id(doc_id: &str) -> Option<String> {
    // Document ID format: "{prefix}{cmd1}:{cmd2}:{context}"
    let stripped = doc_id
        .strip_prefix(SUCCESS_PREFIX)
        .or_else(|| doc_id.strip_prefix(FAILURE_PREFIX))
        .or_else(|| doc_id.strip_prefix(WORKFLOW_PREFIX))?;

    // Get the first command part
    stripped.split(':').next().map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_knowledge_graph_creation() {
        let kg = CommandKnowledgeGraph::new().await.unwrap();
        assert_eq!(kg.command_count(), 0);
    }

    #[tokio::test]
    async fn test_get_or_create_node_id() {
        let kg = CommandKnowledgeGraph::new().await.unwrap();

        let id1 = kg.get_or_create_node_id("cargo build");
        let id2 = kg.get_or_create_node_id("cargo build --release");
        let id3 = kg.get_or_create_node_id("cargo test");

        // Same normalized command should get same ID
        assert_eq!(id1, id2);
        // Different command should get different ID
        assert_ne!(id1, id3);
    }

    #[tokio::test]
    async fn test_record_success_sequence() {
        let kg = CommandKnowledgeGraph::new().await.unwrap();

        kg.record_success_sequence("cargo build", "cargo test", "session1")
            .await
            .unwrap();

        let stats = kg.get_stats().await;
        assert_eq!(stats.success_edges, 1);
        assert_eq!(stats.unique_commands, 2);
    }

    #[tokio::test]
    async fn test_record_failure() {
        let kg = CommandKnowledgeGraph::new().await.unwrap();

        kg.record_failure("cargo build", "error[E0432]: unresolved import", "session1")
            .await
            .unwrap();

        let stats = kg.get_stats().await;
        assert_eq!(stats.failure_edges, 1);
    }

    #[tokio::test]
    async fn test_record_workflow() {
        let kg = CommandKnowledgeGraph::new().await.unwrap();

        let commands = vec![
            "cargo fmt".to_string(),
            "cargo clippy".to_string(),
            "cargo build".to_string(),
            "cargo test".to_string(),
        ];

        kg.record_workflow(&commands, "session1").await.unwrap();

        let stats = kg.get_stats().await;
        assert_eq!(stats.workflow_edges, 3); // 3 edges for 4 commands
        assert_eq!(stats.unique_commands, 4);
    }

    #[tokio::test]
    async fn test_predict_success() {
        let kg = CommandKnowledgeGraph::new().await.unwrap();

        // Record some success patterns
        for i in 0..3 {
            kg.record_success_sequence("cargo build", "cargo test", &format!("s{}", i))
                .await
                .unwrap();
        }

        // Record one failure
        kg.record_failure("cargo build", "build failed", "f1")
            .await
            .unwrap();

        // Prediction should reflect success rate
        let prob = kg.predict_success("cargo build", "cargo test").await;
        assert!(prob > 0.5); // More successes than failures
    }

    #[tokio::test]
    async fn test_truncate_error() {
        assert_eq!(truncate_error("short error"), "short error");
        // Function truncates at 50 chars, so "this is a very long error message that should be t" + "..."
        let long_error = "this is a very long error message that should be truncated to fit";
        let truncated = truncate_error(long_error);
        assert!(truncated.len() <= 53); // 50 chars + "..."
        assert!(truncated.ends_with("..."));
        assert_eq!(truncate_error("line1\nline2\nline3"), "line1");
    }

    #[tokio::test]
    async fn test_extract_command_from_doc_id() {
        assert_eq!(
            extract_command_from_doc_id("success:cargo build:cargo test:session1"),
            Some("cargo build".to_string())
        );
        assert_eq!(
            extract_command_from_doc_id("failure:cargo build:error:session1"),
            Some("cargo build".to_string())
        );
        assert_eq!(
            extract_command_from_doc_id("workflow:session1"),
            Some("session1".to_string())
        );
        assert_eq!(extract_command_from_doc_id("invalid"), None);
    }
}
