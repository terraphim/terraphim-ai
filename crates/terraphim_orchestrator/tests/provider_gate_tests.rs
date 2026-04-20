//! Integration tests for the subscription-aware provider gate and
//! per-provider hour/day budget tracker (Gitea #6).
//!
//! Scenarios exercised:
//! 1. C1/C3 allow-list drops a banned static candidate.
//! 2. `CostTracker::should_pause()` skips dispatch for an exhausted
//!    monthly-budget agent.
//! 3. `ProviderBudgetTracker` hour window exhausts and recovers.
//! 4. Day window exhausts independently of the hour window.
//! 5. Reloading the tracker from a snapshot discards state for providers
//!    that were removed from config.
//! 6. Persistence round-trip survives `with_persistence`.
//! 7. `RoutingDecisionEngine` drops `Exhausted` candidates before scoring.
//!
//! These tests hit real implementations -- no mocks. They avoid the
//! `#[cfg(test)]` module scoping of unit tests so they exercise the
//! public surface and catch any future visibility regressions.

use chrono::{TimeZone, Utc};
use std::path::PathBuf;
use std::sync::Arc;

use terraphim_orchestrator::config::is_allowed_provider;
use terraphim_orchestrator::control_plane::routing::{
    BudgetPressure, DispatchContext, RouteSource, RoutingDecisionEngine,
};
use terraphim_orchestrator::control_plane::telemetry::{CompletionEvent, TokenBreakdown};
use terraphim_orchestrator::cost_tracker::{BudgetVerdict, CostTracker};
use terraphim_orchestrator::provider_budget::{
    provider_has_budget, provider_key_for_model, ProviderBudgetConfig, ProviderBudgetTracker,
};
use terraphim_orchestrator::{
    AgentDefinition, AgentLayer, AgentOrchestrator, CompoundReviewConfig, NightwatchConfig,
    OrchestratorConfig,
};

fn dispatch_ctx_with_static(agent: &str, model: &str) -> DispatchContext {
    DispatchContext {
        agent_name: agent.to_string(),
        task: "task body".to_string(),
        static_model: Some(model.to_string()),
        cli_tool: "opencode".to_string(),
        layer: terraphim_orchestrator::config::AgentLayer::Core,
        session_id: None,
    }
}

// === Scenario 1: C1/C3 allow-list ==========================================

#[test]
fn c1_allowed_prefixes_pass() {
    for allowed in [
        "claude-code/anthropic/claude-sonnet-4-5",
        "opencode-go/minimax-m2.5",
        "kimi-for-coding/k2p5",
        "minimax-coding-plan/MiniMax-M2.5",
        "zai-coding-plan/glm-4.6",
        "sonnet",
        "opus",
        "haiku",
        "anthropic/claude-3-5-sonnet",
    ] {
        assert!(
            is_allowed_provider(allowed),
            "expected {allowed} to pass allow-list"
        );
    }
}

#[test]
fn c3_banned_prefixes_rejected() {
    for banned in [
        "opencode/gpt-4",
        "github-copilot/gpt-5",
        "google/gemini-2.0",
        "huggingface/some-model",
        "minimax/MiniMax-M2.5",
    ] {
        assert!(
            !is_allowed_provider(banned),
            "expected {banned} to be banned"
        );
    }
}

// === Scenario 2: CostTracker should_pause dispatch skip ===================

#[test]
fn cost_tracker_should_pause_reports_exhausted() {
    let mut ct = CostTracker::new();
    ct.register("cold-agent", Some(100)); // $1 cap
    ct.record_cost("cold-agent", 2.00); // $2 spent
    let verdict = ct.check("cold-agent");
    assert!(
        verdict.should_pause(),
        "expected should_pause() true, got {verdict:?}"
    );
    assert!(matches!(verdict, BudgetVerdict::Exhausted { .. }));
}

#[test]
fn cost_tracker_uncapped_never_pauses() {
    let mut ct = CostTracker::new();
    ct.register("unbounded", None);
    ct.record_cost("unbounded", 9999.0);
    let verdict = ct.check("unbounded");
    assert!(!verdict.should_pause());
    assert!(matches!(verdict, BudgetVerdict::Uncapped));
}

// === Scenario 3: Hour window exhausts and recovers ========================

#[test]
fn hour_window_exhausts_and_recovers_next_hour() {
    let t = ProviderBudgetTracker::new(vec![ProviderBudgetConfig {
        id: "opencode-go".to_string(),
        max_hour_cents: Some(100),
        max_day_cents: None,
        error_signatures: None,
    }]);
    let t0 = Utc.with_ymd_and_hms(2026, 4, 19, 10, 30, 0).unwrap();
    let t_next = Utc.with_ymd_and_hms(2026, 4, 19, 11, 5, 0).unwrap();

    let _ = t.record_cost_at("opencode-go", 1.50, t0);
    assert!(matches!(
        t.check_at("opencode-go", t0),
        BudgetVerdict::Exhausted { .. }
    ));
    // Next hour -> fresh bucket.
    assert_eq!(
        t.check_at("opencode-go", t_next),
        BudgetVerdict::WithinBudget
    );
}

