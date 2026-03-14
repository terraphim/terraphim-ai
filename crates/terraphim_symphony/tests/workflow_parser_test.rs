//! Integration tests for WORKFLOW.md parsing.
//!
//! Tests parsing of the sample fixture and various edge cases
//! to exercise the full config pipeline end-to-end.

use terraphim_symphony::config::workflow::WorkflowDefinition;
use terraphim_symphony::config::ServiceConfig;

#[test]
fn parse_fixture_workflow() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/sample_workflow.md");
    let def = WorkflowDefinition::load(&path).unwrap();

    // Verify YAML front matter was parsed
    assert!(!def.config.is_empty());

    // Verify tracker section
    let tracker = def.config.get("tracker").unwrap().as_mapping().unwrap();
    let kind = tracker.get("kind").unwrap().as_str().unwrap();
    assert_eq!(kind, "gitea");

    let owner = tracker.get("owner").unwrap().as_str().unwrap();
    assert_eq!(owner, "terraphim");

    let repo = tracker.get("repo").unwrap().as_str().unwrap();
    assert_eq!(repo, "test-project");

    // Verify prompt template contains Liquid tags
    assert!(def.prompt_template.contains("{{ issue.identifier }}"));
    assert!(def.prompt_template.contains("{{ issue.title }}"));
    assert!(def.prompt_template.contains("{% if issue.description %}"));
    assert!(def.prompt_template.contains("{% if attempt %}"));
}

#[test]
fn fixture_config_typed_values() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/sample_workflow.md");
    let def = WorkflowDefinition::load(&path).unwrap();
    let config = ServiceConfig::from_workflow(def);

    assert_eq!(config.tracker_kind().as_deref(), Some("gitea"));
    assert_eq!(config.tracker_gitea_owner().as_deref(), Some("terraphim"));
    assert_eq!(config.tracker_gitea_repo().as_deref(), Some("test-project"));
    assert_eq!(config.poll_interval_ms(), 30_000);
    assert_eq!(config.max_concurrent_agents(), 5);
    assert_eq!(config.max_turns(), 10);
    assert_eq!(config.max_retry_backoff_ms(), 300_000);
    assert_eq!(config.codex_command(), "codex app-server");
    assert_eq!(config.codex_turn_timeout_ms(), 3_600_000);
    assert_eq!(config.codex_read_timeout_ms(), 5_000);
    assert_eq!(config.codex_stall_timeout_ms(), 300_000);
    assert_eq!(config.hooks_timeout_ms(), 30_000);

    assert_eq!(config.active_states(), vec!["Todo", "In Progress"]);
    assert_eq!(config.terminal_states(), vec!["Done", "Closed", "Cancelled"]);

    assert!(config.hooks_after_create().is_some());
    assert!(config.hooks_before_run().is_some());
    assert!(config.hooks_after_run().is_none());
    assert!(config.hooks_before_remove().is_none());
}

#[test]
fn parse_minimal_workflow() {
    let content = "Just a prompt template.";
    let def = WorkflowDefinition::parse(content).unwrap();
    assert!(def.config.is_empty());
    assert_eq!(def.prompt_template, "Just a prompt template.");
}

#[test]
fn parse_empty_front_matter() {
    let content = "---\n---\nPrompt after empty front matter.";
    let def = WorkflowDefinition::parse(content).unwrap();
    assert!(def.config.is_empty());
    assert_eq!(def.prompt_template, "Prompt after empty front matter.");
}

#[test]
fn parse_front_matter_only() {
    let content = "---\nkey: value\n---";
    let def = WorkflowDefinition::parse(content).unwrap();
    assert!(!def.config.is_empty());
    assert_eq!(def.prompt_template, "");
}

#[test]
fn parse_complex_front_matter() {
    let content = r#"---
tracker:
  kind: linear
  project_slug: my-project
  api_key: $LINEAR_API_KEY
  active_states:
    - Backlog
    - Todo
    - In Progress
  terminal_states:
    - Done
    - Cancelled
agent:
  max_concurrent_agents: 3
  max_turns: 15
  max_concurrent_agents_by_state:
    todo: 1
    in progress: 2
codex:
  command: "claude --agent"
  turn_timeout_ms: 1800000
---
Work on {{ issue.identifier }}.
"#;
    let def = WorkflowDefinition::parse(content).unwrap();
    let config = ServiceConfig::from_workflow(def);

    assert_eq!(config.tracker_kind().as_deref(), Some("linear"));
    assert_eq!(config.tracker_project_slug().as_deref(), Some("my-project"));
    assert_eq!(config.max_concurrent_agents(), 3);
    assert_eq!(config.max_turns(), 15);
    assert_eq!(config.codex_command(), "claude --agent");
    assert_eq!(config.codex_turn_timeout_ms(), 1_800_000);

    let by_state = config.max_concurrent_agents_by_state();
    assert_eq!(by_state.get("todo"), Some(&1));
    assert_eq!(by_state.get("in progress"), Some(&2));

    assert_eq!(
        config.active_states(),
        vec!["Backlog", "Todo", "In Progress"]
    );
}

#[test]
fn parse_non_map_front_matter_fails() {
    let content = "---\n- list item\n---\nBody.";
    let result = WorkflowDefinition::parse(content);
    assert!(result.is_err());
}

#[test]
fn parse_invalid_yaml_fails() {
    let content = "---\n: invalid: yaml: [broken\n---\nBody.";
    let result = WorkflowDefinition::parse(content);
    assert!(result.is_err());
}

#[test]
fn load_nonexistent_file_fails() {
    let result = WorkflowDefinition::load(std::path::Path::new("/nonexistent/WORKFLOW.md"));
    assert!(result.is_err());
}
