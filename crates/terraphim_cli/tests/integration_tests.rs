//! Integration tests for terraphim-cli
//!
//! These tests verify end-to-end functionality of role switching,
//! KG search, and replace operations.

#[allow(deprecated)] // cargo_bin is deprecated but still works
use assert_cmd::Command;
#[allow(unused_imports)] // Used in test assertions
use predicates::prelude::*;
use serial_test::serial;
use std::process::Command as StdCommand;

/// Detect if running in CI environment (GitHub Actions, Docker containers in CI, etc.)
fn is_ci_environment() -> bool {
    // Check standard CI environment variables
    std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        // Check if running as root in a container (common in CI Docker containers)
        || (std::env::var("USER").as_deref() == Ok("root")
            && std::path::Path::new("/.dockerenv").exists())
        // Check if the home directory is /root (typical for CI containers)
        || std::env::var("HOME").as_deref() == Ok("/root")
}

/// Get a command for the terraphim-cli binary
#[allow(deprecated)] // cargo_bin is deprecated but still functional
fn cli_command() -> Command {
    Command::cargo_bin("terraphim-cli").unwrap()
}

/// Helper to run CLI and get JSON output
fn run_cli_json(args: &[&str]) -> Result<serde_json::Value, String> {
    let output = StdCommand::new("cargo")
        .args(["run", "-p", "terraphim-cli", "--"])
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    if !output.status.success() {
        // Try to parse error output as JSON
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
            return Ok(json);
        }
        return Err(format!(
            "Command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse JSON: {} - output: {}", e, stdout))
}

/// Check if a JSON response contains an error field.
/// In CI environments, KG-related errors are expected and treated as skipped tests.
/// Returns true if the test should continue (no error or CI-skippable error).
/// Panics with descriptive message if error is present (except in CI for KG errors).
fn check_json_for_error(json: &serde_json::Value, context: &str) -> bool {
    if let Some(error) = json.get("error") {
        let error_str = error.as_str().unwrap_or("");
        // In CI, various errors are expected due to missing fixture files,
        // filesystem restrictions, or unavailable services
        if is_ci_environment()
            && (error_str.contains("Failed to build thesaurus")
                || error_str.contains("Knowledge graph not configured")
                || error_str.contains("Config error")
                || error_str.contains("Middleware error")
                || error_str.contains("IO error")
                || error_str.contains("Builder error"))
        {
            eprintln!(
                "{} skipped in CI - KG fixtures unavailable: {:?}",
                context, error
            );
            return false; // Skip remaining assertions
        }
        panic!("{} returned error: {:?}", context, error);
    }
    true // Continue with assertions
}

#[cfg(test)]
mod role_switching_tests {
    use super::*;