// === Scenario 4: Day window independent of hour ===========================

#[test]
fn day_window_independent_of_hour() {
    let t = ProviderBudgetTracker::new(vec![ProviderBudgetConfig {
        id: "opencode-go".to_string(),
        max_hour_cents: Some(100),
        max_day_cents: Some(150),
        error_signatures: None,
    }]);
    let t0 = Utc.with_ymd_and_hms(2026, 4, 19, 10, 0, 0).unwrap();
    let t1 = Utc.with_ymd_and_hms(2026, 4, 19, 11, 0, 0).unwrap();

    // $0.90 in hour 10 -> both windows: near but not exhausted.
    let _ = t.record_cost_at("opencode-go", 0.90, t0);
    // $0.70 in hour 11 -> hour bucket is only $0.70 (healthy) but day
    // bucket is now $1.60 > $1.50 cap -> Exhausted.
    let _ = t.record_cost_at("opencode-go", 0.70, t1);
    let verdict = t.check_at("opencode-go", t1);
    assert!(
        matches!(verdict, BudgetVerdict::Exhausted { .. }),
        "day cap should trip across hour boundary; got {verdict:?}"
    );
}

// === Scenario 5: stale snapshot entries discarded =========================

#[test]
fn reload_drops_state_for_removed_providers() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    // Drop the empty placeholder so `with_persistence` treats it as missing.
    drop(tmp);

    // Session 1: persist state for "old-provider".
    let t1 = ProviderBudgetTracker::with_persistence(
        vec![ProviderBudgetConfig {
            id: "old-provider".to_string(),
            max_hour_cents: Some(100),
            max_day_cents: None,
            error_signatures: None,
        }],
        path.clone(),
    )
    .unwrap();
    let now = Utc.with_ymd_and_hms(2026, 4, 19, 10, 0, 0).unwrap();
    let _ = t1.record_cost_at("old-provider", 0.50, now);
    t1.persist().unwrap();

    // Session 2: config removes "old-provider", adds "new-provider".
    let t2 = ProviderBudgetTracker::with_persistence(
        vec![ProviderBudgetConfig {
            id: "new-provider".to_string(),
            max_hour_cents: Some(100),
            max_day_cents: None,
            error_signatures: None,
        }],
        path.clone(),
    )
    .unwrap();
    let snap = t2.snapshot();
    assert!(
        !snap.providers.contains_key("old-provider"),
        "stale provider state must not leak across config edits"
    );

    let _ = std::fs::remove_file(&path);
}

// === Scenario 6: persistence round-trip ===================================

#[test]
fn persistence_round_trip_preserves_spend() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    drop(tmp);

    let cfgs = vec![ProviderBudgetConfig {
        id: "kimi-for-coding".to_string(),
        max_hour_cents: Some(500),
        max_day_cents: Some(2000),
        error_signatures: None,
    }];

    let t1 = ProviderBudgetTracker::with_persistence(cfgs.clone(), path.clone()).unwrap();
    let t0 = Utc.with_ymd_and_hms(2026, 4, 19, 10, 0, 0).unwrap();
    let _ = t1.record_cost_at("kimi-for-coding", 1.23, t0);
    t1.persist().unwrap();

    let t2 = ProviderBudgetTracker::with_persistence(cfgs, path.clone()).unwrap();
    let snap = t2.snapshot();
    let entry = snap
        .providers
        .get("kimi-for-coding")
        .expect("provider state must survive round-trip");
    // $1.23 -> 12_300 sub-cents (hundredths).
    assert_eq!(entry.hour.sub_cents, 12_300);
    assert_eq!(entry.day.sub_cents, 12_300);

    let _ = std::fs::remove_file(&path);
}

// === Scenario 7: routing drops Exhausted candidate ========================

