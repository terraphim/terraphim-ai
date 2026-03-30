//! Proptest axiom tests for symbolic embeddings
//!
//! This module contains property-based tests that verify the mathematical
//! axioms of the symbolic embedding similarity function on random DAGs.

#![cfg(feature = "medical")]

use ahash::{AHashMap, AHashSet};
use proptest::collection::vec as prop_vec;
use proptest::prelude::*;
use proptest::test_runner::TestRunner;
use terraphim_rolegraph::symbolic_embeddings::SymbolicEmbeddingIndex;
use terraphim_types::MedicalNodeType;

/// Generate a random MedicalNodeType
fn medical_node_type_strategy() -> impl Strategy<Value = MedicalNodeType> {
    prop_oneof![
        Just(MedicalNodeType::Concept),
        Just(MedicalNodeType::Disease),
        Just(MedicalNodeType::Drug),
        Just(MedicalNodeType::Gene),
        Just(MedicalNodeType::Variant),
        Just(MedicalNodeType::Procedure),
        Just(MedicalNodeType::Treatment),
        Just(MedicalNodeType::Symptom),
        Just(MedicalNodeType::Protein),
        Just(MedicalNodeType::BiologicalProcess),
        Just(MedicalNodeType::MolecularFunction),
        Just(MedicalNodeType::CellularComponent),
        Just(MedicalNodeType::Pathway),
        Just(MedicalNodeType::Anatomy),
        Just(MedicalNodeType::Phenotype),
        Just(MedicalNodeType::Enzyme),
        Just(MedicalNodeType::Transporter),
        Just(MedicalNodeType::Carrier),
        Just(MedicalNodeType::Target),
        Just(MedicalNodeType::Metabolite),
        Just(MedicalNodeType::Exposure),
        Just(MedicalNodeType::Chemical),
        Just(MedicalNodeType::SideEffect),
        Just(MedicalNodeType::Tissue),
        Just(MedicalNodeType::CellType),
        Just(MedicalNodeType::Organism),
        Just(MedicalNodeType::OntologyTerm),
    ]
}

/// Generate a random DAG with specified node count
fn dag_strategy(
    node_count: usize,
) -> impl Strategy<Value = (AHashMap<u64, AHashSet<u64>>, AHashMap<u64, MedicalNodeType>)> {
    let node_types_strategy: BoxedStrategy<Vec<MedicalNodeType>> =
        prop_vec(medical_node_type_strategy(), node_count).boxed();

    node_types_strategy.prop_map(move |node_types: Vec<MedicalNodeType>| {
        let mut isa_parents: AHashMap<u64, AHashSet<u64>> = AHashMap::new();
        let mut node_types_map: AHashMap<u64, MedicalNodeType> = AHashMap::new();

        for (i, node_type) in node_types.iter().enumerate().take(node_count) {
            node_types_map.insert(i as u64, *node_type);
        }

        for i in 1..node_count {
            let num_parents = if i == 1 {
                1
            } else {
                let max_parents = (i - 1).min(3);
                1 + (i % max_parents)
            };

            let mut parents = AHashSet::new();
            for j in 0..num_parents {
                let parent_id = ((i - 1 - j) % i) as u64;
                parents.insert(parent_id);
            }

            if !parents.is_empty() {
                isa_parents.insert(i as u64, parents);
            }
        }

        (isa_parents, node_types_map)
    })
}

