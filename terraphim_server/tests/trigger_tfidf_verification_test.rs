//! Verification test for issue #2835: trigger::/pinned:: KG directives + TF-IDF fallback.
//!
//! The features were implemented in terraphim-core and published to the registries.
//! This test confirms the public API is accessible at the version consumed by terraphim_server.
//! It runs entirely in-process with no external services.

#[cfg(test)]
mod trigger_tfidf_verification {
    use ahash::AHashMap;
    use terraphim_rolegraph::{RoleGraph, TriggerIndex};
    use terraphim_types::{RoleName, Thesaurus};

    #[test]
    fn trigger_index_builds_and_queries() {
        let mut index = TriggerIndex::new(0.3);
        let mut triggers: AHashMap<u64, String> = AHashMap::new();
        triggers.insert(
            1,
            "managing cargo dependencies in rust projects".to_string(),
        );
        triggers.insert(2, "async tokio runtime configuration".to_string());
        index.build(triggers);

        let results = index.query("cargo dependency management");
        assert!(
            !results.is_empty(),
            "TF-IDF should match cargo dependency trigger"
        );
        let ids: Vec<u64> = results.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&1), "node 1 should score above threshold");

        let empty = index.query("completely unrelated xyzzy");
        assert!(empty.is_empty(), "unrelated query should return no results");
    }

    #[test]
    fn trigger_index_empty_returns_empty() {
        let index = TriggerIndex::new(0.3);
        assert!(index.query("any query").is_empty());
        assert!(index.is_empty());
    }

    #[test]
    fn rolegraph_find_with_fallback_is_callable() {
        let thesaurus = Thesaurus::new("test".to_string());
        let role = RoleName::from("test-role");
        let graph = RoleGraph::new_sync(role, thesaurus).expect("new_sync should succeed");

        // Empty graph => no Aho-Corasick matches, no trigger index => empty results
        let results = graph.find_matching_node_ids_with_fallback("dependency management", false);
        assert!(
            results.is_empty(),
            "empty graph should return empty results"
        );
    }

    #[test]
    fn rolegraph_load_trigger_index_pinned_always_included() {
        let thesaurus = Thesaurus::new("test".to_string());
        let role = RoleName::from("test-role");
        let mut graph = RoleGraph::new_sync(role, thesaurus).expect("new_sync should succeed");

        let mut triggers: AHashMap<u64, String> = AHashMap::new();
        triggers.insert(42, "semantic search with knowledge graphs".to_string());
        graph.load_trigger_index(triggers, vec![99], 0.3);

        // Pinned node 99 must appear in results even when no term matches
        let results = graph.find_matching_node_ids_with_fallback("unrelated xyzzy", true);
        assert!(
            results.contains(&99),
            "pinned node 99 must always be included when include_pinned=true"
        );

        // include_pinned=false: pinned node should NOT appear (no synonym match either)
        let results_no_pinned =
            graph.find_matching_node_ids_with_fallback("unrelated xyzzy", false);
        assert!(
            !results_no_pinned.contains(&99),
            "pinned node 99 must not appear when include_pinned=false"
        );
    }
}