#[tokio::test]
async fn routing_drops_provider_budget_exhausted_candidate() {
    // opencode-go: $0.50/hour. Pre-spend $1.00 to exhaust the hour
    // bucket, then ask the routing engine to pick a candidate whose
    // model prefix is opencode-go. It must be filtered out and the
    // engine must fall back to the CLI default.
    let tracker = ProviderBudgetTracker::new(vec![ProviderBudgetConfig {
        id: "opencode-go".to_string(),
        max_hour_cents: Some(50),
        max_day_cents: None,
        error_signatures: None,
    }]);
    let _ = tracker.record_cost("opencode-go", 1.00);
    assert!(
        !provider_has_budget(&tracker, "opencode-go"),
        "sanity: provider should be exhausted before the routing call"
    );

    let engine = RoutingDecisionEngine::with_provider_budget(
        None,
        Vec::new(),
        terraphim_router::Router::new(),
        None,
        Some(Arc::new(tracker)),
    );

    let ctx = dispatch_ctx_with_static("agent", "opencode-go/minimax-m2.5");
    let decision = engine.decide_route(&ctx, &BudgetVerdict::Uncapped).await;

    assert_eq!(
        decision.candidate.source,
        RouteSource::CliDefault,
        "exhausted candidate must not win; rationale={}",
        decision.rationale
    );
    assert!(
        decision.rationale.contains("provider-budget"),
        "rationale should reference provider-budget: {}",
        decision.rationale
    );
    assert_eq!(decision.budget_pressure, BudgetPressure::NoPressure);
}

// === Scenario 8: record_telemetry wiring drives ProviderBudgetTracker ====

fn agent_with_model(name: &str, model: &str) -> AgentDefinition {
    AgentDefinition {
        name: name.to_string(),
        layer: AgentLayer::Core,
        cli_tool: "echo".to_string(),
        task: "task".to_string(),
        model: Some(model.to_string()),
        schedule: None,
        capabilities: vec![],
        max_memory_bytes: None,
        budget_monthly_cents: Some(10_000),
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
        project: None,
    }
}

fn budget_aware_config(
    providers: Vec<ProviderBudgetConfig>,
    state_file: Option<PathBuf>,
    agents: Vec<AgentDefinition>,
) -> OrchestratorConfig {
    OrchestratorConfig {
        working_dir: PathBuf::from("/tmp/terraphim-provider-budget-test"),
        nightwatch: NightwatchConfig::default(),
        compound_review: CompoundReviewConfig {
            cli_tool: None,
            provider: None,
            model: None,
            schedule: "0 2 * * *".to_string(),
            max_duration_secs: 60,
            repo_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            create_prs: false,
            worktree_root: PathBuf::from("/tmp/terraphim-provider-budget-test/.worktrees"),
            base_branch: "main".to_string(),
            max_concurrent_agents: 3,
            ..Default::default()
        },
        workflow: None,
        agents,
        restart_cooldown_secs: 60,
        max_restart_count: 10,
        restart_budget_window_secs: 43_200,
        disk_usage_threshold: 100,
        tick_interval_secs: 30,
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
        providers,
        provider_budget_state_file: state_file,
        pause_dir: None,
        project_circuit_breaker_threshold: 3,
        fleet_escalation_owner: None,
        fleet_escalation_repo: None,
    }
}

fn completion_event(model: &str, cost_usd: f64) -> CompletionEvent {
    CompletionEvent {
        model: model.to_string(),
        session_id: "sess-1".to_string(),
        completed_at: Utc::now(),
        latency_ms: 100,
        success: true,
        tokens: TokenBreakdown::default(),
        cost_usd,
        error: None,
    }
}

#[tokio::test]
async fn record_telemetry_feeds_provider_budget_tracker() {
    // Regression for the P1 "Layer 3 is read-only at runtime": feed
    // telemetry events through the orchestrator's `record_telemetry`
    // and confirm the provider budget tracker observes the spend.
    let providers = vec![ProviderBudgetConfig {
        id: "opencode-go".to_string(),
        max_hour_cents: Some(50),
        max_day_cents: Some(200),
        error_signatures: None,
    }];
    let config = budget_aware_config(
        providers,
        None,
        vec![agent_with_model("worker", "opencode-go/minimax-m2.5")],
    );

    let orch = AgentOrchestrator::new(config).expect("build orchestrator");
    let tracker = orch
        .provider_budget_tracker()
        .cloned()
        .expect("tracker must be constructed when [[providers]] is set");

    // Sanity: nothing recorded yet, budget is healthy.
    assert_eq!(tracker.check("opencode-go"), BudgetVerdict::WithinBudget);

    // Feed a real CompletionEvent: $0.40 to opencode-go. Below the $0.50
    // cap so verdict stays WithinBudget but `sub_cents` must update.
    orch.record_telemetry_for_test(vec![(
        "worker".to_string(),
        completion_event("opencode-go/minimax-m2.5", 0.40),
    )])
    .await;

    let snap_1 = tracker.snapshot();
    let entry_1 = snap_1
        .providers
        .get("opencode-go")
        .expect("provider state must exist after telemetry wire-up");
    assert_eq!(
        entry_1.hour.sub_cents, 4_000,
        "record_telemetry must route $0.40 into the hour bucket"
    );
    assert_eq!(entry_1.day.sub_cents, 4_000);

    // Feed another event that should push the hour cap over; the
    // routing-side `check()` must now observe Exhausted.
    orch.record_telemetry_for_test(vec![(
        "worker".to_string(),
        completion_event("opencode-go/minimax-m2.5", 0.30),
    )])
    .await;

    assert!(
        matches!(
            tracker.check("opencode-go"),
            BudgetVerdict::Exhausted { .. }
        ),
        "second record must tip the hour cap to Exhausted; snapshot={:?}",
        tracker.snapshot()
    );
}

