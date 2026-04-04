//! End-to-end integration test against a real Gitea instance.
//!
//! Validates the full Symphony flow:
//! 1. Parse WORKFLOW.md config for Gitea
//! 2. Validate configuration
//! 3. Connect to real Gitea tracker and fetch issues
//! 4. Normalise issues into the Issue model
//! 5. Sort candidates for dispatch
//! 6. Check dispatch eligibility
//! 7. Create workspace
//! 8. Render prompt template with real issue data
//! 9. Produce orchestrator state snapshot
//!
//! Requires GITEA_TOKEN environment variable to be set.
//! Run with: cargo test --test gitea_integration_test -- --ignored --nocapture

use terraphim_symphony::config::ServiceConfig;
use terraphim_symphony::config::template::render_prompt;
use terraphim_symphony::config::workflow::WorkflowDefinition;
use terraphim_symphony::orchestrator::dispatch;
use terraphim_symphony::orchestrator::state::OrchestratorRuntimeState;
use terraphim_symphony::tracker::IssueTracker;
use terraphim_symphony::tracker::gitea::GiteaTracker;
use terraphim_symphony::workspace::WorkspaceManager;

use std::collections::HashMap;

/// Build a Gitea-focused ServiceConfig using the real agent-tasks repo.
fn gitea_config(workspace_root: &std::path::Path) -> ServiceConfig {
    let yaml = format!(
        r#"---
tracker:
  kind: gitea
  endpoint: https://git.terraphim.cloud
  api_key: $GITEA_TOKEN
  owner: terraphim
  repo: agent-tasks
  active_states:
    - Todo
    - In Progress
  terminal_states:
    - Done
    - Closed
    - Cancelled

polling:
  interval_ms: 30000

workspace:
  root: "{}"

agent:
  max_concurrent_agents: 3
  max_turns: 5
  max_retry_backoff_ms: 60000

codex:
  command: "echo test-agent"
  turn_timeout_ms: 60000
  read_timeout_ms: 5000
  stall_timeout_ms: 60000
---
You are working on issue {{{{ issue.identifier }}}}: {{{{ issue.title }}}}.

{{% if issue.description %}}
## Issue Description

{{{{ issue.description }}}}
{{% endif %}}

## Instructions

1. Read the issue carefully.
2. Implement the required changes.
3. Write tests.
4. Commit referencing {{{{ issue.identifier }}}}.

{{% if attempt %}}
This is retry attempt {{{{ attempt }}}}. Continue from previous work.
{{% endif %}}"#,
        workspace_root.display()
    );

    let workflow = WorkflowDefinition::parse(&yaml).unwrap();
    ServiceConfig::from_workflow(workflow)
}

#[test]
#[ignore]
fn step_1_parse_and_validate_gitea_config() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = gitea_config(tmp.path());

    // Verify typed config values
    assert_eq!(config.tracker_kind().as_deref(), Some("gitea"));
    assert_eq!(config.tracker_gitea_owner().as_deref(), Some("terraphim"));
    assert_eq!(config.tracker_gitea_repo().as_deref(), Some("agent-tasks"));
    assert_eq!(config.tracker_endpoint(), "https://git.terraphim.cloud");
    assert_eq!(config.max_concurrent_agents(), 3);
    assert_eq!(config.max_turns(), 5);
    assert_eq!(config.poll_interval_ms(), 30_000);
    assert_eq!(config.codex_command(), "echo test-agent");

    // Validate for dispatch -- requires GITEA_TOKEN env var
    let result = config.validate_for_dispatch();
    if result.is_err() {
        println!("Validation error (GITEA_TOKEN may not be set): {result:?}");
    }
    assert!(
        result.is_ok(),
        "Config should validate when GITEA_TOKEN is set"
    );

    println!("[PASS] Step 1: Config parsed and validated successfully");
}

