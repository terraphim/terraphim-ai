use std::sync::Arc;

use terraphim_config::{ConfigBuilder, ConfigState};
use terraphim_file_search::kg_scorer::KgPathScorer;
use terraphim_mcp_server::McpService;
use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

/// Build a minimal ConfigState for testing (no live services needed).
async fn minimal_config_state() -> Arc<ConfigState> {
    let mut config = ConfigBuilder::default()
        .build()
        .expect("default config builds");
    let state = ConfigState::new(&mut config)
        .await
        .expect("ConfigState from default config");
    Arc::new(state)
}

fn thesaurus_with_terms(name: &str, terms: &[(&str, &str)]) -> Thesaurus {
    let mut t = Thesaurus::new(name.to_string());
    for (id, val) in terms {
        let key = NormalizedTermValue::from(val.to_string());
        let term = NormalizedTerm {
            id: id.to_string(),
            value: NormalizedTermValue::from(val.to_string()),
            display_value: None,
            url: None,
        };
        t.insert(key, term);
    }
    t
}

/// find_files without a KG scorer returns results sorted by fuzzy score alone.
#[tokio::test]
async fn find_files_no_scorer_returns_results() {
    let config_state = minimal_config_state().await;
    let service = McpService::new(config_state);

    // Search in the terraphim-ai workspace root
    let workspace = env!("CARGO_MANIFEST_DIR")
        .trim_end_matches("crates/terraphim_mcp_server")
        .trim_end_matches('/');
    let result = service
        .find_files("lib.rs".to_string(), Some(workspace.to_string()), Some(5))
        .await
        .expect("find_files should succeed");

    assert!(
        !result.is_error.unwrap_or(false),
        "expected success, got: {:?}",
        result.content
    );

    // At least the summary line
    assert!(
        result.content.len() >= 1,
        "expected at least one content item"
    );

    // The summary should mention files found
    let summary = result.content[0]
        .as_text()
        .map(|t| t.text.as_str())
        .unwrap_or("");
    assert!(
        summary.contains("lib.rs") || summary.contains("Found"),
        "unexpected summary: {summary}"
    );
}

/// find_files with a KG scorer boosts paths that match KG terms.
#[tokio::test]
async fn find_files_with_kg_scorer_boosts_matching_paths() {
    let config_state = minimal_config_state().await;

    // Build a thesaurus that recognises "automata" - files under
    // crates/terraphim_automata/ should be boosted.
    let thesaurus = thesaurus_with_terms("test", &[("1", "automata")]);
    let scorer = Arc::new(KgPathScorer::new(thesaurus));
    let service = McpService::new(config_state).with_kg_scorer(scorer);

    let workspace = env!("CARGO_MANIFEST_DIR")
        .trim_end_matches("crates/terraphim_mcp_server")
        .trim_end_matches('/');

    let result = service
        .find_files("lib".to_string(), Some(workspace.to_string()), Some(10))
        .await
        .expect("find_files should succeed");

    assert!(
        !result.is_error.unwrap_or(false),
        "expected success, got: {:?}",
        result.content
    );

    // Verify we got results back
    assert!(result.content.len() > 1, "expected results beyond summary");

    // At least one result should reference automata (boosted to top)
    let has_automata = result.content.iter().any(|c| {
        c.as_text()
            .map(|t| t.text.contains("automata"))
            .unwrap_or(false)
    });
    assert!(
        has_automata,
        "expected automata-path file in top results; got: {:?}",
        result.content
    );
}

/// find_files with a non-existent path returns an error result.
#[tokio::test]
async fn find_files_invalid_path_returns_error() {
    let config_state = minimal_config_state().await;
    let service = McpService::new(config_state);

    let result = service
        .find_files(
            "anything".to_string(),
            Some("/tmp/__terraphim_nonexistent_dir_xyz__".to_string()),
            None,
        )
        .await;

    // Should return an ErrorData (Err) for a missing directory
    assert!(
        result.is_err(),
        "expected Err for non-existent path, got Ok"
    );
}