#[tokio::test]
async fn record_telemetry_ignores_zero_cost_and_unknown_model() {
    // zero-cost events must be a no-op; unknown bare models silently
    // feed their own synthetic key (rather than panicking or mis-routing).
    let providers = vec![ProviderBudgetConfig {
        id: "kimi-for-coding".to_string(),
        max_hour_cents: Some(100),
        max_day_cents: None,
        error_signatures: None,
    }];
    let config = budget_aware_config(
        providers,
        None,
        vec![agent_with_model("worker", "kimi-for-coding/k2p5")],
    );
    let orch = AgentOrchestrator::new(config).expect("build orchestrator");
    let tracker = orch
        .provider_budget_tracker()
        .cloned()
        .expect("tracker must be constructed");

    // Zero cost event -- no spend recorded.
    orch.record_telemetry_for_test(vec![(
        "worker".to_string(),
        completion_event("kimi-for-coding/k2p5", 0.0),
    )])
    .await;
    assert_eq!(
        tracker
            .snapshot()
            .providers
            .get("kimi-for-coding")
            .map(|e| e.hour.sub_cents)
            .unwrap_or(0),
        0
    );

    // Model whose provider is not in [[providers]] -- should not
    // poison the tracker state for providers we *do* track.
    orch.record_telemetry_for_test(vec![(
        "worker".to_string(),
        completion_event("opencode-go/minimax-m2.5", 0.25),
    )])
    .await;
    assert_eq!(
        tracker
            .snapshot()
            .providers
            .get("kimi-for-coding")
            .map(|e| e.hour.sub_cents)
            .unwrap_or(0),
        0,
        "unrelated provider spend must not land in kimi-for-coding's bucket"
    );
}

// === Scenario 9: persistence across a simulated restart ==================

#[tokio::test]
async fn provider_budget_persistence_round_trip_via_orchestrator() {
    // Drive the tracker through the orchestrator, persist, then build
    // a new orchestrator from the same config + state file and verify
    // the counters carry across the "restart".
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let state_path = tmp.path().to_path_buf();
    drop(tmp);

    let providers = vec![ProviderBudgetConfig {
        id: "opencode-go".to_string(),
        max_hour_cents: Some(500),
        max_day_cents: Some(2_000),
        error_signatures: None,
    }];

    // Session 1: spend + persist.
    {
        let config = budget_aware_config(
            providers.clone(),
            Some(state_path.clone()),
            vec![agent_with_model("worker", "opencode-go/minimax-m2.5")],
        );
        let orch = AgentOrchestrator::new(config).expect("build orchestrator");
        orch.record_telemetry_for_test(vec![(
            "worker".to_string(),
            completion_event("opencode-go/minimax-m2.5", 1.23),
        )])
        .await;
        // Explicit persist to mimic the reconcile-tick flush.
        orch.provider_budget_tracker()
            .expect("tracker")
            .persist()
            .expect("persist must succeed");
    }

    // Session 2: rebuild from config + state; counters must survive.
    let config2 = budget_aware_config(
        providers,
        Some(state_path.clone()),
        vec![agent_with_model("worker", "opencode-go/minimax-m2.5")],
    );
    let orch2 = AgentOrchestrator::new(config2).expect("rebuild orchestrator");
    let snap = orch2
        .provider_budget_tracker()
        .expect("tracker present on restart")
        .snapshot();
    let entry = snap
        .providers
        .get("opencode-go")
        .expect("state must be reloaded");
    assert_eq!(
        entry.hour.sub_cents, 12_300,
        "hour bucket must survive restart"
    );
    assert_eq!(
        entry.day.sub_cents, 12_300,
        "day bucket must survive restart"
    );

    let _ = std::fs::remove_file(&state_path);
}

// === Helper: provider_key_for_model edges =================================

#[test]
fn provider_key_helper_classifies_bare_and_prefixed() {
    assert_eq!(
        provider_key_for_model("opencode-go/minimax-m2.5"),
        Some("opencode-go")
    );
    assert_eq!(
        provider_key_for_model("kimi-for-coding/k2p5"),
        Some("kimi-for-coding")
    );
    assert_eq!(provider_key_for_model("sonnet"), Some("claude-code"));
    assert_eq!(provider_key_for_model("opus"), Some("claude-code"));
    assert_eq!(provider_key_for_model("anthropic"), Some("claude-code"));
    // Unknown bare identifier -> echoed back as its own key.
    assert_eq!(provider_key_for_model("mystery"), Some("mystery"));
}
