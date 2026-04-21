//! Integration tests for terraphim_automata library
//!
//! These tests verify that the actual terraphim_automata library works
//! for our use case, not just the aho-corasick fallback.

#![cfg(feature = "terraphim")]

use terraphim_automata::find_matches;
use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

/// Create a test thesaurus with wrangler patterns
fn create_wrangler_thesaurus() -> Thesaurus {
    let mut thesaurus = Thesaurus::new("Cloudflare Tools".to_string());

    // Add wrangler patterns
    // The key is the pattern to match, the value is the normalized term
    let wrangler_patterns = vec![
        ("npx wrangler", "wrangler", 1),
        ("bunx wrangler", "wrangler", 2),
        ("pnpm wrangler", "wrangler", 3),
        ("yarn wrangler", "wrangler", 4),
    ];

    for (pattern, normalized, id) in wrangler_patterns {
        let normalized_term = NormalizedTerm {
            display_value: None,
            id,
            value: NormalizedTermValue::from(normalized),
            url: Some("https://developers.cloudflare.com/workers/wrangler/".to_string()),
        };
        thesaurus.insert(NormalizedTermValue::from(pattern), normalized_term);
    }

    thesaurus
}

/// Create a test thesaurus with multiple tool patterns
fn create_comprehensive_thesaurus() -> Thesaurus {
    let mut thesaurus = Thesaurus::new("Development Tools".to_string());

    let patterns = vec![
        // Wrangler patterns
        (
            "npx wrangler",
            "wrangler",
            1,
            "https://developers.cloudflare.com/workers/wrangler/",
        ),
        (
            "bunx wrangler",
            "wrangler",
            2,
            "https://developers.cloudflare.com/workers/wrangler/",
        ),
        // NPM patterns
        (
            "npm install",
            "npm",
            3,
            "https://docs.npmjs.com/cli/install",
        ),
        ("npm test", "npm", 4, "https://docs.npmjs.com/cli/test"),
        // Cargo patterns
        (
            "cargo build",
            "cargo",
            5,
            "https://doc.rust-lang.org/cargo/",
        ),
        ("cargo test", "cargo", 6, "https://doc.rust-lang.org/cargo/"),
    ];

    for (pattern, normalized, id, url) in patterns {
        let normalized_term = NormalizedTerm {
            display_value: None,
            id,
            value: NormalizedTermValue::from(normalized),
            url: Some(url.to_string()),
        };
        thesaurus.insert(NormalizedTermValue::from(pattern), normalized_term);
    }

    thesaurus
}

#[test]
fn test_create_wrangler_thesaurus() {
    let thesaurus = create_wrangler_thesaurus();

    // Verify thesaurus was created
    assert_eq!(thesaurus.name(), "Cloudflare Tools");
    assert!(!thesaurus.is_empty());

    // Verify it contains our patterns by using find_matches
    let text = "npx wrangler deploy";
    let matches = find_matches(text, thesaurus, true).expect("find_matches should succeed");
    assert!(!matches.is_empty(), "Should find npx wrangler pattern");
}

#[test]
fn test_find_npx_wrangler_via_terraphim() {
    let thesaurus = create_wrangler_thesaurus();
    let text = "npx wrangler deploy --env production";

    // Use the actual terraphim_automata find_matches function
    let matches = find_matches(text, thesaurus, true).expect("find_matches should succeed");

    // Verify we found the match
    assert!(!matches.is_empty(), "Should find npx wrangler in text");
    assert_eq!(matches.len(), 1, "Should find exactly one match");

    let matched = &matches[0];
    assert_eq!(matched.term, "npx wrangler");
    assert_eq!(matched.normalized_term.value.to_string(), "wrangler");
    assert_eq!(matched.normalized_term.id, 1);
}

#[test]
fn test_find_bunx_wrangler_via_terraphim() {
    let thesaurus = create_wrangler_thesaurus();
    let text = "bunx wrangler deploy";

    let matches = find_matches(text, thesaurus, true).expect("find_matches should succeed");

    assert!(!matches.is_empty(), "Should find bunx wrangler in text");
    assert_eq!(matches.len(), 1);

    let matched = &matches[0];
    assert_eq!(matched.term, "bunx wrangler");
    assert_eq!(matched.normalized_term.value.to_string(), "wrangler");
    assert_eq!(matched.normalized_term.id, 2);
}