#[tokio::test]
#[ignore]
async fn step_2_gitea_tracker_fetch_issues() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = gitea_config(tmp.path());

    let tracker = GiteaTracker::from_config(&config)
        .expect("GiteaTracker should construct from valid config");

    let issues = tracker
        .fetch_candidate_issues()
        .await
        .expect("Should fetch issues from Gitea");

    println!("Fetched {} candidate issues from Gitea", issues.len());

    for issue in &issues {
        println!(
            "  {} [{}] {} (priority={:?}, labels={:?})",
            issue.identifier, issue.state, issue.title, issue.priority, issue.labels
        );

        // Verify normalisation
        assert!(!issue.id.is_empty(), "Issue ID must not be empty");
        assert!(!issue.identifier.is_empty(), "Identifier must not be empty");
        assert!(
            issue.identifier.contains("terraphim/agent-tasks#"),
            "Identifier should follow owner/repo#N format, got: {}",
            issue.identifier
        );
        assert!(!issue.title.is_empty(), "Title must not be empty");
        assert!(!issue.state.is_empty(), "State must not be empty");
        assert!(issue.is_dispatchable(), "Issue should be dispatchable");
    }

    // We created at least 2 test issues
    assert!(
        issues.len() >= 2,
        "Expected at least 2 open issues, got {}",
        issues.len()
    );

    println!(
        "[PASS] Step 2: Fetched and normalised {} issues",
        issues.len()
    );
}

#[tokio::test]
#[ignore]
async fn step_3_dispatch_sorting_and_eligibility() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = gitea_config(tmp.path());

    let tracker = GiteaTracker::from_config(&config).unwrap();
    let mut issues = tracker.fetch_candidate_issues().await.unwrap();
    assert!(
        !issues.is_empty(),
        "Need at least one issue for dispatch test"
    );

    println!("Before sort:");
    for i in &issues {
        println!(
            "  {} priority={:?} created={:?}",
            i.identifier, i.priority, i.created_at
        );
    }

    // Sort for dispatch
    dispatch::sort_for_dispatch(&mut issues);

    println!("After sort:");
    for i in &issues {
        println!(
            "  {} priority={:?} created={:?}",
            i.identifier, i.priority, i.created_at
        );
    }

    // Check dispatch eligibility
    let state = OrchestratorRuntimeState::new(30_000, 3);
    let active_states = config.active_states();
    let terminal_states = config.terminal_states();
    let per_state_limits = HashMap::new();

    let mut eligible_count = 0;
    for issue in &issues {
        let eligible = dispatch::is_dispatch_eligible(
            issue,
            &state,
            &active_states,
            &terminal_states,
            &per_state_limits,
        );
        println!("  {} eligible={}", issue.identifier, eligible);
        if eligible {
            eligible_count += 1;
        }
    }

    assert!(
        eligible_count > 0,
        "At least one issue should be eligible for dispatch"
    );

    println!(
        "[PASS] Step 3: Sorted {} issues, {} eligible",
        issues.len(),
        eligible_count
    );
}

#[tokio::test]
#[ignore]
async fn step_4_workspace_and_prompt_rendering() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = gitea_config(tmp.path());

    let tracker = GiteaTracker::from_config(&config).unwrap();
    let issues = tracker.fetch_candidate_issues().await.unwrap();
    assert!(!issues.is_empty(), "Need at least one issue");

    let issue = &issues[0];
    println!("Using issue: {} - {}", issue.identifier, issue.title);

    // Create workspace
    let ws_mgr = WorkspaceManager::new(&config).unwrap();
    let ws_info = ws_mgr
        .prepare(&issue.identifier, &std::collections::HashMap::new())
        .await
        .unwrap();

    println!(
        "Workspace created: {} (new={})",
        ws_info.path.display(),
        ws_info.created_now
    );
    assert!(ws_info.path.exists(), "Workspace directory must exist");

    // Render prompt template (first attempt)
    let prompt = render_prompt(config.prompt_template(), issue, None).unwrap();
    println!("--- Rendered prompt (first run) ---");
    println!("{}", prompt);
    println!("--- End prompt ---");

    assert!(
        prompt.contains(&issue.identifier),
        "Prompt must contain issue identifier"
    );
    assert!(
        prompt.contains(&issue.title),
        "Prompt must contain issue title"
    );
    assert!(
        !prompt.contains("retry attempt"),
        "First run prompt should not contain retry text"
    );

    // Render prompt template (retry attempt)
    let retry_prompt = render_prompt(config.prompt_template(), issue, Some(2)).unwrap();
    println!("--- Rendered prompt (retry 2) ---");
    println!("{}", retry_prompt);
    println!("--- End prompt ---");

    assert!(
        retry_prompt.contains("retry attempt 2"),
        "Retry prompt must contain attempt number"
    );

    // Cleanup
    ws_mgr.cleanup(&issue.identifier).await.unwrap();
    assert!(
        !ws_info.path.exists(),
        "Workspace should be removed after cleanup"
    );

    println!("[PASS] Step 4: Workspace created, prompt rendered, workspace cleaned up");
}

