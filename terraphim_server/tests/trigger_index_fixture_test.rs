//! Fixture-driven integration tests for `trigger::` and `pinned::` KG directives.
//!
//! Satisfies AC4 of the Gitea #84 plan (issue #2173): verifies that KG markdown
//! files on disk carrying `trigger::` and `pinned::` directives are correctly
//! parsed by `terraphim_automata` and integrated into the `terraphim_rolegraph`
//! trigger-index + two-pass fallback search.
//!
//! These tests deliberately drive the pipeline from disk (the regression gap
//! the issue names): `parse_markdown_directives_dir` -> deterministic ID
//! assignment -> `RoleGraph::load_trigger_index` ->
//! `find_matching_node_ids_with_fallback`. Existing coverage elsewhere uses
//! in-memory triggers only.

#![cfg(test)]

use std::path::{Path, PathBuf};

use ahash::AHashMap;
use terraphim_automata::parse_markdown_directives_dir;
use terraphim_rolegraph::{DEFAULT_TRIGGER_THRESHOLD, RoleGraph};
use terraphim_types::{RoleName, Thesaurus};

/// Resolve the fixture haystack directory relative to this crate's manifest.
///
/// `terraphim_server` is a workspace member at the repo root, so the haystack
/// lives at `<manifest>/fixtures/haystack`.
fn fixture_haystack_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/haystack")
}

/// Parse the fixture directory, assign deterministic sequential node IDs to
/// every concept that carries a `trigger::` directive, and return
/// `(triggers_map, pinned_ids)`.
///
/// IDs are assigned in sorted-by-concept-name order so the mapping is stable
/// across runs and independent of `HashMap` iteration order. A concept appears
/// in the map iff it has a non-empty `trigger::`; it is added to `pinned_ids`
/// iff its `MarkdownDirectives::pinned` flag is `true`.
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

    for (concept, directives) in concepts {
        if let Some(trigger_text) = &directives.trigger {
            let trimmed = trigger_text.trim();
            if trimmed.is_empty() {
                continue;
            }
            triggers.insert(next_id, trimmed.to_string());
            if directives.pinned {
                pinned_ids.push(next_id);
            }
            // Diagnostic on panic: which concept failed to assign.
            let _ = concept;
            next_id += 1;
        }
    }

    (triggers, pinned_ids)
}

/// Build a minimal empty `RoleGraph` (no Aho-Corasick synonyms) and load the
/// fixture-derived trigger index into it. Using an empty thesaurus guarantees
/// that `find_matching_node_ids_with_fallback` exercises the TF-IDF fallback
/// path (pass 2) rather than an Aho-Corasick synonym hit.
fn graph_with_fixture_trigger_index() -> RoleGraph {
    let thesaurus = Thesaurus::new("test".to_string());
    let role = RoleName::from("test-role");
    let mut graph = RoleGraph::new_sync(role, thesaurus).expect("new_sync should succeed");

    let (triggers, pinned) = build_trigger_data_from_fixtures();
    graph.load_trigger_index(triggers, pinned, DEFAULT_TRIGGER_THRESHOLD);
    graph
}

#[test]
fn fixture_dir_contains_trigger_and_pinned_directives() {
    // AC4 (precondition): at least one fixture file carries trigger:: and
    // pinned:: true. If this fails, the fixture
    // `rust_dependency_management_trigger.md` was removed or its directives
    // were altered.
    let (triggers, pinned_ids) = build_trigger_data_from_fixtures();

    assert!(
        !triggers.is_empty(),
        "Expected at least one trigger:: directive in fixture files under \
         terraphim_server/fixtures/haystack/; add a .md file with 'trigger::' \
         to satisfy AC4 of Gitea #84"
    );
    assert!(
        !pinned_ids.is_empty(),
        "Expected at least one pinned:: true directive in fixture files under \
         terraphim_server/fixtures/haystack/; add a .md file with 'pinned:: true' \
         to satisfy AC4 of Gitea #84"
    );
}

#[test]
fn trigger_index_is_populated_from_disk_fixtures() {
    // AC4 (1): the trigger index built from on-disk fixtures is non-empty and
    // the graph reports a non-empty trigger index.
    let graph = graph_with_fixture_trigger_index();

    // The fallback only consults the trigger index when it is non-empty, so
    // confirm the graph actually ingested the fixture data by observing that
    // a trigger-matching query returns a non-empty result (next test) and that
    // an unrelated query against an empty graph would be empty. Here we assert
    // the positive direction: a known fixture trigger phrase matches.
    let results = graph.find_matching_node_ids_with_fallback("cargo dependency management", false);
    assert!(
        !results.is_empty(),
        "trigger index should be populated from fixtures and match a query \
         sharing tokens with the fixture's trigger text"
    );
}

#[test]
fn trigger_only_query_returns_results_via_tfidf_fallback() {
    // AC4 (2): a query that matches ONLY via trigger (no synonym present in the
    // empty thesaurus) must still return results through the TF-IDF fallback.
    let graph = graph_with_fixture_trigger_index();

    // "crate version auditing" shares no synonym with the empty thesaurus but
    // overlaps the fixture trigger text semantically. The fallback should fire.
    let results = graph.find_matching_node_ids_with_fallback("cargo crate version", false);
    assert!(
        !results.is_empty(),
        "a trigger-only query must return results via the TF-IDF fallback when \
         Aho-Corasick finds no synonym matches"
    );

    // Negative control: a completely unrelated query returns nothing (the
    // fallback must not match arbitrarily).
    let unrelated = graph.find_matching_node_ids_with_fallback("xyzzy plugh frobnicate", false);
    assert!(
        unrelated.is_empty(),
        "an unrelated query must return no results; got {unrelated:?}"
    );
}

#[test]
fn pinned_entries_present_only_when_include_pinned_true() {
    // AC4 (3): pinned entries appear in results when include_pinned=true and
    // are absent when include_pinned=false.
    let graph = graph_with_fixture_trigger_index();
    let (_triggers, pinned_ids) = build_trigger_data_from_fixtures();
    let a_pinned_id = pinned_ids
        .first()
        .copied()
        .expect("at least one pinned id is required by the fixture precondition");

    // Use a query that matches NO synonym and NO trigger token, so the ONLY way
    // the pinned id can appear is via the include_pinned path.
    let query = "completely unrelated xyzzy plugh";

    let with_pinned = graph.find_matching_node_ids_with_fallback(query, true);
    assert!(
        with_pinned.contains(&a_pinned_id),
        "pinned node {a_pinned_id} must appear when include_pinned=true; got {with_pinned:?}"
    );

    let without_pinned = graph.find_matching_node_ids_with_fallback(query, false);
    assert!(
        !without_pinned.contains(&a_pinned_id),
        "pinned node {a_pinned_id} must NOT appear when include_pinned=false; got {without_pinned:?}"
    );
}
