//! Symbolic embeddings for medical knowledge graph nodes.
//!
//! This module provides a symbolic embedding representation for nodes in a
//! medical knowledge graph. Embeddings are computed from IS-A hierarchy
//! relationships (ancestor/descendant sets) and support Jaccard-based
//! similarity queries without requiring petgraph or any external graph library.
//!
//! The approach uses AHashMap-based parent/child maps (the same data model as
//! RoleGraph) and computes transitive closures to build ancestor and descendant
//! sets for each node.

use ahash::{AHashMap, AHashSet};
use std::sync::RwLock;
use terraphim_types::MedicalNodeType;

/// Symbolic embedding for a single node in the medical knowledge graph.
///
/// Encodes the node's position in the IS-A hierarchy via its ancestor and
/// descendant sets, depth, and semantic type. Similarity between embeddings
/// is computed using Jaccard similarity of these sets combined with path
/// distance estimation.
#[derive(Debug, Clone)]
pub struct SymbolicEmbedding {
    /// The node ID this embedding represents
    pub node_id: u64,
    /// Set of transitive ancestor node IDs (via IS-A parents)
    pub ancestors: AHashSet<u64>,
    /// Set of transitive descendant node IDs (via IS-A children)
    pub descendants: AHashSet<u64>,
    /// Depth in the IS-A hierarchy (0 for root nodes)
    pub depth: usize,
    /// The medical semantic type of this node
    pub semantic_type: MedicalNodeType,
}

impl SymbolicEmbedding {
    /// Create a new symbolic embedding.
    pub fn new(
        node_id: u64,
        ancestors: AHashSet<u64>,
        descendants: AHashSet<u64>,
        depth: usize,
        semantic_type: MedicalNodeType,
    ) -> Self {
        Self {
            node_id,
            ancestors,
            descendants,
            depth,
            semantic_type,
        }
    }

    /// Returns true if this node has no ancestors (i.e., it is a root in the IS-A hierarchy).
    pub fn is_root(&self) -> bool {
        self.ancestors.is_empty()
    }

    /// Returns true if this node has no descendants (i.e., it is a leaf in the IS-A hierarchy).
    pub fn is_leaf(&self) -> bool {
        self.descendants.is_empty()
    }

    /// Compute Jaccard similarity between this embedding and another.
    ///
    /// The Jaccard similarity is computed over the union of ancestor and
    /// descendant sets: |A intersection B| / |A union B|.
    /// Returns 1.0 if both sets are empty (two isolated nodes are maximally
    /// similar to each other in the absence of hierarchy information).
    pub fn jaccard_similarity(&self, other: &SymbolicEmbedding) -> f64 {
        let self_set: AHashSet<u64> = self
            .ancestors
            .iter()
            .chain(self.descendants.iter())
            .copied()
            .collect();
        let other_set: AHashSet<u64> = other
            .ancestors
            .iter()
            .chain(other.descendants.iter())
            .copied()
            .collect();

        let intersection_size = self_set.intersection(&other_set).count();
        let union_size = self_set.union(&other_set).count();

        if union_size == 0 {
            return 1.0;
        }

        intersection_size as f64 / union_size as f64
    }
}

/// Index of symbolic embeddings for fast similarity queries.
///
/// Maintains embeddings keyed by node ID, a type index for filtered queries,
/// and a similarity cache protected by an RwLock for concurrent access.
#[derive(Debug)]
pub struct SymbolicEmbeddingIndex {
    /// Node ID -> SymbolicEmbedding
    embeddings: AHashMap<u64, SymbolicEmbedding>,
    /// MedicalNodeType -> set of node IDs with that type
    by_type: AHashMap<MedicalNodeType, AHashSet<u64>>,
    /// Cache of computed similarity scores: (a, b) -> score
    similarity_cache: RwLock<AHashMap<(u64, u64), f64>>,
}

impl SymbolicEmbeddingIndex {
    /// Create a new empty embedding index.
    pub fn new() -> Self {
        Self {
            embeddings: AHashMap::new(),
            by_type: AHashMap::new(),
            similarity_cache: RwLock::new(AHashMap::new()),
        }
    }