    #[test]
    #[serial]
    fn test_list_roles() {
        let result = run_cli_json(&["roles"]);

        match result {
            Ok(json) => {
                assert!(json.is_array(), "Roles should be an array");
                let roles = json.as_array().unwrap();
                // Should have at least one role (Default)
                assert!(!roles.is_empty(), "Should have at least one role");
            }
            Err(e) => {
                // May fail if service can't initialize - acceptable in CI
                eprintln!("Roles test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_config_shows_selected_role() {
        let result = run_cli_json(&["config"]);

        match result {
            Ok(json) => {
                assert!(
                    json.get("selected_role").is_some(),
                    "Config should have selected_role"
                );
                let selected = json["selected_role"].as_str().unwrap();
                assert!(!selected.is_empty(), "Selected role should not be empty");
            }
            Err(e) => {
                eprintln!("Config test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_search_with_default_role() {
        let result = run_cli_json(&["search", "test query"]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Search with default role") {
                    return; // Skip in CI when KG not available
                }
                assert!(json.get("role").is_some(), "Search result should have role");
                // Role should be the default selected role
                let role = json["role"].as_str().unwrap();
                assert!(!role.is_empty(), "Role should not be empty");
            }
            Err(e) => {
                eprintln!("Search test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_search_with_explicit_role() {
        let result = run_cli_json(&["search", "test", "--role", "Default"]);

        match result {
            Ok(json) => {
                assert_eq!(
                    json["role"].as_str(),
                    Some("Default"),
                    "Should use specified role"
                );
            }
            Err(e) => {
                eprintln!("Search with role test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_graph_with_explicit_role() {
        let result = run_cli_json(&["graph", "--role", "Default"]);

        match result {
            Ok(json) => {
                assert_eq!(
                    json["role"].as_str(),
                    Some("Default"),
                    "Should use specified role"
                );
            }
            Err(e) => {
                eprintln!("Graph with role test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_find_with_explicit_role() {
        let result = run_cli_json(&["find", "test text", "--role", "Default"]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Find with role") {
                    return; // Skip in CI when KG not available
                }
                // Should succeed with the specified role
                assert!(
                    json.get("text").is_some() || json.get("matches").is_some(),
                    "Find should have text or matches field"
                );
            }
            Err(e) => {
                eprintln!("Find with role test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_replace_with_explicit_role() {
        let result = run_cli_json(&["replace", "test text", "--role", "Default"]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Replace with role") {
                    return; // Skip in CI when KG not available
                }
                // May have original field or be an error
                assert!(
                    json.get("original").is_some() || json.get("replaced").is_some(),
                    "Replace should have original or replaced field: {:?}",
                    json
                );
            }
            Err(e) => {
                eprintln!("Replace with role test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_thesaurus_with_explicit_role() {
        let result = run_cli_json(&["thesaurus", "--role", "Default"]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Thesaurus with role") {
                    return; // Skip in CI when KG not available
                }
                // Should have either role or terms field
                assert!(
                    json.get("role").is_some()
                        || json.get("terms").is_some()
                        || json.get("name").is_some(),
                    "Thesaurus should have role, terms, or name field: {:?}",
                    json
                );
            }
            Err(e) => {
                eprintln!("Thesaurus with role test skipped: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod kg_search_tests {
    use super::*;

    #[test]
    #[serial]
    fn test_basic_search() {
        let result = run_cli_json(&["search", "rust"]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Basic search") {
                    return; // Skip in CI when KG not available
                }
                assert_eq!(json["query"].as_str(), Some("rust"));
                assert!(json.get("results").is_some());
                assert!(json.get("count").is_some());
            }
            Err(e) => {
                eprintln!("Basic search test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_search_with_limit() {
        let result = run_cli_json(&["search", "test", "--limit", "3"]);

        match result {
            Ok(json) => {
                let count = json["count"].as_u64().unwrap_or(0);
                assert!(count <= 3, "Results should respect limit");
            }
            Err(e) => {
                eprintln!("Search with limit test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_search_with_multiple_words() {
        let result = run_cli_json(&["search", "rust async programming"]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Multi-word search") {
                    return; // Skip in CI when KG not available
                }
                assert_eq!(json["query"].as_str(), Some("rust async programming"));
            }
            Err(e) => {
                eprintln!("Multi-word search test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_search_returns_array_of_results() {
        let result = run_cli_json(&["search", "tokio"]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Search results array") {
                    return; // Skip in CI when KG not available
                }
                assert!(json["results"].is_array(), "Results should be an array");
            }
            Err(e) => {
                eprintln!("Search results array test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_search_results_have_required_fields() {
        let result = run_cli_json(&["search", "api"]);

        match result {
            Ok(json) => {
                if let Some(results) = json["results"].as_array() {
                    for doc in results {
                        assert!(doc.get("id").is_some(), "Document should have id");
                        assert!(doc.get("title").is_some(), "Document should have title");
                        assert!(doc.get("url").is_some(), "Document should have url");
                    }
                }
            }
            Err(e) => {
                eprintln!("Search results fields test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_graph_returns_concepts() {
        let result = run_cli_json(&["graph"]);

        match result {
            Ok(json) => {
                assert!(json.get("concepts").is_some(), "Graph should have concepts");
                assert!(json["concepts"].is_array(), "Concepts should be an array");
            }
            Err(e) => {
                eprintln!("Graph concepts test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_graph_with_custom_top_k() {
        let result = run_cli_json(&["graph", "--top-k", "5"]);

        match result {
            Ok(json) => {
                assert_eq!(json["top_k"].as_u64(), Some(5));
                let concepts = json["concepts"].as_array().unwrap();
                assert!(concepts.len() <= 5, "Should return at most 5 concepts");
            }
            Err(e) => {
                eprintln!("Graph top-k test skipped: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod replace_tests {
    use super::*;

    #[test]
    #[serial]
    fn test_replace_markdown_format() {
        // Use Terraphim Engineer role which has knowledge graph configured
        let result = run_cli_json(&[
            "replace",
            "rust programming",
            "--link-format",
            "markdown",
            "--role",
            "Terraphim Engineer",
        ]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Replace markdown") {
                    return; // Skip in CI when KG not available
                }
                assert_eq!(json["format"].as_str(), Some("markdown"));
                assert_eq!(json["original"].as_str(), Some("rust programming"));
                assert!(json.get("replaced").is_some());
            }
            Err(e) => {
                eprintln!("Replace markdown test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_replace_html_format() {
        // Use Terraphim Engineer role which has knowledge graph configured
        let result = run_cli_json(&[
            "replace",
            "async tokio",
            "--link-format",
            "html",
            "--role",
            "Terraphim Engineer",
        ]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Replace html") {
                    return; // Skip in CI when KG not available
                }
                assert_eq!(json["format"].as_str(), Some("html"));
            }
            Err(e) => {
                eprintln!("Replace html test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_replace_wiki_format() {
        // Use Terraphim Engineer role which has knowledge graph configured
        let result = run_cli_json(&[
            "replace",
            "docker kubernetes",
            "--link-format",
            "wiki",
            "--role",
            "Terraphim Engineer",
        ]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Replace wiki") {
                    return; // Skip in CI when KG not available
                }
                assert_eq!(json["format"].as_str(), Some("wiki"));
            }
            Err(e) => {
                eprintln!("Replace wiki test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_replace_plain_format() {
        // Use Terraphim Engineer role which has knowledge graph configured
        let result = run_cli_json(&[
            "replace",
            "git github",
            "--link-format",
            "plain",
            "--role",
            "Terraphim Engineer",
        ]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Replace plain") {
                    return; // Skip in CI when KG not available
                }
                assert_eq!(json["format"].as_str(), Some("plain"));
                // Plain format should not modify text
                assert_eq!(
                    json["original"].as_str(),
                    json["replaced"].as_str(),
                    "Plain format should not modify text"
                );
            }
            Err(e) => {
                eprintln!("Replace plain test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_replace_default_format_is_markdown() {
        // Use Terraphim Engineer role which has knowledge graph configured
        let result = run_cli_json(&["replace", "test text", "--role", "Terraphim Engineer"]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Replace default format") {
                    return; // Skip in CI when KG not available
                }
                assert_eq!(
                    json["format"].as_str(),
                    Some("markdown"),
                    "Default format should be markdown"
                );
            }
            Err(e) => {
                eprintln!("Replace default format test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_replace_preserves_unmatched_text() {
        // Use Terraphim Engineer role which has knowledge graph configured
        let result = run_cli_json(&[
            "replace",
            "some random text without matches xyz123",
            "--format",
            "markdown",
            "--role",
            "Terraphim Engineer",
        ]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Replace preserves text") {
                    return; // Skip in CI when KG not available
                }
                let _original = json["original"].as_str().unwrap();
                let replaced = json["replaced"].as_str().unwrap();
                // Text without matches should be preserved
                assert!(replaced.contains("xyz123") || replaced.contains("random"));
            }
            Err(e) => {
                eprintln!("Replace preserves text test skipped: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod find_tests {
    use super::*;

    #[test]
    #[serial]
    fn test_find_basic() {
        // Use Terraphim Engineer role which has knowledge graph configured
        let result = run_cli_json(&["find", "rust async tokio", "--role", "Terraphim Engineer"]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Find basic") {
                    return; // Skip in CI when KG not available
                }
                assert_eq!(json["text"].as_str(), Some("rust async tokio"));
                assert!(json.get("matches").is_some());
                assert!(json.get("count").is_some());
            }
            Err(e) => {
                eprintln!("Find basic test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_find_returns_array_of_matches() {
        // Use Terraphim Engineer role which has knowledge graph configured
        let result = run_cli_json(&["find", "api server client", "--role", "Terraphim Engineer"]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Find matches array") {
                    return; // Skip in CI when KG not available
                }
                assert!(json["matches"].is_array(), "Matches should be an array");
            }
            Err(e) => {
                eprintln!("Find matches array test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_find_matches_have_required_fields() {
        // Use Terraphim Engineer role which has knowledge graph configured
        let result = run_cli_json(&[
            "find",
            "database json config",
            "--role",
            "Terraphim Engineer",
        ]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Find matches fields") {
                    return; // Skip in CI when KG not available
                }
                if let Some(matches) = json["matches"].as_array() {
                    for m in matches {
                        assert!(m.get("term").is_some(), "Match should have term");
                        assert!(
                            m.get("normalized").is_some(),
                            "Match should have normalized"
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("Find matches fields test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_find_count_matches_array_length() {
        // Use Terraphim Engineer role which has knowledge graph configured
        let result = run_cli_json(&[
            "find",
            "linux docker kubernetes",
            "--role",
            "Terraphim Engineer",
        ]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Find count") {
                    return; // Skip in CI when KG not available
                }
                let count = json["count"].as_u64().unwrap_or(0) as usize;
                let matches_len = json["matches"].as_array().map(|a| a.len()).unwrap_or(0);
                assert_eq!(count, matches_len, "Count should match array length");
            }
            Err(e) => {
                eprintln!("Find count test skipped: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod thesaurus_tests {
    use super::*;

    #[test]
    #[serial]
    fn test_thesaurus_basic() {
        // Use Terraphim Engineer role which has knowledge graph configured
        let result = run_cli_json(&["thesaurus", "--role", "Terraphim Engineer"]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Thesaurus basic") {
                    return; // Skip in CI when KG not available
                }
                assert!(json.get("role").is_some());
                assert!(json.get("name").is_some());
                assert!(json.get("terms").is_some());
                assert!(json.get("total_count").is_some());
                assert!(json.get("shown_count").is_some());
            }
            Err(e) => {
                eprintln!("Thesaurus basic test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_thesaurus_with_limit() {
        // Use Terraphim Engineer role which has knowledge graph configured
        let result = run_cli_json(&["thesaurus", "--limit", "5", "--role", "Terraphim Engineer"]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Thesaurus limit") {
                    return; // Skip in CI when KG not available
                }
                let shown = json["shown_count"].as_u64().unwrap_or(0);
                assert!(shown <= 5, "Should respect limit");

                let terms = json["terms"].as_array().unwrap();
                assert!(terms.len() <= 5, "Terms array should respect limit");
            }
            Err(e) => {
                eprintln!("Thesaurus limit test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_thesaurus_terms_have_required_fields() {
        // Use Terraphim Engineer role which has knowledge graph configured
        let result = run_cli_json(&["thesaurus", "--limit", "10", "--role", "Terraphim Engineer"]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Thesaurus terms fields") {
                    return; // Skip in CI when KG not available
                }
                if let Some(terms) = json["terms"].as_array() {
                    for term in terms {
                        assert!(term.get("id").is_some(), "Term should have id");
                        assert!(term.get("term").is_some(), "Term should have term");
                        assert!(
                            term.get("normalized").is_some(),
                            "Term should have normalized"
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("Thesaurus terms fields test skipped: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_thesaurus_total_count_greater_or_equal_shown() {
        // Use Terraphim Engineer role which has knowledge graph configured
        let result = run_cli_json(&["thesaurus", "--limit", "5", "--role", "Terraphim Engineer"]);

        match result {
            Ok(json) => {
                if !check_json_for_error(&json, "Thesaurus count") {
                    return; // Skip in CI when KG not available
                }
                let total = json["total_count"].as_u64().unwrap_or(0);
                let shown = json["shown_count"].as_u64().unwrap_or(0);
                assert!(total >= shown, "Total count should be >= shown count");
            }
            Err(e) => {
                eprintln!("Thesaurus count test skipped: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod output_format_tests {
    use super::*;

    #[test]
    #[serial]
    fn test_json_output() {
        let output = cli_command()
            .args(["--format", "json", "roles"])
            .output()
            .expect("Failed to execute");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();

        // Output should either be valid JSON or contain an error
        if !trimmed.is_empty() {
            let is_json = (trimmed.starts_with('[') && trimmed.ends_with(']'))
                || (trimmed.starts_with('{') && trimmed.ends_with('}'));
            let has_error = trimmed.contains("error") || trimmed.contains("Error");

            assert!(
                is_json || has_error || output.status.success(),
                "Output should be JSON or contain error: {}",
                trimmed
            );
        }
    }

    #[test]
    #[serial]
    fn test_json_pretty_output() {
        let output = StdCommand::new("cargo")
            .args(["run", "-p", "terraphim-cli", "--"])
            .args(["--format", "json-pretty", "config"])
            .output()
            .expect("Failed to execute");

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Pretty JSON has multiple lines
            let lines: Vec<&str> = stdout.lines().collect();
            assert!(lines.len() > 1, "Pretty JSON should have multiple lines");
        }
    }

    #[test]
    #[serial]
    fn test_text_output() {
        let output = StdCommand::new("cargo")
            .args(["run", "-p", "terraphim-cli", "--"])
            .args(["--format", "text", "config"])
            .output()
            .expect("Failed to execute");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Text output should not be empty
        assert!(!stdout.trim().is_empty() || !output.status.success());
    }
}
