use ahash::AHashMap;
use std::path::Path;
use terraphim_automata::parse_markdown_directives_dir;
use terraphim_rolegraph::{DEFAULT_TRIGGER_THRESHOLD, RoleGraph, TriggerIndex};
use terraphim_types::Thesaurus;

/// Resolves the fixture haystack directory relative to this crate's manifest.
fn fixture_haystack_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../terraphim_server/fixtures/haystack")
}

/// Parse the fixture directory, assign sequential node IDs to concepts that have
/// a `trigger::` directive, and return (triggers_map, pinned_ids).
fn build_trigger_data_from_fixtures() -> (AHashMap<u64, String>, Vec<u64>) {
    let fixture_path = fixture_haystack_dir();
    let parse_result =
        parse_markdown_directives_dir(&fixture_path).expect("fixture dir should be parseable");

    let mut triggers: AHashMap<u64, String> = AHashMap::new();
    let mut pinned_ids: Vec<u64> = Vec::new();
    let mut next_id: u64 = 1;

    // Sort by concept name for deterministic ID assignment across test runs.
    let mut concepts: Vec<_> = parse_result.directives.iter().collect();
    concepts.sort_by_key(|(k, _)| k.as_str());

    for (_concept, directives) in concepts {
        if let Some(trigger_text) = &directives.trigger {
            triggers.insert(next_id, trigger_text.clone());
            if directives.pinned {
                pinned_ids.push(next_id);
            }
            next_id += 1;
        }
    }

    (triggers, pinned_ids)
}

#[test]
fn test_empty_index_returns_empty() {
    let index = TriggerIndex::new(0.1);
    let results = index.query("lung cancer treatment");
    assert!(results.is_empty());
}

#[test]
fn test_is_empty_before_build() {
    let index = TriggerIndex::new(0.1);
    assert!(index.is_empty());
}

#[test]
fn test_is_not_empty_after_build() {
    let mut index = TriggerIndex::new(0.1);
    let mut triggers = AHashMap::new();
    triggers.insert(1u64, "lung cancer treatment".to_string());
    index.build(triggers);
    assert!(!index.is_empty());
}

#[test]
fn test_exact_match_scores_highest() {
    let mut index = TriggerIndex::new(0.1);
    let mut triggers = AHashMap::new();
    triggers.insert(1u64, "lung cancer treatment".to_string());
    index.build(triggers);

    let results = index.query("lung cancer treatment");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, 1u64);
    assert!(
        results[0].1 > 0.99,
        "Expected score near 1.0, got {}",
        results[0].1
    );
}

#[test]
fn test_partial_overlap_positive_score() {
    let mut index = TriggerIndex::new(0.1);
    let mut triggers = AHashMap::new();
    triggers.insert(1u64, "lung cancer treatment".to_string());
    index.build(triggers);

    let results = index.query("cancer research");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, 1u64);
    assert!(
        results[0].1 > 0.0,
        "Expected positive score, got {}",
        results[0].1
    );
}

#[test]
fn test_no_overlap_below_threshold() {
    let mut index = TriggerIndex::new(0.1);
    let mut triggers = AHashMap::new();
    triggers.insert(1u64, "lung cancer".to_string());
    index.build(triggers);

    let results = index.query("software engineering");
    assert!(
        results.is_empty(),
        "Expected empty results for unrelated query"
    );
}

#[test]
fn test_stopword_removal() {
    let mut index = TriggerIndex::new(0.1);
    let mut triggers = AHashMap::new();
    triggers.insert(1u64, "treatment for the lung cancer".to_string());
    index.build(triggers);

    // Query without stopwords should still match
    let results = index.query("lung cancer treatment");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, 1u64);
    assert!(results[0].1 > 0.0);
}

#[test]
fn test_short_word_filtering() {
    let mut index = TriggerIndex::new(0.1);
    let mut triggers = AHashMap::new();
    triggers.insert(1u64, "a b cd efg".to_string());
    index.build(triggers);

    // Only "efg" (len > 2) should be indexed
    // Query with "efg" should match
    let results = index.query("efg");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, 1u64);

    // Query with short words only should not match
    let results_short = index.query("cd");
    assert!(
        results_short.is_empty(),
        "Short words (len <= 2) should be filtered"
    );
}

