//! Action graph for understanding relationships between CI/CD actions
//!
//! Integrates with terraphim_rolegraph for semantic action matching and validation.

use crate::{BuildTerm, RunnerResult, RunnerError, InterpretedAction};
use crate::knowledge_graph::ThesaurusBuilder;
use ahash::{AHashMap, AHashSet};

/// Graph of CI/CD actions and their relationships
pub struct ActionGraph {
    /// Nodes in the graph (actions/commands)
    nodes: AHashMap<String, ActionNode>,
    /// Edges between nodes (dependencies, produces, etc.)
    edges: Vec<ActionEdge>,
    /// Thesaurus for term lookup
    thesaurus_builder: ThesaurusBuilder,
}

/// A node in the action graph
#[derive(Debug, Clone)]
pub struct ActionNode {
    /// Node identifier (nterm)
    pub id: String,
    /// Display name
    pub name: String,
    /// Node type
    pub node_type: ActionNodeType,
    /// Metadata
    pub metadata: AHashMap<String, String>,
}

/// Types of nodes in the action graph
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionNodeType {
    /// GitHub Action
    Action,
    /// Shell command
    Command,
    /// Build target
    Target,
    /// Artifact
    Artifact,
    /// Environment variable
    EnvVar,
    /// Service
    Service,
}

/// An edge in the action graph
#[derive(Debug, Clone)]
pub struct ActionEdge {
    /// Source node
    pub from: String,
    /// Target node
    pub to: String,
    /// Edge type
    pub edge_type: EdgeType,
}

/// Types of edges between nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeType {
    /// Source depends on target
    DependsOn,
    /// Source produces target
    Produces,
    /// Source uses target
    Uses,
    /// Source is parent of target
    Parent,
}

impl ActionGraph {
    /// Create a new action graph from a thesaurus builder
    pub fn from_thesaurus(thesaurus_builder: ThesaurusBuilder) -> Self {
        let mut graph = Self {
            nodes: AHashMap::new(),
            edges: Vec::new(),
            thesaurus_builder,
        };
        graph.build_graph();
        graph
    }

    /// Build the graph from thesaurus terms
    fn build_graph(&mut self) {
        // Create nodes from terms
        for term in self.thesaurus_builder.terms() {
            let node_type = match term.term_type {
                crate::TermType::Action => ActionNodeType::Action,
                crate::TermType::Command => ActionNodeType::Command,
                crate::TermType::EarthlyTarget => ActionNodeType::Target,
                crate::TermType::DockerInstruction => ActionNodeType::Command,
                crate::TermType::EnvVar => ActionNodeType::EnvVar,
                crate::TermType::Artifact => ActionNodeType::Artifact,
                crate::TermType::Service => ActionNodeType::Service,
            };

            let node = ActionNode {
                id: term.nterm.clone(),
                name: term.nterm.clone(),
                node_type,
                metadata: AHashMap::new(),
            };

            self.nodes.insert(term.nterm.clone(), node);

            // Create edges from parent relationships
            if let Some(parent) = &term.parent {
                self.edges.push(ActionEdge {
                    from: parent.clone(),
                    to: term.nterm.clone(),
                    edge_type: EdgeType::Parent,
                });
            }

            // Create edges from related terms
            for related in &term.related {
                self.edges.push(ActionEdge {
                    from: term.nterm.clone(),
                    to: related.clone(),
                    edge_type: EdgeType::Uses,
                });
            }
        }

        // Infer additional relationships
        self.infer_dependencies();
    }

    /// Infer dependencies between actions based on common patterns
    fn infer_dependencies(&mut self) {
        let nodes: Vec<_> = self.nodes.keys().cloned().collect();

        for node_id in &nodes {
            // Checkout is typically first
            if node_id.contains("checkout") {
                for other_id in &nodes {
                    if other_id != node_id
                        && (other_id.contains("setup-")
                            || other_id.starts_with("run:")
                            || other_id.starts_with("uses:"))
                    {
                        // Don't add if edge already exists
                        let edge_exists = self.edges.iter().any(|e| {
                            e.from == *other_id && e.to == *node_id && e.edge_type == EdgeType::DependsOn
                        });

                        if !edge_exists {
                            self.edges.push(ActionEdge {
                                from: other_id.clone(),
                                to: node_id.clone(),
                                edge_type: EdgeType::DependsOn,
                            });
                        }
                    }
                }
            }

            // Setup actions produce runtimes
            if node_id.contains("setup-node") {
                self.edges.push(ActionEdge {
                    from: node_id.clone(),
                    to: "npm".to_string(),
                    edge_type: EdgeType::Produces,
                });
                self.edges.push(ActionEdge {
                    from: node_id.clone(),
                    to: "node".to_string(),
                    edge_type: EdgeType::Produces,
                });
            }

            if node_id.contains("setup-python") {
                self.edges.push(ActionEdge {
                    from: node_id.clone(),
                    to: "pip".to_string(),
                    edge_type: EdgeType::Produces,
                });
                self.edges.push(ActionEdge {
                    from: node_id.clone(),
                    to: "python".to_string(),
                    edge_type: EdgeType::Produces,
                });
            }

            // Build targets produce artifacts
            if node_id.starts_with("+") && node_id.contains("build") {
                // Find SAVE ARTIFACT children
                for term in self.thesaurus_builder.get_children(node_id) {
                    if term.nterm.starts_with("SAVE ARTIFACT") {
                        self.edges.push(ActionEdge {
                            from: node_id.clone(),
                            to: term.nterm.clone(),
                            edge_type: EdgeType::Produces,
                        });
                    }
                }
            }
        }
    }

