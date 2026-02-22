//! Medical extension for RoleGraph.
//!
//! This module wraps the existing [`RoleGraph`] with medical-specific capabilities
//! including typed nodes/edges, IS-A hierarchy tracking, and symbolic embeddings.
//! It uses composition (not inheritance) so all existing RoleGraph functionality
//! (document indexing, Aho-Corasick search, etc.) remains available through the
//! `role_graph` field.
//!
//! # Design
//!
//! - All medical metadata is stored in separate AHashMaps alongside the RoleGraph,
//!   keeping the core RoleGraph untouched.
//! - Edge IDs are computed using [`magic_pair`] from the crate root, consistent
//!   with how RoleGraph computes edge IDs.
//! - IS-A relationships are tracked in both directions (parents and children) for
//!   efficient ancestor/descendant traversal.
//! - Symbolic embeddings are built on demand from the IS-A hierarchy.

use crate::symbolic_embeddings::SymbolicEmbeddingIndex;
use crate::{RoleGraph, magic_pair};
use ahash::{AHashMap, AHashSet};
use terraphim_types::{MedicalEdgeType, MedicalNodeType, RoleName, Thesaurus};

/// Error type for medical role graph operations.
#[derive(thiserror::Error, Debug)]
pub enum MedicalRoleGraphError {
    /// Propagated error from the underlying RoleGraph.
    #[error("RoleGraph error: {0}")]
    RoleGraphError(#[from] crate::Error),
}

type Result<T> = std::result::Result<T, MedicalRoleGraphError>;

/// Medical extension for RoleGraph.
///
/// Adds typed nodes/edges, IS-A hierarchy, and symbolic embeddings on top of
/// the existing RoleGraph document indexing infrastructure.
pub struct MedicalRoleGraph {
    /// The underlying RoleGraph (document indexing + search via Aho-Corasick).
    pub role_graph: RoleGraph,
    /// Node ID -> medical type
    node_types: AHashMap<u64, MedicalNodeType>,
    /// Node ID -> display term
    node_terms: AHashMap<u64, String>,
    /// Edge ID (magic_pair) -> medical edge type
    edge_types: AHashMap<u64, MedicalEdgeType>,
    /// Source node -> list of (target, edge_type) for outgoing edges
    outgoing_edges: AHashMap<u64, Vec<(u64, MedicalEdgeType)>>,
    /// Target node -> list of (source, edge_type) for incoming edges
    incoming_edges: AHashMap<u64, Vec<(u64, MedicalEdgeType)>>,
    /// IS-A parent map: child_id -> set of parent_ids
    isa_parents: AHashMap<u64, AHashSet<u64>>,
    /// IS-A child map: parent_id -> set of child_ids
    isa_children: AHashMap<u64, AHashSet<u64>>,
    /// SNOMED ID -> node ID
    snomed_to_id: AHashMap<u64, u64>,
    /// Symbolic embedding index (built on demand)
    embedding_index: Option<SymbolicEmbeddingIndex>,
}

impl MedicalRoleGraph {
    /// Create a new MedicalRoleGraph wrapping a fresh RoleGraph.
    ///
    /// The medical-specific data structures start empty. Use
    /// [`add_medical_node`] and [`add_medical_edge`] to populate them.
    pub async fn new(role: RoleName, thesaurus: Thesaurus) -> Result<Self> {
        let role_graph = RoleGraph::new(role, thesaurus).await?;
        Ok(Self {
            role_graph,
            node_types: AHashMap::new(),
            node_terms: AHashMap::new(),
            edge_types: AHashMap::new(),
            outgoing_edges: AHashMap::new(),
            incoming_edges: AHashMap::new(),
            isa_parents: AHashMap::new(),
            isa_children: AHashMap::new(),
            snomed_to_id: AHashMap::new(),
            embedding_index: None,
        })
    }