#[test]
fn test_case_insensitivity() {
    let mut index = TriggerIndex::new(0.1);
    let mut triggers = AHashMap::new();
    triggers.insert(1u64, "Lung Cancer".to_string());
    index.build(triggers);

    let results = index.query("lung cancer");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, 1u64);
    assert!(results[0].1 > 0.0);
}

#[test]
fn test_threshold_filtering() {
    let mut index = TriggerIndex::new(0.5); // Higher threshold
    let mut triggers = AHashMap::new();
    triggers.insert(1u64, "lung cancer treatment".to_string());
    index.build(triggers);

    // Partial match should be below 0.5 threshold
    let results = index.query("research");
    assert!(results.is_empty(), "Score should be below 0.5 threshold");
}

#[test]
fn test_result_ordering() {
    let mut index = TriggerIndex::new(0.1);
    let mut triggers = AHashMap::new();
    triggers.insert(1u64, "lung cancer treatment".to_string());
    triggers.insert(2u64, "lung cancer research".to_string());
    triggers.insert(3u64, "heart disease treatment".to_string());
    index.build(triggers);

    let results = index.query("lung cancer treatment");
    assert_eq!(results.len(), 3);

    // First result should be exact match (highest score)
    assert_eq!(results[0].0, 1u64);

    // Results should be sorted descending by score
    for i in 1..results.len() {
        assert!(
            results[i - 1].1 >= results[i].1,
            "Results should be sorted descending by score"
        );
    }
}

#[test]
fn test_idf_weighting() {
    let mut index = TriggerIndex::new(0.1);
    let mut triggers = AHashMap::new();
    // "treatment" appears in many documents (common term)
    triggers.insert(1u64, "lung cancer treatment".to_string());
    triggers.insert(2u64, "heart disease treatment".to_string());
    triggers.insert(3u64, "diabetes treatment".to_string());
    // "research" appears only once (rare term)
    triggers.insert(4u64, "lung cancer research".to_string());
    index.build(triggers);

    // Query with "research" should rank doc 4 higher due to IDF weighting
    let results = index.query("research");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, 4u64);

    // Query with both "lung" and "research"
    // Doc 4 (research) should score higher than docs with just "lung"
    let results = index.query("lung research");
    assert!(!results.is_empty());
    // Doc 4 has the rare "research" term, so it should be ranked highest
    assert_eq!(results[0].0, 4u64);
}

#[test]
fn test_multiple_triggers() {
    let mut index = TriggerIndex::new(0.1);
    let mut triggers = AHashMap::new();

    // Add 10+ triggers
    for i in 1..=12 {
        triggers.insert(i as u64, format!("medical condition {}", i));
    }
    index.build(triggers);

    assert!(!index.is_empty());

    // Query should return all matching triggers
    let results = index.query("medical condition");
    assert_eq!(results.len(), 12, "All 12 triggers should match");
}

#[test]
fn test_rebuild_clears_old_data() {
    let mut index = TriggerIndex::new(0.1);

    // First build
    let mut triggers1 = AHashMap::new();
    triggers1.insert(1u64, "lung cancer".to_string());
    triggers1.insert(2u64, "heart disease".to_string());
    index.build(triggers1);

    assert_eq!(index.query("lung").len(), 1);
    assert_eq!(index.query("heart").len(), 1);

    // Second build with different data
    let mut triggers2 = AHashMap::new();
    triggers2.insert(3u64, "diabetes treatment".to_string());
    index.build(triggers2);

    // Old data should be gone
    assert!(index.query("lung").is_empty(), "Old data should be cleared");
    assert!(
        index.query("heart").is_empty(),
        "Old data should be cleared"
    );

    // New data should be present
    assert_eq!(index.query("diabetes").len(), 1);
    assert_eq!(index.query("diabetes")[0].0, 3u64);
}

#[test]
fn test_default_threshold_constant() {
    assert!((DEFAULT_TRIGGER_THRESHOLD - 0.3).abs() < f64::EPSILON);
}

#[test]
fn test_set_threshold() {
    let mut index = TriggerIndex::new(0.3);
    assert!((index.threshold() - 0.3).abs() < f64::EPSILON);
    index.set_threshold(0.8);
    assert!((index.threshold() - 0.8).abs() < f64::EPSILON);
}