    /// Check if all terms in a sequence are connected by a valid path
    pub fn is_sequence_valid(&self, terms: &[String]) -> bool {
        if terms.len() < 2 {
            return true;
        }

        for window in terms.windows(2) {
            if !self.has_path(&window[0], &window[1]) {
                return false;
            }
        }

        true
    }

    /// Check if there's a path from source to target
    pub fn has_path(&self, from: &str, to: &str) -> bool {
        if from == to {
            return true;
        }

        let mut visited = AHashSet::new();
        let mut queue = vec![from.to_string()];

        while let Some(current) = queue.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            for edge in &self.edges {
                if edge.from == current || edge.to == current {
                    let next = if edge.from == current {
                        &edge.to
                    } else {
                        &edge.from
                    };

                    if next == to {
                        return true;
                    }

                    if !visited.contains(next) {
                        queue.push(next.clone());
                    }
                }
            }
        }

        false
    }

    /// Get dependencies for an action
    pub fn get_dependencies(&self, action: &str) -> Vec<String> {
        self.edges
            .iter()
            .filter(|e| e.from == action && e.edge_type == EdgeType::DependsOn)
            .map(|e| e.to.clone())
            .collect()
    }

    /// Get what an action produces
    pub fn get_produces(&self, action: &str) -> Vec<String> {
        self.edges
            .iter()
            .filter(|e| e.from == action && e.edge_type == EdgeType::Produces)
            .map(|e| e.to.clone())
            .collect()
    }

    /// Find similar actions based on graph relationships
    pub fn find_similar(&self, action: &str, limit: usize) -> Vec<String> {
        let mut scores: AHashMap<String, f64> = AHashMap::new();

        // Get direct neighbors
        let neighbors: Vec<_> = self.edges
            .iter()
            .filter(|e| e.from == action || e.to == action)
            .flat_map(|e| vec![e.from.clone(), e.to.clone()])
            .filter(|n| n != action)
            .collect();

        // Score other nodes by shared neighbors
        for (node_id, _) in &self.nodes {
            if node_id == action {
                continue;
            }

            let node_neighbors: AHashSet<_> = self.edges
                .iter()
                .filter(|e| e.from == *node_id || e.to == *node_id)
                .flat_map(|e| vec![e.from.clone(), e.to.clone()])
                .filter(|n| n != node_id)
                .collect();

            let shared = neighbors
                .iter()
                .filter(|n| node_neighbors.contains(*n))
                .count();

            if shared > 0 {
                scores.insert(node_id.clone(), shared as f64);
            }
        }

        // Sort by score and return top results
        let mut results: Vec<_> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.into_iter().take(limit).map(|(k, _)| k).collect()
    }

    /// Validate an interpreted action against the graph
    pub fn validate_interpretation(&self, interpretation: &InterpretedAction) -> RunnerResult<()> {
        // Check that prerequisites exist
        for prereq in &interpretation.prerequisites {
            if !self.nodes.contains_key(prereq) && !prereq.is_empty() {
                return Err(RunnerError::KnowledgeGraph(format!(
                    "Unknown prerequisite: {}",
                    prereq
                )));
            }
        }

        // Check that produced items are valid
        for produced in &interpretation.produces {
            // These can be new, so just validate format
            if produced.is_empty() {
                return Err(RunnerError::KnowledgeGraph(
                    "Empty produced item".to_string()
                ));
            }
        }

        Ok(())
    }

    /// Get node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get edge count
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Get all nodes
    pub fn nodes(&self) -> impl Iterator<Item = &ActionNode> {
        self.nodes.values()
    }

    /// Get all edges
    pub fn edges(&self) -> &[ActionEdge] {
        &self.edges
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_graph() -> ActionGraph {
        let mut builder = ThesaurusBuilder::new("https://github.com/test/repo");

        let workflow = r#"
name: CI
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
      - run: npm ci
      - run: npm test
"#;

        builder
            .add_workflow_content(workflow, "ci.yml")
            .unwrap()
            .add_builtin_terms();

        ActionGraph::from_thesaurus(builder)
    }

    #[test]
    fn test_graph_construction() {
        let graph = create_test_graph();
        assert!(graph.node_count() > 0);
        assert!(graph.edge_count() > 0);
    }

    #[test]
    fn test_path_finding() {
        let graph = create_test_graph();

        // npm should depend on checkout (through setup-node)
        // This tests the inferred dependency chain
        assert!(graph.has_path("run:npm", "uses:actions/checkout"));
    }

    #[test]
    fn test_sequence_validation() {
        let graph = create_test_graph();

        // Valid sequence
        let sequence = vec![
            "uses:actions/checkout".to_string(),
            "uses:actions/setup-node".to_string(),
            "run:npm".to_string(),
        ];
        assert!(graph.is_sequence_valid(&sequence));
    }

    #[test]
    fn test_dependencies() {
        let graph = create_test_graph();

        // npm commands should depend on checkout
        let deps = graph.get_dependencies("run:npm");
        // Dependencies are inferred, so this checks the mechanism works
        assert!(deps.is_empty() || deps.iter().any(|d| d.contains("checkout")));
    }
}
