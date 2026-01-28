//! Integration tests for the CLI onboarding wizard
//!
//! These tests verify the end-to-end functionality of the onboarding module
//! including template application, role configuration, and CLI integration.

use terraphim_config::ServiceType;
use terraphim_types::RelevanceFunction;

// Re-export from the agent crate's onboarding module
// Note: These tests use the public API of the onboarding module

/// Test that all 6 templates are available and can be applied
#[test]
fn test_all_templates_available() {
    use terraphim_agent::onboarding::{apply_template, TemplateRegistry};

    let registry = TemplateRegistry::new();
    let templates = registry.list();

    assert_eq!(templates.len(), 6, "Should have exactly 6 templates");

    let expected_ids = [
        "terraphim-engineer",
        "llm-enforcer",
        "rust-engineer",
        "local-notes",
        "ai-engineer",
        "log-analyst",
    ];

    for id in expected_ids {
        let template = registry.get(id);
        assert!(template.is_some(), "Template '{}' should exist", id);
    }
}

/// Test that terraphim-engineer template creates correct role
#[test]
fn test_terraphim_engineer_template_integration() {
    use terraphim_agent::onboarding::apply_template;

    let role = apply_template("terraphim-engineer", None).expect("Should apply template");

    assert_eq!(role.name.to_string(), "Terraphim Engineer");
    assert_eq!(role.shortname, Some("terra".to_string()));
    assert_eq!(role.relevance_function, RelevanceFunction::TerraphimGraph);
    assert!(
        role.terraphim_it,
        "terraphim_it should be true for TerraphimGraph"
    );
    assert!(role.kg.is_some(), "Should have knowledge graph configured");
    assert!(
        !role.haystacks.is_empty(),
        "Should have at least one haystack"
    );
    assert_eq!(role.haystacks[0].service, ServiceType::Ripgrep);
}

/// Test that llm-enforcer template creates correct role with local KG
#[test]
fn test_llm_enforcer_template_integration() {
    use terraphim_agent::onboarding::apply_template;

    let role = apply_template("llm-enforcer", None).expect("Should apply template");

    assert_eq!(role.name.to_string(), "LLM Enforcer");
    assert_eq!(role.shortname, Some("enforce".to_string()));
    assert!(role.kg.is_some(), "Should have knowledge graph configured");

    let kg = role.kg.as_ref().unwrap();
    assert!(
        kg.knowledge_graph_local.is_some(),
        "Should have local knowledge graph"
    );
    assert!(
        kg.automata_path.is_none(),
        "Should not have remote automata path"
    );
}

/// Test that local-notes template requires path
#[test]
fn test_local_notes_requires_path() {
    use terraphim_agent::onboarding::apply_template;

    let result = apply_template("local-notes", None);
    assert!(result.is_err(), "Should fail without path");

    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("requires"),
        "Error should mention path requirement"
    );
}

/// Test that local-notes template works with path
#[test]
fn test_local_notes_with_path() {
    use terraphim_agent::onboarding::apply_template;

    let role = apply_template("local-notes", Some("/tmp/test-notes"))
        .expect("Should apply template with path");

    assert_eq!(role.name.to_string(), "Local Notes");
    assert_eq!(role.haystacks[0].location, "/tmp/test-notes");
    assert_eq!(role.haystacks[0].service, ServiceType::Ripgrep);
}

/// Test that ai-engineer template has Ollama LLM configured
#[test]
fn test_ai_engineer_has_llm() {
    use terraphim_agent::onboarding::apply_template;

    let role = apply_template("ai-engineer", None).expect("Should apply template");

    assert_eq!(role.name.to_string(), "AI Engineer");
    assert!(role.llm_enabled, "LLM should be enabled");
    assert!(
        role.extra.contains_key("llm_provider"),
        "Should have llm_provider"
    );
    assert!(
        role.extra.contains_key("ollama_model"),
        "Should have ollama_model"
    );
}

/// Test that rust-engineer template uses QueryRs
#[test]
fn test_rust_engineer_uses_queryrs() {
    use terraphim_agent::onboarding::apply_template;

    let role = apply_template("rust-engineer", None).expect("Should apply template");

    assert_eq!(role.name.to_string(), "Rust Engineer");
    assert_eq!(role.haystacks[0].service, ServiceType::QueryRs);
    assert!(role.haystacks[0].location.contains("query.rs"));
}

/// Test that log-analyst template uses Quickwit
#[test]
fn test_log_analyst_uses_quickwit() {
    use terraphim_agent::onboarding::apply_template;

    let role = apply_template("log-analyst", None).expect("Should apply template");

    assert_eq!(role.name.to_string(), "Log Analyst");
    assert_eq!(role.haystacks[0].service, ServiceType::Quickwit);
    assert_eq!(role.relevance_function, RelevanceFunction::BM25);
}

/// Test that invalid template returns error
#[test]
fn test_invalid_template_error() {
    use terraphim_agent::onboarding::apply_template;

    let result = apply_template("nonexistent-template", None);
    assert!(result.is_err(), "Should fail for nonexistent template");

    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("not found"),
        "Error should mention template not found"
    );
}

/// Test that custom path overrides default
#[test]
fn test_custom_path_override() {
    use terraphim_agent::onboarding::apply_template;

    let custom_path = "/custom/search/path";
    let role =
        apply_template("terraphim-engineer", Some(custom_path)).expect("Should apply template");

    assert_eq!(
        role.haystacks[0].location, custom_path,
        "Custom path should override default"
    );
}

/// Test template registry listing
#[test]
fn test_template_registry_list() {
    use terraphim_agent::onboarding::TemplateRegistry;

    let registry = TemplateRegistry::new();
    let templates = registry.list();

    // Verify first template is terraphim-engineer (primary)
    assert_eq!(templates[0].id, "terraphim-engineer");
    assert_eq!(templates[0].name, "Terraphim Engineer");

    // Verify second template is llm-enforcer (second priority)
    assert_eq!(templates[1].id, "llm-enforcer");
    assert_eq!(templates[1].name, "LLM Enforcer");

    // Verify all templates have required fields
    for template in templates {
        assert!(!template.id.is_empty(), "Template ID should not be empty");
        assert!(
            !template.name.is_empty(),
            "Template name should not be empty"
        );
        assert!(
            !template.description.is_empty(),
            "Template description should not be empty"
        );
    }
}
