//! Constructor-level validation tests for `AgentOrchestrator::from_config_file`.
//!
//! These verify that invalid configs are rejected at production startup,
//! not just in the `adf --check` dry-run path.

use std::path::PathBuf;

use terraphim_orchestrator::error::OrchestratorError;
use terraphim_orchestrator::AgentOrchestrator;

fn fixture(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push("runtime_validate");
    p.push(name);
    p
}

#[test]
fn rejects_banned_provider_at_startup() {
    let result = AgentOrchestrator::from_config_file(fixture("banned_provider.toml"));
    assert!(result.is_err(), "expected Err for banned provider");
    let err = result.err().unwrap();
    match err {
        OrchestratorError::BannedProvider {
            agent,
            provider,
            field,
        } => {
            assert_eq!(agent, "bad-agent");
            assert!(
                provider.starts_with("opencode/"),
                "expected opencode/ prefix, got {provider}"
            );
            assert_eq!(field, "model");
        }
        other => panic!("expected BannedProvider, got {other:?}"),
    }
}

#[test]
fn rejects_unknown_project_ref_at_startup() {
    let result = AgentOrchestrator::from_config_file(fixture("unknown_project_ref.toml"));
    assert!(result.is_err(), "expected Err for unknown project ref");
    let err = result.err().unwrap();
    match err {
        OrchestratorError::UnknownAgentProject { agent, project } => {
            assert_eq!(agent, "ghost-agent");
            assert_eq!(project, "nonexistent");
        }
        other => panic!("expected UnknownAgentProject, got {other:?}"),
    }
}

#[test]
fn rejects_duplicate_project_id_at_startup() {
    let result = AgentOrchestrator::from_config_file(fixture("duplicate_project_id.toml"));
    assert!(result.is_err(), "expected Err for duplicate project id");
    let err = result.err().unwrap();
    match err {
        OrchestratorError::DuplicateProjectId(id) => {
            assert_eq!(id, "alpha");
        }
        other => panic!("expected DuplicateProjectId, got {other:?}"),
    }
}

#[test]
fn rejects_mixed_mode_at_startup() {
    let result = AgentOrchestrator::from_config_file(fixture("mixed_mode.toml"));
    assert!(result.is_err(), "expected Err for mixed project mode");
    let err = result.err().unwrap();
    match err {
        OrchestratorError::MixedProjectMode { kind, name } => {
            assert_eq!(kind, "agent");
            assert_eq!(name, "unbound-agent");
        }
        other => panic!("expected MixedProjectMode, got {other:?}"),
    }
}

#[test]
fn accepts_valid_multi_project_config_at_startup() {
    let result = AgentOrchestrator::from_config_file(fixture("valid_multi_project.toml"));
    assert!(
        result.is_ok(),
        "valid multi-project config should load without error: {:?}",
        result.err()
    );
}

#[test]
fn accepts_valid_legacy_config_at_startup() {
    let result = AgentOrchestrator::from_config_file(fixture("valid_legacy.toml"));
    assert!(
        result.is_ok(),
        "valid legacy config should load without error: {:?}",
        result.err()
    );
}