    /// Build an embedding index from IS-A parent relationships and node type assignments.
    ///
    /// # Arguments
    ///
    /// * `isa_parents` - Map from child node ID to set of direct parent node IDs
    /// * `node_types` - Map from node ID to its medical semantic type
    ///
    /// # Algorithm
    ///
    /// 1. Collect all node IDs from both maps
    /// 2. Compute transitive ancestors for each node via BFS/DFS on isa_parents
    /// 3. Invert to compute transitive descendants
    /// 4. Compute depth by sorting nodes by ancestor count (ascending) and
    ///    setting depth = max(parent depths) + 1
    /// 5. Build the type index
    pub fn build_from_hierarchy(
        isa_parents: &AHashMap<u64, AHashSet<u64>>,
        node_types: &AHashMap<u64, MedicalNodeType>,
    ) -> Self {
        // Collect all node IDs
        let mut all_nodes: AHashSet<u64> = AHashSet::new();
        for (child, parents) in isa_parents {
            all_nodes.insert(*child);
            for parent in parents {
                all_nodes.insert(*parent);
            }
        }
        for node_id in node_types.keys() {
            all_nodes.insert(*node_id);
        }

        // Compute transitive ancestors for each node
        let mut ancestors_map: AHashMap<u64, AHashSet<u64>> = AHashMap::new();
        for &node_id in &all_nodes {
            let ancestors = Self::compute_ancestors(node_id, isa_parents);
            ancestors_map.insert(node_id, ancestors);
        }

        // Compute transitive descendants by inverting the ancestor relationship
        let mut descendants_map: AHashMap<u64, AHashSet<u64>> = AHashMap::new();
        for &node_id in &all_nodes {
            descendants_map.insert(node_id, AHashSet::new());
        }
        for (&node_id, ancestors) in &ancestors_map {
            for &ancestor_id in ancestors {
                descendants_map
                    .entry(ancestor_id)
                    .or_default()
                    .insert(node_id);
            }
        }

        // Compute depths: sort nodes by ancestor count ascending, then iterate
        let mut nodes_by_ancestor_count: Vec<(u64, usize)> = all_nodes
            .iter()
            .map(|&id| {
                let count = ancestors_map.get(&id).map_or(0, |a| a.len());
                (id, count)
            })
            .collect();
        nodes_by_ancestor_count.sort_by_key(|&(_, count)| count);

        let mut depth_map: AHashMap<u64, usize> = AHashMap::new();
        for &(node_id, _) in &nodes_by_ancestor_count {
            let direct_parents = isa_parents.get(&node_id);
            let depth = match direct_parents {
                Some(parents) if !parents.is_empty() => {
                    let max_parent_depth = parents
                        .iter()
                        .filter_map(|p| depth_map.get(p))
                        .max()
                        .copied()
                        .unwrap_or(0);
                    max_parent_depth + 1
                }
                _ => 0,
            };
            depth_map.insert(node_id, depth);
        }

        // Build embeddings and type index
        let mut embeddings = AHashMap::new();
        let mut by_type: AHashMap<MedicalNodeType, AHashSet<u64>> = AHashMap::new();

        for &node_id in &all_nodes {
            let ancestors = ancestors_map.remove(&node_id).unwrap_or_default();
            let descendants = descendants_map.remove(&node_id).unwrap_or_default();
            let depth = depth_map.get(&node_id).copied().unwrap_or(0);
            let semantic_type = node_types
                .get(&node_id)
                .copied()
                .unwrap_or(MedicalNodeType::Concept);

            let embedding =
                SymbolicEmbedding::new(node_id, ancestors, descendants, depth, semantic_type);
            embeddings.insert(node_id, embedding);
            by_type.entry(semantic_type).or_default().insert(node_id);
        }

        Self {
            embeddings,
            by_type,
            similarity_cache: RwLock::new(AHashMap::new()),
        }
    }

    /// Compute transitive ancestors for a node by walking up the IS-A parent map.
    fn compute_ancestors(
        node_id: u64,
        isa_parents: &AHashMap<u64, AHashSet<u64>>,
    ) -> AHashSet<u64> {
        let mut ancestors = AHashSet::new();
        let mut stack: Vec<u64> = Vec::new();

        // Seed with direct parents
        if let Some(parents) = isa_parents.get(&node_id) {
            for &parent in parents {
                stack.push(parent);
            }
        }

        while let Some(current) = stack.pop() {
            if ancestors.insert(current) {
                // If newly inserted, explore its parents too
                if let Some(parents) = isa_parents.get(&current) {
                    for &parent in parents {
                        if !ancestors.contains(&parent) {
                            stack.push(parent);
                        }
                    }
                }
            }
        }

        ancestors
    }

