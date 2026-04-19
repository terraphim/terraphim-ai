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
fn rejects_banned_fallback_model_at_startup() {
    let result = AgentOrchestrator::from_config_file(fixture("banned_fallback.toml"));
    assert!(result.is_err(), "expected Err for banned fallback_model");
    let err = result.err().unwrap();
    match err {
        OrchestratorError::BannedProvider {
            agent,
            provider,
            field,
        } => {
            assert_eq!(agent, "bad-fallback-agent");
            assert!(
                provider.starts_with("opencode/"),
                "expected opencode/ prefix, got {provider}"
            );
            assert_eq!(field, "fallback_model");
        }
        other => panic!("expected BannedProvider on fallback_model, got {other:?}"),
    }
}

#[test]
fn rejects_mixed_mode_flow_at_startup() {
    let result = AgentOrchestrator::from_config_file(fixture("mixed_mode_flow.toml"));
    assert!(
        result.is_err(),
        "expected Err for flow in legacy (no-projects) mode"
    );
    let err = result.err().unwrap();
    match err {
        OrchestratorError::MixedProjectMode { kind, name } => {
            assert_eq!(kind, "flow");
            assert_eq!(name, "orphan-flow");
        }
        other => panic!("expected MixedProjectMode for flow, got {other:?}"),
    }
}

#[test]
fn accepts_valid_multi_project_config_at_startup() {
    let orch = AgentOrchestrator::from_config_file(fixture("valid_multi_project.toml"))
        .expect("valid multi-project config should load");

    let cfg = orch.config();
    assert_eq!(cfg.projects.len(), 2);
    let project_ids: Vec<&str> = cfg.projects.iter().map(|p| p.id.as_str()).collect();
    assert!(project_ids.contains(&"alpha"));
    assert!(project_ids.contains(&"beta"));
    assert_eq!(cfg.agents.len(), 2);
}

#[test]
fn accepts_valid_legacy_config_at_startup() {
    let orch = AgentOrchestrator::from_config_file(fixture("valid_legacy.toml"))
        .expect("valid legacy config should load");

    let cfg = orch.config();
    assert!(
        cfg.projects.is_empty(),
        "legacy config should have no projects"
    );
    assert_eq!(cfg.agents.len(), 1);
    assert!(cfg.agents[0].project.is_none());
}