#[test]
fn test_custom_stopwords() {
    // "cancer" is not a default stopword, but we make it one
    let mut stopwords = ahash::AHashSet::new();
    stopwords.insert("cancer".to_string());

    let mut index = TriggerIndex::with_stopwords(0.1, stopwords);
    let mut triggers = AHashMap::new();
    triggers.insert(1u64, "lung cancer treatment".to_string());
    triggers.insert(2u64, "treatment options".to_string());
    index.build(triggers);

    // "cancer" is a stopword now, so querying it should not match trigger 1
    // only "treatment" tokens remain in trigger 1, same as trigger 2
    let results = index.query("cancer");
    assert!(
        results.is_empty(),
        "Custom stopword 'cancer' should be filtered from queries"
    );
}

#[test]
fn test_default_stopwords_filter() {
    // "the" and "with" are default stopwords
    assert!(TriggerIndex::is_default_stopword("the"));
    assert!(TriggerIndex::is_default_stopword("with"));
    assert!(!TriggerIndex::is_default_stopword("cancer"));
    assert!(!TriggerIndex::is_default_stopword("treatment"));
}

// --- Fixture-based integration tests (AC4 of Gitea #84) ---
// These tests load KG markdown files from disk and verify that
// trigger:: and pinned:: directives are correctly parsed and integrated.

#[test]
fn test_kg_fixture_trigger_index_populated() {
    let (triggers, _pinned_ids) = build_trigger_data_from_fixtures();

    assert!(
        !triggers.is_empty(),
        "Expected at least one trigger:: directive in fixture files under \
         terraphim_server/fixtures/haystack/; add a .md file with 'trigger::' to proceed"
    );
}

#[test]
fn test_kg_fixture_pinned_entries_present() {
    let (_triggers, pinned_ids) = build_trigger_data_from_fixtures();

    assert!(
        !pinned_ids.is_empty(),
        "Expected at least one pinned:: true directive in fixture files under \
         terraphim_server/fixtures/haystack/; add a .md file with 'pinned:: true'"
    );
}

#[tokio::test]
async fn test_kg_fixture_trigger_fallback_returns_node_id() {
    let (triggers, pinned_ids) = build_trigger_data_from_fixtures();

    assert!(
        !triggers.is_empty(),
        "No trigger:: directives found in fixture files"
    );

    // Capture the first entry before moving the map.
    let (expected_id, trigger_text) = {
        let (id, text) = triggers.iter().next().unwrap();
        (*id, text.clone())
    };

    let thesaurus = Thesaurus::new("fixture-trigger-test".to_string());
    let mut rolegraph = RoleGraph::new("fixture-trigger-test".into(), thesaurus)
        .await
        .unwrap();
    rolegraph.load_trigger_index(triggers, pinned_ids, DEFAULT_TRIGGER_THRESHOLD);

    // A query closely matching the trigger text should return the node via TF-IDF fallback
    // (the thesaurus is empty so Aho-Corasick finds nothing first).
    let results = rolegraph.find_matching_node_ids_with_fallback(&trigger_text, false);
    assert!(
        results.contains(&expected_id),
        "Expected trigger fallback to return node {expected_id} for query '{trigger_text}', \
         got: {results:?}"
    );
}

#[tokio::test]
async fn test_kg_fixture_pinned_always_included() {
    let (triggers, pinned_ids) = build_trigger_data_from_fixtures();

    assert!(
        !pinned_ids.is_empty(),
        "No pinned:: true directives found in fixture files"
    );

    let first_pinned = pinned_ids[0];

    let thesaurus = Thesaurus::new("fixture-pinned-test".to_string());
    let mut rolegraph = RoleGraph::new("fixture-pinned-test".into(), thesaurus)
        .await
        .unwrap();
    rolegraph.load_trigger_index(triggers, pinned_ids, DEFAULT_TRIGGER_THRESHOLD);

    // Completely unrelated query — pinned entries must still appear when include_pinned=true.
    let results =
        rolegraph.find_matching_node_ids_with_fallback("completely unrelated xyz zyx", true);
    assert!(
        results.contains(&first_pinned),
        "Expected pinned node {first_pinned} to appear with include_pinned=true, \
         got: {:?}",
        results
    );

    // With include_pinned=false, a totally unrelated query must NOT return pinned entries.
    let results_no_pin =
        rolegraph.find_matching_node_ids_with_fallback("completely unrelated xyz zyx", false);
    assert!(
        !results_no_pin.contains(&first_pinned),
        "Pinned node {first_pinned} should NOT appear when include_pinned=false, \
         got: {:?}",
        results_no_pin
    );
}
