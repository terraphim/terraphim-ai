//! Integration tests for ADF agent validation across all trigger modes.
//!
//! These tests verify cross-mode consistency and trigger matrix invariants
//! without requiring a live orchestrator or spawned binaries.

use tempfile::TempDir;
use terraphim_orchestrator::config::{AgentDefinition, AgentLayer, OrchestratorConfig};
use terraphim_orchestrator::{
    applicable_modes, schedule_for_agent, validate_agent_all_modes, TriggerMode,
};

fn fixture_config(agents: Vec<AgentDefinition>) -> OrchestratorConfig {
    let tmp = TempDir::new().unwrap();
    OrchestratorConfig {
        working_dir: tmp.path().to_path_buf(),
        nightwatch: Default::default(),
        compound_review: Default::default(),
        workflow: None,
        agents,
        restart_cooldown_secs: 60,
        max_restart_count: 10,
        restart_budget_window_secs: 43_200,
        disk_usage_threshold: 90,
        tick_interval_secs: 30,
        gate_reconcile_interval_ticks: 20,
        handoff_buffer_ttl_secs: None,
        persona_data_dir: None,
        skill_data_dir: None,
        flows: vec![],
        flow_state_dir: None,
        gitea: None,
        mentions: None,
        webhook: None,
        role_config_path: None,
        routing: None,
        #[cfg(feature = "quickwit")]
        quickwit: None,
        projects: vec![],
        include: vec![],
        providers: vec![],
        provider_budget_state_file: None,
        pause_dir: None,
        project_circuit_breaker_threshold: 3,
        fleet_escalation_owner: None,
        fleet_escalation_repo: None,
        post_merge_gate: None,
        learning: Default::default(),
        evolution: Default::default(),
        pr_dispatch: None,
        pr_dispatch_per_project: std::collections::HashMap::new(),
        gitea_skill_repo: None,
        direct_dispatch: None,
    }
}

fn make_agent(
    name: &str,
    layer: AgentLayer,
    schedule: Option<&str>,
    event_only: bool,
) -> AgentDefinition {
    AgentDefinition {
        name: name.to_string(),
        layer,
        cli_tool: "echo".to_string(),
        task: "test task".to_string(),
        schedule: schedule.map(String::from),
        model: Some("minimax-coding-plan/MiniMax-M2.5".to_string()),
        default_tier: None,
        capabilities: vec![],
        max_memory_bytes: None,
        budget_monthly_cents: None,
        provider: None,
        persona: None,
        terraphim_role: None,
        skill_chain: vec![],
        sfia_skills: vec![],
        fallback_provider: None,
        fallback_model: None,
        grace_period_secs: None,
        max_cpu_seconds: None,
        pre_check: None,
        gitea_issue: None,
        event_only,
        project: None,
        evolution_enabled: false,
        rlm_enabled: None,
        bypass_kg_routing: false,
        enabled: true,
    }
}

#[test]
fn test_cron_agents_have_schedule() {
    let agents = vec![
        make_agent(
            "scheduled-agent",
            AgentLayer::Core,
            Some("0 */6 * * *"),
            false,
        ),
        make_agent("event-agent", AgentLayer::Growth, None, true),
    ];
    let config = fixture_config(agents);

    for agent in &config.agents {
        let modes = applicable_modes(agent);
        if modes.contains(&TriggerMode::Cron) {
            assert!(
                agent.schedule.is_some(),
                "agent {} has Cron mode but no schedule",
                agent.name
            );
            let expr = agent.schedule.as_ref().unwrap();
            assert!(
                !expr.trim().is_empty(),
                "agent {} has Cron mode but empty schedule",
                agent.name
            );
        }
    }
}

#[test]
fn test_event_only_agents_no_schedule() {
    let agents = vec![
        make_agent(
            "scheduled-agent",
            AgentLayer::Core,
            Some("0 */6 * * *"),
            false,
        ),
        make_agent("event-agent", AgentLayer::Growth, None, true),
    ];
    let config = fixture_config(agents);

    for agent in &config.agents {
        let modes = applicable_modes(agent);
        if agent.event_only {
            assert!(
                agent.schedule.is_none(),
                "agent {} is event_only but has schedule: {:?}",
                agent.name,
                agent.schedule
            );
            assert!(
                modes.contains(&TriggerMode::PullRequest),
                "event-only agent {} missing PullRequest mode",
                agent.name
            );
            assert!(
                modes.contains(&TriggerMode::Push),
                "event-only agent {} missing Push mode",
                agent.name
            );
        }
    }
}

#[test]
fn test_non_event_only_agents_have_mention_mode() {
    let agents = vec![
        make_agent(
            "scheduled-agent",
            AgentLayer::Core,
            Some("0 */6 * * *"),
            false,
        ),
        make_agent("safety-agent", AgentLayer::Safety, None, false),
    ];
    let config = fixture_config(agents);

    for agent in &config.agents {
        if !agent.event_only {
            let modes = applicable_modes(agent);
            assert!(
                modes.contains(&TriggerMode::Mention),
                "non-event-only agent {} missing Mention mode",
                agent.name
            );
        }
    }
}