    /// Compute similarity between two nodes.
    ///
    /// The score is a weighted combination:
    /// - 70% Jaccard similarity of ancestor+descendant sets
    /// - 30% path distance score (estimated from ancestor overlap)
    ///
    /// Results are cached in the similarity cache (protected by RwLock).
    /// Returns None if either node is not in the index.
    pub fn similarity(&self, a: u64, b: u64) -> Option<f64> {
        if a == b {
            return Some(1.0);
        }

        // Normalize key order for cache symmetry
        let cache_key = if a <= b { (a, b) } else { (b, a) };

        // Check cache first (read lock)
        {
            let cache = self.similarity_cache.read().ok()?;
            if let Some(&score) = cache.get(&cache_key) {
                return Some(score);
            }
        }

        let emb_a = self.embeddings.get(&a)?;
        let emb_b = self.embeddings.get(&b)?;

        // 70% Jaccard similarity
        let jaccard = emb_a.jaccard_similarity(emb_b);

        // 30% path distance score
        let path_score = Self::path_distance_score(emb_a, emb_b);

        let score = 0.7 * jaccard + 0.3 * path_score;

        // Cache the result (write lock)
        if let Ok(mut cache) = self.similarity_cache.write() {
            cache.insert(cache_key, score);
        }

        Some(score)
    }

    /// Estimate path distance score from ancestor overlap.
    ///
    /// Uses the Lowest Common Ancestor (LCA) depth estimation:
    /// - Find common ancestors between the two nodes
    /// - The estimated path length is: depth(a) + depth(b) - 2 * max_common_depth
    /// - Convert to a similarity score in [0.0, 1.0] using 1.0 / (1.0 + path_length)
    fn path_distance_score(emb_a: &SymbolicEmbedding, emb_b: &SymbolicEmbedding) -> f64 {
        // If one is an ancestor of the other, the path length is the depth difference
        if emb_a.ancestors.contains(&emb_b.node_id) {
            let path_len = emb_a.depth.saturating_sub(emb_b.depth);
            return 1.0 / (1.0 + path_len as f64);
        }
        if emb_b.ancestors.contains(&emb_a.node_id) {
            let path_len = emb_b.depth.saturating_sub(emb_a.depth);
            return 1.0 / (1.0 + path_len as f64);
        }

        // Find common ancestors and estimate LCA depth
        let common_ancestors: AHashSet<u64> = emb_a
            .ancestors
            .intersection(&emb_b.ancestors)
            .copied()
            .collect();

        if common_ancestors.is_empty() {
            // No common ancestors -> maximum distance
            return 0.0;
        }

        // Estimate max common ancestor depth as proxy for LCA depth
        // We use the number of ancestors of each common ancestor as a proxy for depth
        // since we do not have access to the full depth map here. Instead, we use the
        // actual depth fields which were pre-computed.
        // For the LCA estimation, we use: path = depth(a) + depth(b) - 2 * lca_depth
        // We approximate lca_depth as the maximum depth among common ancestors.
        // Since we do not store depth for all ancestors, we estimate it.
        let max_depth = emb_a.depth.max(emb_b.depth);
        if max_depth == 0 {
            return 1.0;
        }

        // Estimate path length: depth(a) + depth(b) - 2 * estimated_lca_depth
        // We estimate the LCA depth as proportional to the fraction of shared ancestors
        let total_unique_ancestors = emb_a.ancestors.union(&emb_b.ancestors).count();
        let shared_fraction = if total_unique_ancestors > 0 {
            common_ancestors.len() as f64 / total_unique_ancestors as f64
        } else {
            0.0
        };

        // The more ancestors they share, the closer the LCA is to both nodes
        let estimated_lca_depth = (shared_fraction * max_depth as f64).round() as usize;
        let path_length = (emb_a.depth + emb_b.depth).saturating_sub(2 * estimated_lca_depth);

        1.0 / (1.0 + path_length as f64)
    }