    /// Create a new empty MedicalRoleGraph without requiring a Thesaurus.
    ///
    /// Use this when populating the graph via data loaders (SNOMED, PrimeKG)
    /// that add nodes and edges directly rather than relying on Aho-Corasick
    /// thesaurus matching.
    pub fn new_empty() -> Result<Self> {
        let empty_thesaurus = Thesaurus::new("empty".to_string());
        let role_graph = RoleGraph::new_sync("empty".into(), empty_thesaurus)?;
        Ok(Self {
            role_graph,
            node_types: AHashMap::new(),
            node_terms: AHashMap::new(),
            edge_types: AHashMap::new(),
            outgoing_edges: AHashMap::new(),
            incoming_edges: AHashMap::new(),
            isa_parents: AHashMap::new(),
            isa_children: AHashMap::new(),
            snomed_to_id: AHashMap::new(),
            embedding_index: None,
        })
    }

    /// Register a medical node with its type, display term, and optional SNOMED ID.
    ///
    /// This does not create a node in the underlying RoleGraph (which is driven
    /// by document indexing). It registers the medical metadata for this node ID
    /// so that typed queries, hierarchy traversal, and embeddings can use it.
    pub fn add_medical_node(
        &mut self,
        id: u64,
        term: String,
        node_type: MedicalNodeType,
        snomed_id: Option<u64>,
    ) {
        self.node_types.insert(id, node_type);
        self.node_terms.insert(id, term);
        if let Some(sid) = snomed_id {
            self.snomed_to_id.insert(sid, id);
        }
        // Invalidate embeddings since the node set changed
        self.embedding_index = None;
    }

    /// Register a typed medical edge between two nodes.
    ///
    /// Uses [`magic_pair`] to compute the edge ID, consistent with RoleGraph.
    /// If the edge type is [`MedicalEdgeType::IsA`], the IS-A parent/child maps
    /// are updated accordingly (source IS-A target, so target is a parent of source).
    pub fn add_medical_edge(&mut self, source: u64, target: u64, edge_type: MedicalEdgeType) {
        let edge_id = magic_pair(source, target);
        self.edge_types.insert(edge_id, edge_type);

        // Populate adjacency index for O(degree) lookups
        self.outgoing_edges
            .entry(source)
            .or_default()
            .push((target, edge_type));
        self.incoming_edges
            .entry(target)
            .or_default()
            .push((source, edge_type));

        if edge_type == MedicalEdgeType::IsA {
            // source IS-A target means target is a parent of source
            self.isa_parents.entry(source).or_default().insert(target);
            self.isa_children.entry(target).or_default().insert(source);
            // Invalidate embeddings since the hierarchy changed
            self.embedding_index = None;
        }
    }

    /// Get the medical type of a node.
    pub fn get_node_type(&self, id: u64) -> Option<MedicalNodeType> {
        self.node_types.get(&id).copied()
    }

    /// Get the display term for a node.
    pub fn get_node_term(&self, id: u64) -> Option<&str> {
        self.node_terms.get(&id).map(|s| s.as_str())
    }

    /// Get the medical edge type between two nodes.
    pub fn get_edge_type(&self, source: u64, target: u64) -> Option<MedicalEdgeType> {
        let edge_id = magic_pair(source, target);
        self.edge_types.get(&edge_id).copied()
    }

    /// Get all transitive ancestors of a node via IS-A relationships.
    ///
    /// Walks up the IS-A parent map transitively. Returns an empty vec if
    /// the node has no IS-A parents.
    pub fn get_ancestors(&self, node_id: u64) -> Vec<u64> {
        let mut ancestors = AHashSet::new();
        let mut stack = Vec::new();

        if let Some(parents) = self.isa_parents.get(&node_id) {
            for &parent in parents {
                stack.push(parent);
            }
        }

        while let Some(current) = stack.pop() {
            if ancestors.insert(current) {
                if let Some(parents) = self.isa_parents.get(&current) {
                    for &parent in parents {
                        if !ancestors.contains(&parent) {
                            stack.push(parent);
                        }
                    }
                }
            }
        }

        ancestors.into_iter().collect()
    }

