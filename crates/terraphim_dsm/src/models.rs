use serde::{Deserialize, Serialize};

/// Represents a semantic grouping of modules by domain concept
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticGroup {
    /// Domain concept name
    pub concept: String,
    /// Module paths belonging to this concept
    pub modules: Vec<String>,
    /// Number of modules in this group
    pub count: usize,
}

/// Complete semantic analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticAnalysis {
    pub groups: Vec<SemanticGroup>,
    pub total_modules: usize,
    pub uncategorized_count: usize,
    pub knowledge_graph_concepts: usize,
}