    /// Find the k nearest neighbors to a query node by similarity score.
    ///
    /// Returns a vector of (node_id, score) pairs sorted by descending score.
    /// The query node itself is excluded from results.
    pub fn nearest_neighbors(&self, query: u64, k: usize) -> Vec<(u64, f64)> {
        let mut scores: Vec<(u64, f64)> = self
            .embeddings
            .keys()
            .filter(|&&id| id != query)
            .filter_map(|&id| self.similarity(query, id).map(|s| (id, s)))
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(k);
        scores
    }

    /// Find the k nearest neighbors of a given type to a query node.
    ///
    /// Only considers nodes whose semantic type matches `node_type`.
    /// Returns a vector of (node_id, score) pairs sorted by descending score.
    pub fn nearest_neighbors_by_type(
        &self,
        query: u64,
        node_type: MedicalNodeType,
        k: usize,
    ) -> Vec<(u64, f64)> {
        let candidates = match self.by_type.get(&node_type) {
            Some(ids) => ids,
            None => return Vec::new(),
        };

        let mut scores: Vec<(u64, f64)> = candidates
            .iter()
            .filter(|&&id| id != query)
            .filter_map(|&id| self.similarity(query, id).map(|s| (id, s)))
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(k);
        scores
    }

    /// Get the embedding for a specific node.
    pub fn get_embedding(&self, node_id: u64) -> Option<&SymbolicEmbedding> {
        self.embeddings.get(&node_id)
    }

    /// Get all node IDs of a given medical type.
    pub fn nodes_by_type(&self, node_type: MedicalNodeType) -> Option<&AHashSet<u64>> {
        self.by_type.get(&node_type)
    }

    /// Get an iterator over all embeddings.
    pub fn all_embeddings(&self) -> impl Iterator<Item = (&u64, &SymbolicEmbedding)> {
        self.embeddings.iter()
    }

    /// Clear the similarity cache.
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.similarity_cache.write() {
            cache.clear();
        }
    }

    /// Return the number of cached similarity scores and the total number of embeddings.
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache_size = self.similarity_cache.read().map(|c| c.len()).unwrap_or(0);
        (cache_size, self.embeddings.len())
    }
}