#[test]
fn test_all_agents_have_local_mode() {
    let agents = vec![
        make_agent(
            "scheduled-agent",
            AgentLayer::Core,
            Some("0 */6 * * *"),
            false,
        ),
        make_agent("event-agent", AgentLayer::Growth, None, true),
        make_agent("safety-agent", AgentLayer::Safety, None, false),
    ];
    let config = fixture_config(agents);

    for agent in &config.agents {
        let modes = applicable_modes(agent);
        assert!(
            modes.contains(&TriggerMode::Local),
            "agent {} missing Local mode",
            agent.name
        );
    }
}

#[test]
fn test_all_agents_have_webhook_mode() {
    let agents = vec![
        make_agent(
            "scheduled-agent",
            AgentLayer::Core,
            Some("0 */6 * * *"),
            false,
        ),
        make_agent("event-agent", AgentLayer::Growth, None, true),
    ];
    let config = fixture_config(agents);

    for agent in &config.agents {
        let modes = applicable_modes(agent);
        assert!(
            modes.contains(&TriggerMode::Webhook),
            "agent {} missing Webhook mode",
            agent.name
        );
    }
}

#[test]
fn test_validate_all_modes_completes_for_cron_agent() {
    let agent = make_agent(
        "scheduled-agent",
        AgentLayer::Core,
        Some("0 */6 * * *"),
        false,
    );
    let config = fixture_config(vec![agent.clone()]);

    let (report, mode_results) = validate_agent_all_modes(&config, &agent);

    assert_eq!(report.agent_name, "scheduled-agent");
    assert!(mode_results.contains_key(&TriggerMode::Local));
    assert!(mode_results.contains_key(&TriggerMode::Cron));
    assert!(mode_results.contains_key(&TriggerMode::Mention));
    assert!(mode_results.contains_key(&TriggerMode::Webhook));
    assert!(!mode_results.contains_key(&TriggerMode::PullRequest));
    assert!(!mode_results.contains_key(&TriggerMode::Push));
}

#[test]
fn test_validate_all_modes_completes_for_event_only_agent() {
    let agent = make_agent("event-agent", AgentLayer::Growth, None, true);
    let config = fixture_config(vec![agent.clone()]);

    let (_report, mode_results) = validate_agent_all_modes(&config, &agent);

    assert!(mode_results.contains_key(&TriggerMode::Local));
    assert!(mode_results.contains_key(&TriggerMode::PullRequest));
    assert!(mode_results.contains_key(&TriggerMode::Push));
    assert!(mode_results.contains_key(&TriggerMode::Webhook));
    assert!(!mode_results.contains_key(&TriggerMode::Cron));
    assert!(!mode_results.contains_key(&TriggerMode::Mention));
}

#[test]
fn test_schedule_for_agent_returns_correct_expression() {
    let agents = vec![
        make_agent("agent-a", AgentLayer::Core, Some("0 3 * * *"), false),
        make_agent("agent-b", AgentLayer::Growth, Some("15 */6 * * *"), false),
        make_agent("agent-c", AgentLayer::Safety, None, false),
    ];
    let config = fixture_config(agents);

    assert_eq!(
        schedule_for_agent(&config, "agent-a"),
        Some("0 3 * * *".to_string())
    );
    assert_eq!(
        schedule_for_agent(&config, "agent-b"),
        Some("15 */6 * * *".to_string())
    );
    assert_eq!(schedule_for_agent(&config, "agent-c"), None);
    assert_eq!(schedule_for_agent(&config, "nonexistent"), None);
}

#[test]
fn test_applicable_modes_for_mixed_agents() {
    let agents = [
        make_agent("cron-mention", AgentLayer::Core, Some("0 */6 * * *"), false),
        make_agent("event-only", AgentLayer::Growth, None, true),
        make_agent("safety-only", AgentLayer::Safety, None, false),
    ];

    let cron_agent = agents.iter().find(|a| a.name == "cron-mention").unwrap();
    let event_agent = agents.iter().find(|a| a.name == "event-only").unwrap();
    let safety_agent = agents.iter().find(|a| a.name == "safety-only").unwrap();

    let cron_modes = applicable_modes(cron_agent);
    assert!(cron_modes.contains(&TriggerMode::Cron));
    assert!(cron_modes.contains(&TriggerMode::Mention));
    assert!(!cron_modes.contains(&TriggerMode::PullRequest));
    assert!(!cron_modes.contains(&TriggerMode::Push));

    let event_modes = applicable_modes(event_agent);
    assert!(!event_modes.contains(&TriggerMode::Cron));
    assert!(!event_modes.contains(&TriggerMode::Mention));
    assert!(event_modes.contains(&TriggerMode::PullRequest));
    assert!(event_modes.contains(&TriggerMode::Push));

    let safety_modes = applicable_modes(safety_agent);
    assert!(!safety_modes.contains(&TriggerMode::Cron));
    assert!(safety_modes.contains(&TriggerMode::Mention));
    assert!(!safety_modes.contains(&TriggerMode::PullRequest));
    assert!(!safety_modes.contains(&TriggerMode::Push));
}

#[test]
fn test_mode_results_have_correct_trigger_mode_set() {
    let agent = make_agent("test-agent", AgentLayer::Core, Some("0 */6 * * *"), false);
    let config = fixture_config(vec![agent.clone()]);

    let (_report, mode_results) = validate_agent_all_modes(&config, &agent);

    for (mode, result) in &mode_results {
        assert_eq!(
            result.trigger_mode, *mode,
            "ModeResult.trigger_mode {:?} does not match map key",
            result.trigger_mode
        );
    }
}