#[test]
fn test_find_multiple_wrangler_invocations() {
    let thesaurus = create_wrangler_thesaurus();
    let text = "npx wrangler login && bunx wrangler deploy";

    let matches = find_matches(text, thesaurus, true).expect("find_matches should succeed");

    // Should find both invocations
    assert_eq!(matches.len(), 2, "Should find both wrangler invocations");

    // Verify first match (npx wrangler)
    assert_eq!(matches[0].term, "npx wrangler");
    assert_eq!(matches[0].normalized_term.value.to_string(), "wrangler");

    // Verify second match (bunx wrangler)
    assert_eq!(matches[1].term, "bunx wrangler");
    assert_eq!(matches[1].normalized_term.value.to_string(), "wrangler");
}

#[test]
fn test_case_insensitive_matching() {
    let thesaurus = create_wrangler_thesaurus();
    let text = "NPX WRANGLER deploy";

    let matches = find_matches(text, thesaurus, true).expect("find_matches should succeed");

    // terraphim_automata uses aho-corasick internally with case-insensitive matching
    assert!(
        !matches.is_empty(),
        "Should find match despite case differences"
    );
}

#[test]
fn test_comprehensive_tool_matching() {
    let thesaurus = create_comprehensive_thesaurus();
    let text = "npm install && cargo build && npx wrangler deploy";

    let matches = find_matches(text, thesaurus, true).expect("find_matches should succeed");

    // Should find all three tools
    assert_eq!(matches.len(), 3, "Should find npm, cargo, and wrangler");

    // Verify each tool was found
    let tool_names: Vec<String> = matches
        .iter()
        .map(|m| m.normalized_term.value.to_string())
        .collect();

    assert!(tool_names.contains(&"npm".to_string()));
    assert!(tool_names.contains(&"cargo".to_string()));
    assert!(tool_names.contains(&"wrangler".to_string()));
}

#[test]
fn test_match_positions() {
    let thesaurus = create_wrangler_thesaurus();
    let text = "npx wrangler deploy";

    // Request position information
    let matches = find_matches(text, thesaurus, true).expect("find_matches should succeed");

    assert_eq!(matches.len(), 1);

    let matched = &matches[0];
    // Verify we have position information
    assert!(matched.pos.is_some(), "Should have position information");

    let (start, end) = matched.pos.unwrap();

    // Verify positions are correct
    assert_eq!(&text[start..end], "npx wrangler");
}

#[test]
fn test_no_matches() {
    let thesaurus = create_wrangler_thesaurus();
    let text = "echo hello world";

    let matches = find_matches(text, thesaurus, false)
        .expect("find_matches should succeed even with no matches");

    assert!(
        matches.is_empty(),
        "Should find no matches in unrelated text"
    );
}

#[test]
fn test_leftmost_longest_matching() {
    let mut thesaurus = Thesaurus::new("Test".to_string());

    // Add overlapping patterns
    thesaurus.insert(
        NormalizedTermValue::from("npm"),
        NormalizedTerm {
            display_value: None,
            id: 1,
            value: NormalizedTermValue::from("npm"),
            url: Some("https://npmjs.com".to_string()),
        },
    );

    thesaurus.insert(
        NormalizedTermValue::from("npm install"),
        NormalizedTerm {
            display_value: None,
            id: 2,
            value: NormalizedTermValue::from("npm-install"),
            url: Some("https://npmjs.com/install".to_string()),
        },
    );

    let text = "npm install packages";
    let matches = find_matches(text, thesaurus, true).expect("find_matches should succeed");

    // Should prefer the longest match
    assert_eq!(matches.len(), 1, "Should find one match (longest)");
    assert_eq!(
        matches[0].term, "npm install",
        "Should match the longer pattern"
    );
}

#[test]
fn test_wrangler_with_complex_flags() {
    let thesaurus = create_wrangler_thesaurus();
    let text = "npx wrangler deploy --env prod --minify --compatibility-date 2024-01-01";

    let matches = find_matches(text, thesaurus, true).expect("find_matches should succeed");

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].term, "npx wrangler");

    // Verify the match is at the beginning
    let (start, _) = matches[0].pos.unwrap();
    assert_eq!(start, 0, "Match should be at the start of the text");
}

