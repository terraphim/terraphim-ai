//! Integration tests for config validation (dispatch preflight).
//!
//! Verifies that `validate_for_dispatch` correctly identifies
//! invalid configuration before the orchestrator attempts dispatch.

use terraphim_symphony::config::workflow::WorkflowDefinition;
use terraphim_symphony::config::ServiceConfig;
use terraphim_symphony::SymphonyError;

fn config_from_yaml(yaml: &str) -> ServiceConfig {
    let content = format!("---\n{yaml}\n---\nPrompt body.");
    let workflow = WorkflowDefinition::parse(&content).unwrap();
    ServiceConfig::from_workflow(workflow)
}

#[test]
fn valid_linear_config() {
    let cfg = config_from_yaml(
        "tracker:\n  kind: linear\n  api_key: test-key\n  project_slug: my-proj",
    );
    assert!(cfg.validate_for_dispatch().is_ok());
}

#[test]
fn valid_gitea_config() {
    let cfg = config_from_yaml(
        "tracker:\n  kind: gitea\n  api_key: test-token\n  owner: me\n  repo: myrepo",
    );
    assert!(cfg.validate_for_dispatch().is_ok());
}

#[test]
fn missing_tracker_kind() {
    let cfg = config_from_yaml("polling:\n  interval_ms: 5000");
    let err = cfg.validate_for_dispatch().unwrap_err();
    match err {
        SymphonyError::ValidationFailed { checks } => {
            assert!(checks.iter().any(|c| c.contains("tracker.kind")));
        }
        _ => panic!("expected ValidationFailed"),
    }
}

#[test]
fn unsupported_tracker_kind() {
    let cfg = config_from_yaml("tracker:\n  kind: jira");
    let err = cfg.validate_for_dispatch().unwrap_err();
    match err {
        SymphonyError::ValidationFailed { checks } => {
            assert!(checks.iter().any(|c| c.contains("unsupported")));
        }
        _ => panic!("expected ValidationFailed"),
    }
}

#[test]
fn linear_missing_project_slug() {
    let cfg = config_from_yaml("tracker:\n  kind: linear\n  api_key: key-123");
    let err = cfg.validate_for_dispatch().unwrap_err();
    match err {
        SymphonyError::ValidationFailed { checks } => {
            assert!(checks.iter().any(|c| c.contains("project_slug")));
        }
        _ => panic!("expected ValidationFailed"),
    }
}

#[test]
fn linear_missing_api_key() {
    // Clear any env var that might provide the key
    // SAFETY: test-specific env var name, single-threaded test context
    unsafe { std::env::remove_var("LINEAR_API_KEY") };

    let cfg = config_from_yaml("tracker:\n  kind: linear\n  project_slug: proj");
    let err = cfg.validate_for_dispatch().unwrap_err();
    match err {
        SymphonyError::ValidationFailed { checks } => {
            assert!(checks.iter().any(|c| c.contains("api_key") || c.contains("LINEAR_API_KEY")));
        }
        _ => panic!("expected ValidationFailed"),
    }
}

#[test]
fn gitea_missing_owner() {
    let cfg = config_from_yaml("tracker:\n  kind: gitea\n  api_key: tok\n  repo: r");
    let err = cfg.validate_for_dispatch().unwrap_err();
    match err {
        SymphonyError::ValidationFailed { checks } => {
            assert!(checks.iter().any(|c| c.contains("owner")));
        }
        _ => panic!("expected ValidationFailed"),
    }
}

#[test]
fn gitea_missing_repo() {
    let cfg = config_from_yaml("tracker:\n  kind: gitea\n  api_key: tok\n  owner: o");
    let err = cfg.validate_for_dispatch().unwrap_err();
    match err {
        SymphonyError::ValidationFailed { checks } => {
            assert!(checks.iter().any(|c| c.contains("repo")));
        }
        _ => panic!("expected ValidationFailed"),
    }
}

#[test]
fn empty_codex_command_rejected() {
    let cfg = config_from_yaml(
        "tracker:\n  kind: linear\n  api_key: k\n  project_slug: p\ncodex:\n  command: \"\"",
    );
    let err = cfg.validate_for_dispatch().unwrap_err();
    match err {
        SymphonyError::ValidationFailed { checks } => {
            assert!(checks.iter().any(|c| c.contains("codex.command")));
        }
        _ => panic!("expected ValidationFailed"),
    }
}

#[test]
fn multiple_validation_failures() {
    // Missing kind AND project slug
    let cfg = config_from_yaml("codex:\n  command: \"\"");
    let err = cfg.validate_for_dispatch().unwrap_err();
    match err {
        SymphonyError::ValidationFailed { checks } => {
            // Should have at least 2 errors: missing kind and empty command
            assert!(checks.len() >= 2, "expected multiple checks, got: {checks:?}");
        }
        _ => panic!("expected ValidationFailed"),
    }
}