#[tokio::test]
#[ignore]
async fn step_5_orchestrator_state_snapshot() {
    let config_yaml = r#"---
tracker:
  kind: gitea
  endpoint: https://git.terraphim.cloud
  api_key: $GITEA_TOKEN
  owner: terraphim
  repo: agent-tasks
---
Prompt."#;

    let workflow = WorkflowDefinition::parse(config_yaml).unwrap();
    let config = ServiceConfig::from_workflow(workflow);

    // Create runtime state and snapshot
    let state =
        OrchestratorRuntimeState::new(config.poll_interval_ms(), config.max_concurrent_agents());

    let snapshot = state.snapshot(chrono::Utc::now());
    println!(
        "Snapshot: {}",
        serde_json::to_string_pretty(&snapshot).unwrap()
    );

    assert_eq!(snapshot.counts.running, 0);
    assert_eq!(snapshot.counts.retrying, 0);
    assert!(snapshot.rate_limits.is_none());
    assert!(snapshot.running.is_empty());
    assert!(snapshot.retrying.is_empty());
    assert_eq!(snapshot.codex_totals.total_tokens, 0);

    println!("[PASS] Step 5: Orchestrator state snapshot generated");
}

#[tokio::test]
#[ignore]
async fn full_end_to_end_chain() {
    println!("=== Symphony End-to-End Validation ===");
    println!();

    // -- 1. Parse and validate config --
    let tmp = tempfile::TempDir::new().unwrap();
    let config = gitea_config(tmp.path());
    config
        .validate_for_dispatch()
        .expect("Config must validate");
    println!("[1/7] Config parsed and validated");

    // -- 2. Create tracker --
    let tracker = GiteaTracker::from_config(&config).expect("Tracker must construct");
    println!("[2/7] GiteaTracker created");

    // -- 3. Fetch issues --
    let issues = tracker
        .fetch_candidate_issues()
        .await
        .expect("Must fetch issues");
    println!("[3/7] Fetched {} candidate issues", issues.len());
    assert!(!issues.is_empty(), "Must have at least one issue");

    // -- 4. Sort for dispatch --
    let mut sorted = issues.clone();
    dispatch::sort_for_dispatch(&mut sorted);
    println!("[4/7] Sorted issues for dispatch:");
    for (i, issue) in sorted.iter().enumerate() {
        println!(
            "  {}: {} [{}] {} (priority={:?})",
            i + 1,
            issue.identifier,
            issue.state,
            issue.title,
            issue.priority
        );
    }

    // -- 5. Check eligibility --
    let state = OrchestratorRuntimeState::new(30_000, config.max_concurrent_agents());
    let active_states = config.active_states();
    let terminal_states = config.terminal_states();
    let per_state_limits = config.max_concurrent_agents_by_state();

    let eligible: Vec<_> = sorted
        .iter()
        .filter(|i| {
            dispatch::is_dispatch_eligible(
                i,
                &state,
                &active_states,
                &terminal_states,
                &per_state_limits,
            )
        })
        .collect();
    println!("[5/7] {} issues eligible for dispatch", eligible.len());
    assert!(!eligible.is_empty(), "At least one issue must be eligible");

    // -- 6. Prepare workspace and render prompt --
    let first = eligible[0];
    let ws_mgr = WorkspaceManager::new(&config).unwrap();
    let ws = ws_mgr
        .prepare(&first.identifier, &std::collections::HashMap::new())
        .await
        .unwrap();
    println!(
        "[6/7] Workspace prepared for {}: {} (new={})",
        first.identifier,
        ws.path.display(),
        ws.created_now
    );

    let prompt = render_prompt(config.prompt_template(), first, None).unwrap();
    assert!(prompt.contains(&first.identifier));
    assert!(prompt.contains(&first.title));
    println!("  Prompt rendered ({} chars)", prompt.len());

    // -- 7. State snapshot --
    let snapshot = state.snapshot(chrono::Utc::now());
    let snapshot_json = serde_json::to_string_pretty(&snapshot).unwrap();
    println!("[7/7] State snapshot:");
    println!("{}", snapshot_json);

    // Cleanup
    ws_mgr.cleanup(&first.identifier).await.unwrap();

    println!();
    println!("=== End-to-End Validation PASSED ===");
}