#[test]
fn test_all_package_manager_variants() {
    let thesaurus = create_wrangler_thesaurus();

    let test_cases = vec![
        ("npx wrangler deploy", "npx wrangler"),
        ("bunx wrangler deploy", "bunx wrangler"),
        ("pnpm wrangler deploy", "pnpm wrangler"),
        ("yarn wrangler deploy", "yarn wrangler"),
    ];

    for (command, expected_match) in test_cases {
        let matches =
            find_matches(command, thesaurus.clone(), true).expect("find_matches should succeed");

        assert_eq!(matches.len(), 1, "Failed for command: {}", command);
        assert_eq!(
            matches[0].term, expected_match,
            "Failed for command: {}",
            command
        );
        assert_eq!(matches[0].normalized_term.value.to_string(), "wrangler");
    }
}

#[test]
fn test_terraphim_with_json_serialization() {
    let thesaurus = create_wrangler_thesaurus();

    // Serialize thesaurus to JSON
    let json = serde_json::to_string(&thesaurus).expect("Should serialize thesaurus to JSON");

    // Deserialize back
    let deserialized: Thesaurus =
        serde_json::from_str(&json).expect("Should deserialize thesaurus from JSON");

    // Use deserialized thesaurus
    let text = "npx wrangler deploy";
    let matches = find_matches(text, deserialized, true).expect("find_matches should succeed");

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].term, "npx wrangler");
}

#[test]
fn test_terraphim_with_empty_text() {
    let thesaurus = create_wrangler_thesaurus();
    let text = "";

    let matches =
        find_matches(text, thesaurus, false).expect("find_matches should succeed with empty text");

    assert!(matches.is_empty(), "Should find no matches in empty text");
}

#[test]
fn test_terraphim_with_special_characters() {
    let thesaurus = create_wrangler_thesaurus();
    let text = "npx wrangler deploy > deploy.log 2>&1";

    let matches = find_matches(text, thesaurus, true).expect("find_matches should succeed");

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].term, "npx wrangler");
}

#[test]
fn test_terraphim_url_preservation() {
    let thesaurus = create_wrangler_thesaurus();
    let text = "npx wrangler deploy";

    let matches = find_matches(text, thesaurus, true).expect("find_matches should succeed");

    assert_eq!(matches.len(), 1);

    // Verify URL was preserved
    let url = matches[0]
        .normalized_term
        .url
        .as_ref()
        .expect("Should have URL");
    assert_eq!(url, "https://developers.cloudflare.com/workers/wrangler/");
}

#[test]
fn test_terraphim_automata_performance() {
    // Create a larger thesaurus
    let mut thesaurus = Thesaurus::new("Performance Test".to_string());

    // Add 100 patterns
    for i in 0..100 {
        let pattern = format!("tool_{}", i);
        thesaurus.insert(
            NormalizedTermValue::from(pattern.as_str()),
            NormalizedTerm {
                display_value: None,
                id: i,
                value: NormalizedTermValue::from(pattern.as_str()),
                url: Some(format!("https://example.com/{}", i)),
            },
        );
    }

    // Create a large text with multiple matches
    let mut text = String::new();
    for i in (0..100).step_by(10) {
        text.push_str(&format!("tool_{} ", i));
    }

    // This should complete quickly
    let start = std::time::Instant::now();
    let matches = find_matches(&text, thesaurus, true).expect("find_matches should succeed");
    let duration = start.elapsed();

    // Verify matches found
    assert_eq!(matches.len(), 10, "Should find 10 matches");

    // Performance check: should complete in under 10ms for this size
    assert!(
        duration.as_millis() < 10,
        "Should complete quickly, took {:?}",
        duration
    );
}

#[test]
fn test_terraphim_actually_used_not_fallback() {
    // This test proves we're using terraphim_automata, not just aho-corasick
    // by verifying that find_matches works directly with terraphim API

    let thesaurus = create_wrangler_thesaurus();
    let text = "bunx wrangler deploy --env production";

    // Call terraphim_automata::find_matches directly
    let result = find_matches(text, thesaurus, true);

    // If we get a successful result, terraphim is working
    assert!(
        result.is_ok(),
        "terraphim_automata::find_matches should succeed"
    );

    let matches = result.unwrap();
    assert!(!matches.is_empty(), "Should find matches using terraphim");
    assert_eq!(matches[0].term, "bunx wrangler");

    // This proves the library is actually installed and functional,
    // not just a stub or fallback implementation
    println!("SUCCESS: terraphim_automata is actually being used!");
}