fn random_dag_strategy()
-> impl Strategy<Value = (AHashMap<u64, AHashSet<u64>>, AHashMap<u64, MedicalNodeType>)> {
    (5usize..=100).prop_flat_map(|node_count| dag_strategy(node_count))
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_identity((isa_parents, node_types) in random_dag_strategy()) {
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        for (&node_id, _) in &node_types {
            let sim = index.similarity(node_id, node_id);
            prop_assert!(sim.is_some(), "Self-similarity should be Some");
            prop_assert!(
                (sim.unwrap() - 1.0).abs() < f64::EPSILON,
                "Self-similarity should be 1.0, got {:?} for node {}",
                sim,
                node_id
            );
        }
    }

    #[test]
    fn prop_symmetry((isa_parents, node_types) in random_dag_strategy()) {
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);
        let node_ids: Vec<u64> = node_types.keys().copied().collect();

        for (i, &a) in node_ids.iter().enumerate() {
            for &b in &node_ids[i..] {
                let sim_ab = index.similarity(a, b);
                let sim_ba = index.similarity(b, a);

                prop_assert_eq!(
                    sim_ab, sim_ba,
                    "Similarity should be symmetric: sim({}, {}) = {:?}, sim({}, {}) = {:?}",
                    a, b, sim_ab, b, a, sim_ba
                );
            }
        }
    }

    #[test]
    fn prop_boundedness((isa_parents, node_types) in random_dag_strategy()) {
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);
        let node_ids: Vec<u64> = node_types.keys().copied().collect();

        for &a in &node_ids {
            for &b in &node_ids {
                if let Some(sim) = index.similarity(a, b) {
                    prop_assert!(
                        (0.0..=1.0).contains(&sim),
                        "Similarity should be in [0.0, 1.0], got {} for ({}, {})",
                        sim, a, b
                    );
                }
            }
        }
    }

    #[test]
    fn prop_monotonicity((isa_parents, node_types) in random_dag_strategy()) {
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        for (&child, parents) in &isa_parents {
            for &parent in parents {
                if let Some(grandparents) = isa_parents.get(&parent) {
                    for &grandparent in grandparents {
                        let sim_parent = index.similarity(child, parent);
                        let sim_grandparent = index.similarity(child, grandparent);

                        if let (Some(sp), Some(sg)) = (sim_parent, sim_grandparent) {
                            prop_assert!(
                                sp >= sg || (sp - sg).abs() < f64::EPSILON,
                                "Monotonicity violated: sim(child={}, parent={}) = {} < sim(child={}, grandparent={}) = {}",
                                child, parent, sp, child, grandparent, sg
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn prop_subsumption_coherence((isa_parents, node_types) in random_dag_strategy()) {
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        for (&child, parents) in &isa_parents {
            let child_emb = index.get_embedding(child).expect("Child embedding should exist");

            for &parent in parents {
                let parent_emb = index.get_embedding(parent).expect("Parent embedding should exist");

                prop_assert!(
                    child_emb.ancestors.contains(&parent),
                    "Child {} ancestors should contain parent {}",
                    child, parent
                );

                for &ancestor in &parent_emb.ancestors {
                    prop_assert!(
                        child_emb.ancestors.contains(&ancestor),
                        "Child {} ancestors should contain parent's ancestor {}",
                        child, ancestor
                    );
                }
            }
        }
    }

    #[test]
    fn prop_depth_invariance((isa_parents, node_types) in random_dag_strategy()) {
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        let mut nodes_by_parents: AHashMap<Vec<u64>, Vec<u64>> = AHashMap::new();
        for (&node, parents) in &isa_parents {
            let mut parent_vec: Vec<u64> = parents.iter().copied().collect();
            parent_vec.sort();
            nodes_by_parents.entry(parent_vec).or_default().push(node);
        }

        for (parents, nodes) in nodes_by_parents {
            if nodes.len() >= 2 && !parents.is_empty() {
                for &parent in &parents {
                    let sim_0 = index.similarity(nodes[0], parent);

                    for &sibling in &nodes[1..] {
                        let sim_1 = index.similarity(sibling, parent);

                        prop_assert_eq!(
                            sim_0, sim_1,
                            "Siblings should have equal similarity to shared parent {}: sim({}, {}) = {:?}, sim({}, {}) = {:?}",
                            parent, nodes[0], parent, sim_0, sibling, parent, sim_1
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn prop_idempotent_cache((isa_parents, node_types) in random_dag_strategy()) {
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);
        let node_ids: Vec<u64> = node_types.keys().copied().collect();

        for (i, &a) in node_ids.iter().enumerate().take(10.min(node_ids.len())) {
            for &b in &node_ids[i..10.min(node_ids.len())] {
                let sim1 = index.similarity(a, b);
                let sim2 = index.similarity(a, b);

                prop_assert_eq!(
                    sim1, sim2,
                    "Cache should be idempotent: sim({}, {}) returned {:?} then {:?}",
                    a, b, sim1, sim2
                );
            }
        }

        let (cache_size, _) = index.cache_stats();
        prop_assert!(
            cache_size > 0,
            "Cache should be populated after similarity calls"
        );
    }

    #[test]
    fn prop_triangle_coherence((isa_parents, node_types) in random_dag_strategy()) {
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);
        let node_ids: Vec<u64> = node_types.keys().copied().collect();
        let threshold_high = 0.8;
        let threshold_low = 0.3;

        let max_checks = 100;
        let mut checks = 0;

        for &a in &node_ids {
            for &b in &node_ids {
                if a == b {
                    continue;
                }

                let sim_ab = index.similarity(a, b);
                if sim_ab.map_or(false, |s| s > threshold_high) {
                    for &c in &node_ids {
                        if b == c || a == c {
                            continue;
                        }

                        let sim_bc = index.similarity(b, c);
                        if sim_bc.map_or(false, |s| s > threshold_high) {
                            let sim_ac = index.similarity(a, c);

                            prop_assert!(
                                sim_ac.map_or(false, |s| s > threshold_low),
                                "Triangle coherence violated: sim({}, {}) = {:?}, sim({}, {}) = {:?}, but sim({}, {}) = {:?}",
                                a, b, sim_ab, b, c, sim_bc, a, c, sim_ac
                            );

                            checks += 1;
                            if checks >= max_checks {
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dag_generator_produces_valid_dags() {
        let config = ProptestConfig::default();
        let mut runner = TestRunner::new(config);
        let strategy = random_dag_strategy();

        runner
            .run(
                &strategy,
                |(isa_parents, node_types): (
                    AHashMap<u64, AHashSet<u64>>,
                    AHashMap<u64, MedicalNodeType>,
                )| {
                    for (&child, parents) in &isa_parents {
                        for &parent in parents {
                            prop_assert!(
                                parent < child,
                                "Parent {} should have id less than child {}",
                                parent,
                                child
                            );
                        }
                    }

                    for &node in isa_parents.keys() {
                        prop_assert!(
                            node_types.contains_key(&node),
                            "Node {} should have a type",
                            node
                        );
                    }

                    let index =
                        SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

                    for &node in node_types.keys() {
                        prop_assert!(
                            index.get_embedding(node).is_some(),
                            "Node {} should be in the index",
                            node
                        );
                    }

                    Ok(())
                },
            )
            .unwrap();
    }
}
