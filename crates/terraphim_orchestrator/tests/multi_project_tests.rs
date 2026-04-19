//! Integration tests for multi-project config schema, include-glob loader,
//! and load-time validation (C1 banned providers, project references,
//! mixed-mode rejection).

use std::path::PathBuf;

use terraphim_orchestrator::config::OrchestratorConfig;
use terraphim_orchestrator::error::OrchestratorError;

fn fixture(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push("multi_project");
    p.push(name);
    p
}

#[test]
fn parses_inline_multi_project_config() {
    let config = OrchestratorConfig::from_file(fixture("base_inline.toml")).unwrap();

    assert_eq!(config.projects.len(), 2);
    assert_eq!(config.projects[0].id, "alpha");
    assert_eq!(config.projects[1].id, "beta");
    assert_eq!(config.projects[1].schedule_offset_minutes, 5);

    assert_eq!(config.agents.len(), 2);
    assert_eq!(config.agents[0].project.as_deref(), Some("alpha"));
    assert_eq!(config.agents[1].project.as_deref(), Some("beta"));

    config.validate().unwrap();
}

#[test]
fn expands_include_glob_and_merges_fragments() {
    let config = OrchestratorConfig::from_file(fixture("base_include.toml")).unwrap();

    assert_eq!(config.projects.len(), 2);
    let ids: Vec<&str> = config.projects.iter().map(|p| p.id.as_str()).collect();
    assert!(ids.contains(&"alpha"));
    assert!(ids.contains(&"beta"));

    assert_eq!(config.agents.len(), 3);
    let names: Vec<&str> = config.agents.iter().map(|a| a.name.as_str()).collect();
    assert!(names.contains(&"alpha-watcher"));
    assert!(names.contains(&"beta-watcher"));
    assert!(names.contains(&"beta-reviewer"));

    assert_eq!(config.flows.len(), 1);
    assert_eq!(config.flows[0].name, "alpha-flow");
    assert_eq!(config.flows[0].project, "alpha");

    assert_eq!(config.include, vec!["conf.d/*.toml".to_string()]);

    config.validate().unwrap();
}

#[test]
fn rejects_agent_with_unknown_project() {
    let toml_str = r#"
working_dir = "/tmp/t"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp/repo"

[[projects]]
id = "alpha"
working_dir = "/tmp/alpha"

[[agents]]
name = "ghost"
layer = "Safety"
cli_tool = "claude"
task = "Haunt"
project = "nonexistent"
"#;
    let config = OrchestratorConfig::from_toml(toml_str).unwrap();
    let err = config.validate().unwrap_err();
    match err {
        OrchestratorError::UnknownAgentProject { agent, project } => {
            assert_eq!(agent, "ghost");
            assert_eq!(project, "nonexistent");
        }
        other => panic!("expected UnknownAgentProject, got {other:?}"),
    }
}

#[test]
fn rejects_flow_with_unknown_project() {
    let toml_str = r#"
working_dir = "/tmp/t"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp/repo"

[[projects]]
id = "alpha"
working_dir = "/tmp/alpha"

[[agents]]
name = "alpha-watcher"
layer = "Safety"
cli_tool = "claude"
task = "Watch"
project = "alpha"

[[flows]]
name = "orphan-flow"
project = "nonexistent"
repo_path = "/tmp/x"

[[flows.steps]]
name = "build"
kind = "action"
command = "cargo build"
"#;
    let config = OrchestratorConfig::from_toml(toml_str).unwrap();
    let err = config.validate().unwrap_err();
    match err {
        OrchestratorError::UnknownFlowProject { flow, project } => {
            assert_eq!(flow, "orphan-flow");
            assert_eq!(project, "nonexistent");
        }
        other => panic!("expected UnknownFlowProject, got {other:?}"),
    }
}

#[test]
fn rejects_duplicate_project_id() {
    let toml_str = r#"
working_dir = "/tmp/t"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp/repo"

[[projects]]
id = "alpha"
working_dir = "/tmp/alpha"

[[projects]]
id = "alpha"
working_dir = "/tmp/alpha2"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "claude"
task = "t"
project = "alpha"
"#;
    let config = OrchestratorConfig::from_toml(toml_str).unwrap();
    let err = config.validate().unwrap_err();
    match err {
        OrchestratorError::DuplicateProjectId(id) => assert_eq!(id, "alpha"),
        other => panic!("expected DuplicateProjectId, got {other:?}"),
    }
}