    /// Get all transitive descendants of a node via IS-A relationships.
    ///
    /// Walks down the IS-A child map transitively. Returns an empty vec if
    /// the node has no IS-A children.
    pub fn get_descendants(&self, node_id: u64) -> Vec<u64> {
        let mut descendants = AHashSet::new();
        let mut stack = Vec::new();

        if let Some(children) = self.isa_children.get(&node_id) {
            for &child in children {
                stack.push(child);
            }
        }

        while let Some(current) = stack.pop() {
            if descendants.insert(current) {
                if let Some(children) = self.isa_children.get(&current) {
                    for &child in children {
                        if !descendants.contains(&child) {
                            stack.push(child);
                        }
                    }
                }
            }
        }

        descendants.into_iter().collect()
    }

    /// Find all nodes connected to `condition_id` via a Treats edge.
    ///
    /// Uses the adjacency index for O(degree) lookups instead of scanning
    /// all edges. Checks both outgoing and incoming Treats edges since the
    /// relationship can be stored in either direction.
    pub fn get_treatments(&self, condition_id: u64) -> Vec<u64> {
        let mut treatments = Vec::new();
        // Outgoing: condition_id -> target via Treats
        if let Some(edges) = self.outgoing_edges.get(&condition_id) {
            for &(target, edge_type) in edges {
                if edge_type == MedicalEdgeType::Treats {
                    treatments.push(target);
                }
            }
        }
        // Incoming: source -> condition_id via Treats (drug treats condition)
        if let Some(edges) = self.incoming_edges.get(&condition_id) {
            for &(source, edge_type) in edges {
                if edge_type == MedicalEdgeType::Treats {
                    treatments.push(source);
                }
            }
        }
        treatments
    }

    /// Check if a drug has contraindications with any of the given conditions.
    ///
    /// Uses the adjacency index for O(degree) lookups instead of computing
    /// magic_pair. Returns a list of (drug_id, condition_id) pairs for each
    /// Contraindicates edge found.
    pub fn check_contraindication(&self, drug_id: u64, conditions: &[u64]) -> Vec<(u64, u64)> {
        let mut contraindications = Vec::new();
        for &condition_id in conditions {
            let is_contraindicated =
                // drug -> condition
                self.outgoing_edges.get(&drug_id).map_or(false, |edges| {
                    edges
                        .iter()
                        .any(|&(t, et)| t == condition_id && et == MedicalEdgeType::Contraindicates)
                })
                ||
                // condition -> drug
                self.outgoing_edges.get(&condition_id).map_or(false, |edges| {
                    edges
                        .iter()
                        .any(|&(t, et)| t == drug_id && et == MedicalEdgeType::Contraindicates)
                });
            if is_contraindicated {
                contraindications.push((drug_id, condition_id));
            }
        }
        contraindications
    }

    /// Build symbolic embeddings from the current IS-A hierarchy and node types.
    ///
    /// This must be called explicitly after adding nodes and edges. The index
    /// is cached and invalidated whenever nodes or IS-A edges are added.
    pub fn build_embeddings(&mut self) {
        let index =
            SymbolicEmbeddingIndex::build_from_hierarchy(&self.isa_parents, &self.node_types);
        self.embedding_index = Some(index);
    }

    /// Compute symbolic similarity between two nodes.
    ///
    /// Returns None if embeddings have not been built or if either node is
    /// not in the embedding index.
    pub fn symbolic_similarity(&self, a: u64, b: u64) -> Option<f64> {
        self.embedding_index.as_ref()?.similarity(a, b)
    }

    /// Find the k most similar nodes to the given node.
    ///
    /// Returns an empty vec if embeddings have not been built.
    pub fn find_similar(&self, node_id: u64, k: usize) -> Vec<(u64, f64)> {
        match &self.embedding_index {
            Some(index) => index.nearest_neighbors(node_id, k),
            None => Vec::new(),
        }
    }

    /// Get the number of registered medical nodes.
    pub fn node_count(&self) -> usize {
        self.node_types.len()
    }

    /// Get the number of registered medical edges (all types).
    pub fn medical_edge_count(&self) -> usize {
        self.edge_types.len()
    }