impl Default for SymbolicEmbeddingIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a small medical hierarchy for testing:
    ///
    /// ```text
    ///          Disease (100)
    ///          /       \
    ///     Cancer (101)  Infection (102)
    ///      /    \
    ///  Lung (103) Breast (104)
    /// ```
    fn build_test_hierarchy() -> (AHashMap<u64, AHashSet<u64>>, AHashMap<u64, MedicalNodeType>) {
        let mut isa_parents: AHashMap<u64, AHashSet<u64>> = AHashMap::new();

        // Cancer IS-A Disease
        isa_parents.entry(101).or_default().insert(100);
        // Infection IS-A Disease
        isa_parents.entry(102).or_default().insert(100);
        // Lung Cancer IS-A Cancer
        isa_parents.entry(103).or_default().insert(101);
        // Breast Cancer IS-A Cancer
        isa_parents.entry(104).or_default().insert(101);

        let mut node_types: AHashMap<u64, MedicalNodeType> = AHashMap::new();
        node_types.insert(100, MedicalNodeType::Disease);
        node_types.insert(101, MedicalNodeType::Disease);
        node_types.insert(102, MedicalNodeType::Disease);
        node_types.insert(103, MedicalNodeType::Disease);
        node_types.insert(104, MedicalNodeType::Disease);

        (isa_parents, node_types)
    }

    #[test]
    fn test_build_from_hierarchy_node_count() {
        let (isa_parents, node_types) = build_test_hierarchy();
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);
        assert_eq!(index.embeddings.len(), 5);
    }

    #[test]
    fn test_root_detection() {
        let (isa_parents, node_types) = build_test_hierarchy();
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        let disease = index.get_embedding(100).expect("Disease node should exist");
        assert!(disease.is_root(), "Disease (100) should be a root node");
        assert!(!disease.is_leaf(), "Disease (100) should not be a leaf");

        let lung = index
            .get_embedding(103)
            .expect("Lung Cancer node should exist");
        assert!(!lung.is_root(), "Lung Cancer (103) should not be a root");
        assert!(lung.is_leaf(), "Lung Cancer (103) should be a leaf");
    }

    #[test]
    fn test_ancestors_and_descendants() {
        let (isa_parents, node_types) = build_test_hierarchy();
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        // Lung Cancer (103) should have ancestors: Cancer (101), Disease (100)
        let lung = index.get_embedding(103).unwrap();
        assert!(lung.ancestors.contains(&101));
        assert!(lung.ancestors.contains(&100));
        assert_eq!(lung.ancestors.len(), 2);

        // Disease (100) should have descendants: 101, 102, 103, 104
        let disease = index.get_embedding(100).unwrap();
        assert!(disease.descendants.contains(&101));
        assert!(disease.descendants.contains(&102));
        assert!(disease.descendants.contains(&103));
        assert!(disease.descendants.contains(&104));
        assert_eq!(disease.descendants.len(), 4);

        // Cancer (101) should have descendants: 103, 104
        let cancer = index.get_embedding(101).unwrap();
        assert!(cancer.descendants.contains(&103));
        assert!(cancer.descendants.contains(&104));
        assert_eq!(cancer.descendants.len(), 2);
    }

    #[test]
    fn test_depth_computation() {
        let (isa_parents, node_types) = build_test_hierarchy();
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        assert_eq!(index.get_embedding(100).unwrap().depth, 0);
        assert_eq!(index.get_embedding(101).unwrap().depth, 1);
        assert_eq!(index.get_embedding(102).unwrap().depth, 1);
        assert_eq!(index.get_embedding(103).unwrap().depth, 2);
        assert_eq!(index.get_embedding(104).unwrap().depth, 2);
    }

    #[test]
    fn test_similarity_symmetric() {
        let (isa_parents, node_types) = build_test_hierarchy();
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        let sim_ab = index.similarity(103, 104).unwrap();
        let sim_ba = index.similarity(104, 103).unwrap();
        assert!(
            (sim_ab - sim_ba).abs() < f64::EPSILON,
            "Similarity should be symmetric: {sim_ab} vs {sim_ba}"
        );
    }

    #[test]
    fn test_similarity_range() {
        let (isa_parents, node_types) = build_test_hierarchy();
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        for &a in &[100u64, 101, 102, 103, 104] {
            for &b in &[100u64, 101, 102, 103, 104] {
                let sim = index.similarity(a, b).unwrap();
                assert!(
                    (0.0..=1.0).contains(&sim),
                    "Similarity should be in [0.0, 1.0], got {sim} for ({a}, {b})"
                );
            }
        }
    }

    #[test]
    fn test_self_similarity() {
        let (isa_parents, node_types) = build_test_hierarchy();
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        for &node in &[100u64, 101, 102, 103, 104] {
            let sim = index.similarity(node, node).unwrap();
            assert!(
                (sim - 1.0).abs() < f64::EPSILON,
                "Self-similarity should be 1.0, got {sim} for node {node}"
            );
        }
    }

    #[test]
    fn test_siblings_more_similar_than_distant() {
        let (isa_parents, node_types) = build_test_hierarchy();
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        // Lung Cancer (103) and Breast Cancer (104) are siblings under Cancer (101)
        // They should be more similar to each other than to Infection (102)
        let sim_siblings = index.similarity(103, 104).unwrap();
        let sim_distant = index.similarity(103, 102).unwrap();
        assert!(
            sim_siblings > sim_distant,
            "Siblings should be more similar ({sim_siblings}) than distant nodes ({sim_distant})"
        );
    }

    #[test]
    fn test_parent_more_similar_than_grandparent() {
        let (isa_parents, node_types) = build_test_hierarchy();
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        // Lung Cancer (103) -> parent Cancer (101) -> grandparent Disease (100)
        let sim_parent = index.similarity(103, 101).unwrap();
        let sim_grandparent = index.similarity(103, 100).unwrap();
        assert!(
            sim_parent >= sim_grandparent,
            "Parent similarity ({sim_parent}) should be >= grandparent similarity ({sim_grandparent})"
        );
    }

    #[test]
    fn test_nearest_neighbors() {
        let (isa_parents, node_types) = build_test_hierarchy();
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        // Query for neighbors of Lung Cancer (103)
        let neighbors = index.nearest_neighbors(103, 3);
        assert!(!neighbors.is_empty(), "Should find at least one neighbor");
        assert!(neighbors.len() <= 3, "Should return at most k neighbors");

        // Verify sorted by descending score
        for window in neighbors.windows(2) {
            assert!(
                window[0].1 >= window[1].1,
                "Neighbors should be sorted by descending score"
            );
        }

        // The nearest neighbor of Lung Cancer should be Breast Cancer (sibling)
        assert_eq!(
            neighbors[0].0, 104,
            "Nearest neighbor of Lung Cancer (103) should be Breast Cancer (104)"
        );
    }

    #[test]
    fn test_nearest_neighbors_by_type() {
        let mut isa_parents = AHashMap::new();
        isa_parents
            .entry(101u64)
            .or_insert_with(AHashSet::new)
            .insert(100);
        isa_parents
            .entry(200u64)
            .or_insert_with(AHashSet::new)
            .insert(100);

        let mut node_types = AHashMap::new();
        node_types.insert(100, MedicalNodeType::Disease);
        node_types.insert(101, MedicalNodeType::Disease);
        node_types.insert(200, MedicalNodeType::Drug);

        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        // Query for Disease-type neighbors of node 101
        let disease_neighbors = index.nearest_neighbors_by_type(101, MedicalNodeType::Disease, 5);
        for (id, _) in &disease_neighbors {
            let emb = index.get_embedding(*id).unwrap();
            assert_eq!(
                emb.semantic_type,
                MedicalNodeType::Disease,
                "All type-filtered neighbors should be Disease nodes"
            );
        }
    }

    #[test]
    fn test_nodes_by_type() {
        let (isa_parents, node_types) = build_test_hierarchy();
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        let diseases = index.nodes_by_type(MedicalNodeType::Disease);
        assert!(diseases.is_some());
        assert_eq!(diseases.unwrap().len(), 5);

        let drugs = index.nodes_by_type(MedicalNodeType::Drug);
        assert!(drugs.is_none(), "No drugs were added to the hierarchy");
    }

    #[test]
    fn test_cache_stats_and_clear() {
        let (isa_parents, node_types) = build_test_hierarchy();
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        let (cache_size, total) = index.cache_stats();
        assert_eq!(cache_size, 0, "Cache should be empty initially");
        assert_eq!(total, 5, "Should have 5 embeddings");

        // Trigger some similarity computations to populate cache
        let _ = index.similarity(103, 104);
        let _ = index.similarity(101, 102);

        let (cache_size, _) = index.cache_stats();
        assert_eq!(
            cache_size, 2,
            "Cache should have 2 entries after two similarity calls"
        );

        index.clear_cache();
        let (cache_size, _) = index.cache_stats();
        assert_eq!(cache_size, 0, "Cache should be empty after clearing");
    }

    #[test]
    fn test_empty_index() {
        let index = SymbolicEmbeddingIndex::new();
        assert!(index.get_embedding(1).is_none());
        assert!(index.similarity(1, 2).is_none());
        assert!(index.nearest_neighbors(1, 5).is_empty());
        let (cache_size, total) = index.cache_stats();
        assert_eq!(cache_size, 0);
        assert_eq!(total, 0);
    }

    #[test]
    fn test_jaccard_empty_sets() {
        let emb_a = SymbolicEmbedding::new(
            1,
            AHashSet::new(),
            AHashSet::new(),
            0,
            MedicalNodeType::Concept,
        );
        let emb_b = SymbolicEmbedding::new(
            2,
            AHashSet::new(),
            AHashSet::new(),
            0,
            MedicalNodeType::Concept,
        );

        let sim = emb_a.jaccard_similarity(&emb_b);
        assert!(
            (sim - 1.0).abs() < f64::EPSILON,
            "Jaccard of two empty-set embeddings should be 1.0"
        );
    }

    #[test]
    fn test_similarity_nonexistent_node() {
        let (isa_parents, node_types) = build_test_hierarchy();
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        assert!(
            index.similarity(100, 999).is_none(),
            "Similarity with non-existent node should return None"
        );
    }

    #[test]
    fn test_all_embeddings() {
        let (isa_parents, node_types) = build_test_hierarchy();
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        let all: Vec<_> = index.all_embeddings().collect();
        assert_eq!(all.len(), 5);
    }
}