#[test]
fn rejects_mixed_mode_agent_without_project() {
    let toml_str = r#"
working_dir = "/tmp/t"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp/repo"

[[projects]]
id = "alpha"
working_dir = "/tmp/alpha"

[[agents]]
name = "with-project"
layer = "Safety"
cli_tool = "claude"
task = "t"
project = "alpha"

[[agents]]
name = "without-project"
layer = "Safety"
cli_tool = "claude"
task = "t"
"#;
    let config = OrchestratorConfig::from_toml(toml_str).unwrap();
    let err = config.validate().unwrap_err();
    match err {
        OrchestratorError::MixedProjectMode { kind, name } => {
            assert_eq!(kind, "agent");
            assert_eq!(name, "without-project");
        }
        other => panic!("expected MixedProjectMode, got {other:?}"),
    }
}

#[test]
fn rejects_banned_provider_prefixes() {
    let banned = [
        "opencode/foo",
        "github-copilot/gpt-4",
        "google/gemini-2",
        "huggingface/llama3",
        "minimax/abab",
    ];
    for model in banned {
        let toml_str = format!(
            r#"
working_dir = "/tmp/t"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp/repo"

[[projects]]
id = "p"
working_dir = "/tmp/p"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "claude"
task = "t"
project = "p"
model = "{model}"
"#
        );
        let config = OrchestratorConfig::from_toml(&toml_str).unwrap();
        let err = config
            .validate()
            .err()
            .unwrap_or_else(|| panic!("expected error for {model}"));
        match err {
            OrchestratorError::BannedProvider { provider, field, .. } => {
                assert_eq!(provider, model, "provider mismatch for {model}");
                assert_eq!(field, "model");
            }
            other => panic!("expected BannedProvider for {model}, got {other:?}"),
        }
    }
}

#[test]
fn rejects_banned_fallback_provider() {
    let toml_str = r#"
working_dir = "/tmp/t"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp/repo"

[[projects]]
id = "p"
working_dir = "/tmp/p"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "claude"
task = "t"
project = "p"
model = "sonnet"
fallback_model = "google/gemini-2"
"#;
    let config = OrchestratorConfig::from_toml(toml_str).unwrap();
    let err = config.validate().unwrap_err();
    match err {
        OrchestratorError::BannedProvider { field, provider, .. } => {
            assert_eq!(field, "fallback_model");
            assert_eq!(provider, "google/gemini-2");
        }
        other => panic!("expected BannedProvider, got {other:?}"),
    }
}

#[test]
fn accepts_allowed_provider_prefixes_and_bare_models() {
    let allowed = [
        "opencode-go/minimax-m2.5",
        "kimi-for-coding/k2p5",
        "minimax-coding-plan/abab",
        "zai-coding-plan/glm-4",
        "claude-code/sonnet",
        "sonnet",
        "opus",
        "haiku",
    ];
    for model in allowed {
        let toml_str = format!(
            r#"
working_dir = "/tmp/t"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp/repo"

[[projects]]
id = "p"
working_dir = "/tmp/p"

[[agents]]
name = "a"
layer = "Safety"
cli_tool = "claude"
task = "t"
project = "p"
model = "{model}"
"#
        );
        let config = OrchestratorConfig::from_toml(&toml_str).unwrap();
        config
            .validate()
            .unwrap_or_else(|e| panic!("model {model} should be allowed but got {e:?}"));
    }
}

#[test]
fn legacy_single_project_mode_parses_without_projects() {
    let toml_str = r#"
working_dir = "/tmp/t"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp/repo"

[[agents]]
name = "legacy-agent"
layer = "Safety"
cli_tool = "claude"
task = "t"
"#;
    let config = OrchestratorConfig::from_toml(toml_str).unwrap();
    assert!(config.projects.is_empty());
    assert!(config.agents[0].project.is_none());
    config.validate().unwrap();
}