    /// Get the number of IS-A edges (counted as parent-child pairs).
    pub fn isa_edge_count(&self) -> usize {
        self.isa_parents.values().map(|s| s.len()).sum()
    }

    /// Resolve a SNOMED CT concept ID to the internal node ID.
    pub fn snomed_to_node_id(&self, snomed_id: u64) -> Option<u64> {
        self.snomed_to_id.get(&snomed_id).copied()
    }

    /// Get a reference to the embedding index, if built.
    pub fn embedding_index(&self) -> Option<&SymbolicEmbeddingIndex> {
        self.embedding_index.as_ref()
    }

    /// Iterate over all registered medical node IDs and their display terms.
    ///
    /// Useful for search operations that need to match query terms against
    /// all nodes in the graph.
    pub fn iter_node_terms(&self) -> impl Iterator<Item = (u64, &str)> {
        self.node_terms
            .iter()
            .map(|(&id, term)| (id, term.as_str()))
    }
}

impl std::fmt::Debug for MedicalRoleGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MedicalRoleGraph")
            .field("node_count", &self.node_types.len())
            .field("edge_count", &self.edge_types.len())
            .field("isa_edge_count", &self.isa_edge_count())
            .field("snomed_mappings", &self.snomed_to_id.len())
            .field("embeddings_built", &self.embedding_index.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::magic_pair;
    use terraphim_types::Thesaurus;

    async fn create_test_medical_rolegraph() -> MedicalRoleGraph {
        let role = "medical test".to_string();
        let thesaurus = Thesaurus::new("empty".to_string());
        MedicalRoleGraph::new(role.into(), thesaurus)
            .await
            .expect("Failed to create MedicalRoleGraph")
    }

    /// Build a test hierarchy:
    ///
    /// ```text
    ///          Disease (100)
    ///          /       \
    ///     Cancer (101)  Infection (102)
    ///      /    \
    ///  Lung (103) Breast (104)
    /// ```
    ///
    /// Plus some drug and treatment edges.
    async fn create_populated_medical_rolegraph() -> MedicalRoleGraph {
        let mut mrg = create_test_medical_rolegraph().await;

        // Add nodes
        mrg.add_medical_node(
            100,
            "Disease".to_string(),
            MedicalNodeType::Disease,
            Some(64572001),
        );
        mrg.add_medical_node(
            101,
            "Cancer".to_string(),
            MedicalNodeType::Disease,
            Some(363346000),
        );
        mrg.add_medical_node(
            102,
            "Infection".to_string(),
            MedicalNodeType::Disease,
            Some(40733004),
        );
        mrg.add_medical_node(
            103,
            "Lung Cancer".to_string(),
            MedicalNodeType::Disease,
            Some(93880001),
        );
        mrg.add_medical_node(
            104,
            "Breast Cancer".to_string(),
            MedicalNodeType::Disease,
            Some(254837009),
        );

        // Add drugs
        mrg.add_medical_node(200, "Cisplatin".to_string(), MedicalNodeType::Drug, None);
        mrg.add_medical_node(201, "Tamoxifen".to_string(), MedicalNodeType::Drug, None);
        mrg.add_medical_node(202, "Aspirin".to_string(), MedicalNodeType::Drug, None);

        // Add IS-A edges
        mrg.add_medical_edge(101, 100, MedicalEdgeType::IsA); // Cancer IS-A Disease
        mrg.add_medical_edge(102, 100, MedicalEdgeType::IsA); // Infection IS-A Disease
        mrg.add_medical_edge(103, 101, MedicalEdgeType::IsA); // Lung Cancer IS-A Cancer
        mrg.add_medical_edge(104, 101, MedicalEdgeType::IsA); // Breast Cancer IS-A Cancer

        // Add treatment edges
        mrg.add_medical_edge(200, 103, MedicalEdgeType::Treats); // Cisplatin treats Lung Cancer
        mrg.add_medical_edge(201, 104, MedicalEdgeType::Treats); // Tamoxifen treats Breast Cancer

        // Add contraindication
        mrg.add_medical_edge(202, 103, MedicalEdgeType::Contraindicates); // Aspirin contraindicates Lung Cancer

        mrg
    }

    #[tokio::test]
    async fn test_create_medical_rolegraph() {
        let mrg = create_test_medical_rolegraph().await;
        assert_eq!(mrg.node_count(), 0);
        assert_eq!(mrg.medical_edge_count(), 0);
        assert_eq!(mrg.isa_edge_count(), 0);
    }

    #[tokio::test]
    async fn test_add_medical_node() {
        let mut mrg = create_test_medical_rolegraph().await;
        mrg.add_medical_node(
            1,
            "Diabetes".to_string(),
            MedicalNodeType::Disease,
            Some(73211009),
        );

        assert_eq!(mrg.node_count(), 1);
        assert_eq!(mrg.get_node_type(1), Some(MedicalNodeType::Disease));
        assert_eq!(mrg.get_node_term(1), Some("Diabetes"));
        assert_eq!(mrg.snomed_to_node_id(73211009), Some(1));
    }

    #[tokio::test]
    async fn test_add_medical_node_no_snomed() {
        let mut mrg = create_test_medical_rolegraph().await;
        mrg.add_medical_node(1, "Unknown Drug".to_string(), MedicalNodeType::Drug, None);

        assert_eq!(mrg.get_node_type(1), Some(MedicalNodeType::Drug));
        assert_eq!(mrg.get_node_term(1), Some("Unknown Drug"));
    }

    #[tokio::test]
    async fn test_add_medical_edge() {
        let mut mrg = create_test_medical_rolegraph().await;
        mrg.add_medical_node(1, "Drug A".to_string(), MedicalNodeType::Drug, None);
        mrg.add_medical_node(2, "Disease B".to_string(), MedicalNodeType::Disease, None);
        mrg.add_medical_edge(1, 2, MedicalEdgeType::Treats);

        assert_eq!(mrg.medical_edge_count(), 1);
        assert_eq!(mrg.get_edge_type(1, 2), Some(MedicalEdgeType::Treats));
    }

    #[tokio::test]
    async fn test_magic_pair_edge_encoding() {
        // Verify that magic_pair is used for edge IDs
        let edge_id = magic_pair(100, 200);
        let mut mrg = create_test_medical_rolegraph().await;
        mrg.add_medical_node(100, "Node A".to_string(), MedicalNodeType::Concept, None);
        mrg.add_medical_node(200, "Node B".to_string(), MedicalNodeType::Concept, None);
        mrg.add_medical_edge(100, 200, MedicalEdgeType::RelatedTo);

        assert_eq!(
            mrg.get_edge_type(100, 200),
            Some(MedicalEdgeType::RelatedTo)
        );
        // The edge should be stored under magic_pair(100, 200)
        assert!(mrg.edge_types.contains_key(&edge_id));
    }

    #[tokio::test]
    async fn test_isa_hierarchy() {
        let mrg = create_populated_medical_rolegraph().await;

        // Verify IS-A edge count: 4 IS-A edges
        assert_eq!(mrg.isa_edge_count(), 4);

        // Test ancestors
        let lung_ancestors = mrg.get_ancestors(103);
        assert!(
            lung_ancestors.contains(&101),
            "Lung Cancer should have Cancer as ancestor"
        );
        assert!(
            lung_ancestors.contains(&100),
            "Lung Cancer should have Disease as ancestor"
        );
        assert_eq!(lung_ancestors.len(), 2);

        // Test descendants
        let disease_descendants = mrg.get_descendants(100);
        assert!(disease_descendants.contains(&101));
        assert!(disease_descendants.contains(&102));
        assert!(disease_descendants.contains(&103));
        assert!(disease_descendants.contains(&104));
        assert_eq!(disease_descendants.len(), 4);
    }

    #[tokio::test]
    async fn test_ancestors_empty_for_root() {
        let mrg = create_populated_medical_rolegraph().await;
        let root_ancestors = mrg.get_ancestors(100);
        assert!(
            root_ancestors.is_empty(),
            "Root node (Disease) should have no ancestors"
        );
    }

    #[tokio::test]
    async fn test_descendants_empty_for_leaf() {
        let mrg = create_populated_medical_rolegraph().await;
        let leaf_descendants = mrg.get_descendants(103);
        assert!(
            leaf_descendants.is_empty(),
            "Leaf node (Lung Cancer) should have no descendants"
        );
    }

    #[tokio::test]
    async fn test_get_treatments() {
        let mrg = create_populated_medical_rolegraph().await;

        let lung_treatments = mrg.get_treatments(103);
        assert_eq!(lung_treatments.len(), 1);
        assert!(
            lung_treatments.contains(&200),
            "Cisplatin (200) should treat Lung Cancer (103)"
        );

        let breast_treatments = mrg.get_treatments(104);
        assert_eq!(breast_treatments.len(), 1);
        assert!(
            breast_treatments.contains(&201),
            "Tamoxifen (201) should treat Breast Cancer (104)"
        );

        // Disease (100) has no direct treatments
        let disease_treatments = mrg.get_treatments(100);
        assert!(disease_treatments.is_empty());
    }

    #[tokio::test]
    async fn test_check_contraindication() {
        let mrg = create_populated_medical_rolegraph().await;

        // Aspirin (202) is contraindicated for Lung Cancer (103)
        let contras = mrg.check_contraindication(202, &[103, 104]);
        assert_eq!(contras.len(), 1);
        assert_eq!(contras[0], (202, 103));

        // Cisplatin (200) has no contraindications with any condition
        let no_contras = mrg.check_contraindication(200, &[103, 104]);
        assert!(no_contras.is_empty());
    }

    #[tokio::test]
    async fn test_build_embeddings() {
        let mut mrg = create_populated_medical_rolegraph().await;
        assert!(
            mrg.embedding_index().is_none(),
            "Embeddings should not be built initially"
        );

        mrg.build_embeddings();
        assert!(
            mrg.embedding_index().is_some(),
            "Embeddings should be built after build_embeddings()"
        );
    }

    #[tokio::test]
    async fn test_symbolic_similarity() {
        let mut mrg = create_populated_medical_rolegraph().await;
        mrg.build_embeddings();

        // Self-similarity
        let self_sim = mrg.symbolic_similarity(103, 103);
        assert_eq!(self_sim, Some(1.0));

        // Siblings should be more similar than distant nodes
        let sibling_sim = mrg.symbolic_similarity(103, 104).unwrap();
        let distant_sim = mrg.symbolic_similarity(103, 102).unwrap();
        assert!(
            sibling_sim > distant_sim,
            "Siblings (Lung Cancer/Breast Cancer) should be more similar ({sibling_sim}) than distant nodes ({distant_sim})"
        );
    }

    #[tokio::test]
    async fn test_symbolic_similarity_without_embeddings() {
        let mrg = create_populated_medical_rolegraph().await;
        assert!(
            mrg.symbolic_similarity(103, 104).is_none(),
            "Similarity should return None when embeddings are not built"
        );
    }

    #[tokio::test]
    async fn test_find_similar() {
        let mut mrg = create_populated_medical_rolegraph().await;
        mrg.build_embeddings();

        let similar = mrg.find_similar(103, 3);
        assert!(!similar.is_empty(), "Should find similar nodes");

        // Verify scores are in descending order
        for window in similar.windows(2) {
            assert!(window[0].1 >= window[1].1);
        }
    }

    #[tokio::test]
    async fn test_find_similar_without_embeddings() {
        let mrg = create_populated_medical_rolegraph().await;
        let similar = mrg.find_similar(103, 3);
        assert!(
            similar.is_empty(),
            "find_similar should return empty vec without embeddings"
        );
    }

    #[tokio::test]
    async fn test_embedding_invalidation_on_node_add() {
        let mut mrg = create_populated_medical_rolegraph().await;
        mrg.build_embeddings();
        assert!(mrg.embedding_index().is_some());

        // Adding a new node should invalidate embeddings
        mrg.add_medical_node(300, "New Node".to_string(), MedicalNodeType::Concept, None);
        assert!(
            mrg.embedding_index().is_none(),
            "Embeddings should be invalidated after adding a node"
        );
    }

    #[tokio::test]
    async fn test_embedding_invalidation_on_isa_edge_add() {
        let mut mrg = create_populated_medical_rolegraph().await;
        mrg.build_embeddings();
        assert!(mrg.embedding_index().is_some());

        // Adding an IS-A edge should invalidate embeddings
        mrg.add_medical_edge(102, 101, MedicalEdgeType::IsA);
        assert!(
            mrg.embedding_index().is_none(),
            "Embeddings should be invalidated after adding IS-A edge"
        );
    }

    #[tokio::test]
    async fn test_non_isa_edge_does_not_invalidate_embeddings() {
        let mut mrg = create_populated_medical_rolegraph().await;
        mrg.build_embeddings();
        assert!(mrg.embedding_index().is_some());

        // Adding a non-IS-A edge should NOT invalidate embeddings
        mrg.add_medical_edge(200, 102, MedicalEdgeType::Treats);
        assert!(
            mrg.embedding_index().is_some(),
            "Non-IS-A edges should not invalidate embeddings"
        );
    }

    #[tokio::test]
    async fn test_node_counts() {
        let mrg = create_populated_medical_rolegraph().await;
        // 5 diseases + 3 drugs = 8 nodes
        assert_eq!(mrg.node_count(), 8);
        // 4 IS-A + 2 Treats + 1 Contraindicates = 7 edges
        assert_eq!(mrg.medical_edge_count(), 7);
        // 4 IS-A edges
        assert_eq!(mrg.isa_edge_count(), 4);
    }

    #[tokio::test]
    async fn test_debug_output() {
        let mrg = create_populated_medical_rolegraph().await;
        let debug = format!("{:?}", mrg);
        assert!(debug.contains("MedicalRoleGraph"));
        assert!(debug.contains("node_count"));
        assert!(debug.contains("edge_count"));
    }

    #[tokio::test]
    async fn test_get_nonexistent_node() {
        let mrg = create_test_medical_rolegraph().await;
        assert!(mrg.get_node_type(999).is_none());
        assert!(mrg.get_node_term(999).is_none());
        assert!(mrg.snomed_to_node_id(999).is_none());
    }

    #[tokio::test]
    async fn test_get_nonexistent_edge() {
        let mrg = create_test_medical_rolegraph().await;
        assert!(mrg.get_edge_type(1, 2).is_none());
    }

    #[tokio::test]
    async fn test_ancestors_nonexistent_node() {
        let mrg = create_populated_medical_rolegraph().await;
        let ancestors = mrg.get_ancestors(999);
        assert!(ancestors.is_empty());
    }

    #[tokio::test]
    async fn test_descendants_nonexistent_node() {
        let mrg = create_populated_medical_rolegraph().await;
        let descendants = mrg.get_descendants(999);
        assert!(descendants.is_empty());
    }

    #[tokio::test]
    async fn test_multiple_snomed_mappings() {
        let mut mrg = create_test_medical_rolegraph().await;
        mrg.add_medical_node(
            1,
            "Disease A".to_string(),
            MedicalNodeType::Disease,
            Some(100001),
        );
        mrg.add_medical_node(
            2,
            "Disease B".to_string(),
            MedicalNodeType::Disease,
            Some(100002),
        );
        mrg.add_medical_node(
            3,
            "Disease C".to_string(),
            MedicalNodeType::Disease,
            Some(100003),
        );

        assert_eq!(mrg.snomed_to_node_id(100001), Some(1));
        assert_eq!(mrg.snomed_to_node_id(100002), Some(2));
        assert_eq!(mrg.snomed_to_node_id(100003), Some(3));
        assert_eq!(mrg.snomed_to_node_id(999999), None);
    }

    #[tokio::test]
    async fn test_role_graph_accessible() {
        let mrg = create_test_medical_rolegraph().await;
        // Verify the underlying RoleGraph is accessible
        assert_eq!(mrg.role_graph.get_node_count(), 0);
        assert_eq!(mrg.role_graph.get_edge_count(), 0);
    }
}
