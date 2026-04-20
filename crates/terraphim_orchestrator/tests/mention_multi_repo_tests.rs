//! Integration tests for multi-repo mention routing (issue #5).
//!
//! Covers:
//! - Extended `MENTION_RE` regex capturing optional `<project>/` prefix.
//! - Project-aware `resolve_mention` resolution rules.
//! - `parse_mentions` stamping `project_id` onto detected mentions.
//! - `MentionCursor` per-project key isolation at the API level.
//! - One-shot `migrate_legacy_mention_cursor` idempotency.

use terraphim_orchestrator::config::{AgentDefinition, AgentLayer, Project};
use terraphim_orchestrator::mention::{
    migrate_legacy_mention_cursor, parse_mention_tokens, parse_mentions, resolve_mention,
    MentionCursor,
};
use terraphim_orchestrator::persona::PersonaRegistry;
use terraphim_tracker::{CommentUser, IssueComment};

const LEGACY: &str = "__global__";

fn agent(name: &str, project: Option<&str>) -> AgentDefinition {
    AgentDefinition {
        name: name.to_string(),
        layer: AgentLayer::Growth,
        cli_tool: "echo".to_string(),
        task: "t".to_string(),
        schedule: None,
        model: None,
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
        project: project.map(|s| s.to_string()),
    }
}

fn comment(id: u64, body: &str) -> IssueComment {
    IssueComment {
        id,
        issue_number: 0,
        body: body.to_string(),
        user: CommentUser {
            login: "tester".to_string(),
        },
        created_at: "2026-04-19T00:00:00Z".to_string(),
        updated_at: "2026-04-19T00:00:00Z".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Regex: parse_mention_tokens
// ---------------------------------------------------------------------------

#[test]
fn regex_captures_unqualified_mention() {
    let tokens = parse_mention_tokens("hello @adf:developer please");
    assert_eq!(tokens.len(), 1);
    assert!(tokens[0].project.is_none());
    assert_eq!(tokens[0].agent, "developer");
}

#[test]
fn regex_captures_qualified_mention() {
    let tokens = parse_mention_tokens("hello @adf:odilo/developer please");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].project.as_deref(), Some("odilo"));
    assert_eq!(tokens[0].agent, "developer");
}

#[test]
fn regex_mixes_qualified_and_unqualified_in_one_comment() {
    let tokens =
        parse_mention_tokens("@adf:security @adf:odilo/reviewer and @adf:terraphim/cartoprist");
    let names: Vec<(Option<&str>, &str)> = tokens
        .iter()
        .map(|t| (t.project.as_deref(), t.agent.as_str()))
        .collect();
    assert_eq!(
        names,
        vec![
            (None, "security"),
            (Some("odilo"), "reviewer"),
            (Some("terraphim"), "cartoprist"),
        ]
    );
}

#[test]
fn regex_rejects_uppercase_project_prefix() {
    // Uppercase is not allowed in the project prefix and the agent name
    // also requires a lowercase start, so `@adf:Odilo/developer` produces
    // no tokens at all.
    let tokens = parse_mention_tokens("see @adf:Odilo/developer");
    assert!(
        tokens.is_empty(),
        "uppercase prefix must not be captured, got {tokens:?}"
    );
}

#[test]
fn regex_rejects_too_long_project_prefix() {
    // 41-char project prefix exceeds the {1,39} cap (min 2-char start + 39)
    let long = "a".repeat(41);
    let text = format!("@adf:{long}/dev");
    let tokens = parse_mention_tokens(&text);
    // Fallback behaviour: regex may still match `dev` unqualified — the
    // important assertion is that nothing is captured as a qualified
    // mention with the over-long project.
    for t in &tokens {
        assert_ne!(t.project.as_deref(), Some(long.as_str()));
    }
}

#[test]
fn regex_handles_trailing_punctuation() {
    let tokens = parse_mention_tokens("ping @adf:odilo/reviewer, thanks!");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].project.as_deref(), Some("odilo"));
    assert_eq!(tokens[0].agent, "reviewer");
}

#[test]
fn regex_ignores_plain_at_mentions() {
    let tokens = parse_mention_tokens("@alex please review @odilo/developer too");
    assert!(tokens.is_empty());
}

// ---------------------------------------------------------------------------
// resolve_mention: project-aware resolution
// ---------------------------------------------------------------------------

#[test]
fn resolve_mention_qualified_exact_match() {
    let agents = vec![
        agent("developer", Some("odilo")),
        agent("developer", Some("terraphim")),
    ];
    let resolved = resolve_mention(Some("odilo"), "terraphim", "developer", &agents).unwrap();
    assert_eq!(resolved.name, "developer");
    assert_eq!(resolved.project.as_deref(), Some("odilo"));
}

#[test]
fn resolve_mention_qualified_not_found_returns_none() {
    let agents = vec![agent("developer", Some("odilo"))];
    // Ask for a project the agent doesn't belong to.
    let resolved = resolve_mention(Some("terraphim"), "terraphim", "developer", &agents);
    assert!(resolved.is_none());
}

#[test]
fn resolve_mention_qualified_ambiguous_returns_none() {
    // Two agents with the same name AND project — should be impossible
    // at config-validation time, but resolver must still refuse.
    let agents = vec![
        agent("developer", Some("odilo")),
        agent("developer", Some("odilo")),
    ];
    let resolved = resolve_mention(Some("odilo"), LEGACY, "developer", &agents);
    assert!(resolved.is_none());
}

#[test]
fn resolve_mention_unqualified_legacy_matches_any() {
    // Legacy single-project mode: ignore agent's project field entirely.
    let agents = vec![agent("developer", None), agent("reviewer", Some("odilo"))];
    let resolved = resolve_mention(None, LEGACY, "reviewer", &agents).unwrap();
    assert_eq!(resolved.name, "reviewer");
    assert_eq!(resolved.project.as_deref(), Some("odilo"));
}

#[test]
fn resolve_mention_unqualified_prefers_hinted_project() {
    let agents = vec![
        agent("developer", Some("odilo")),
        agent("developer", Some("terraphim")),
    ];
    // Polling odilo's repo → the odilo developer wins.
    let resolved = resolve_mention(None, "odilo", "developer", &agents).unwrap();
    assert_eq!(resolved.project.as_deref(), Some("odilo"));

    // Polling terraphim's repo → the terraphim developer wins.
    let resolved = resolve_mention(None, "terraphim", "developer", &agents).unwrap();
    assert_eq!(resolved.project.as_deref(), Some("terraphim"));
}

#[test]
fn resolve_mention_unqualified_falls_back_to_unbound() {
    // No agent bound to the hinted project — fall back to a
    // project-less agent of the same name.
    let agents = vec![agent("developer", Some("odilo")), agent("floater", None)];
    let resolved = resolve_mention(None, "terraphim", "floater", &agents).unwrap();
    assert_eq!(resolved.name, "floater");
    assert!(resolved.project.is_none());
}

#[test]
fn resolve_mention_unqualified_ambiguous_hinted_returns_none() {
    // Two agents, same name, same hinted project → ambiguous.
    let agents = vec![
        agent("developer", Some("odilo")),
        agent("developer", Some("odilo")),
    ];
    let resolved = resolve_mention(None, "odilo", "developer", &agents);
    assert!(resolved.is_none());
}

#[test]
fn resolve_mention_unqualified_ambiguous_unbound_returns_none() {
    // No hinted match; two unbound agents with the same name → ambiguous.
    let agents = vec![agent("developer", None), agent("developer", None)];
    let resolved = resolve_mention(None, "odilo", "developer", &agents);
    assert!(resolved.is_none());
}

#[test]
fn resolve_mention_unqualified_no_match_returns_none() {
    let agents = vec![agent("developer", Some("odilo"))];
    let resolved = resolve_mention(None, "terraphim", "ghost", &agents);
    assert!(resolved.is_none());
}

// ---------------------------------------------------------------------------
// parse_mentions: project_id stamping
// ---------------------------------------------------------------------------

#[test]
fn parse_mentions_stamps_legacy_project_id() {
    let agents = vec![agent("developer", None)];
    let personas = PersonaRegistry::default();
    let c = comment(42, "@adf:developer please look");
    let mentions = parse_mentions(&c, 7, &agents, &personas, LEGACY);
    assert_eq!(mentions.len(), 1);
    assert_eq!(mentions[0].project_id, LEGACY);
    assert_eq!(mentions[0].agent_name, "developer");
}

#[test]
fn parse_mentions_stamps_hinted_project_id() {
    let agents = vec![agent("developer", Some("odilo"))];
    let personas = PersonaRegistry::default();
    let c = comment(43, "@adf:developer please look");
    let mentions = parse_mentions(&c, 8, &agents, &personas, "odilo");
    assert_eq!(mentions.len(), 1);
    assert_eq!(mentions[0].project_id, "odilo");
}

// ---------------------------------------------------------------------------
// MentionCursor: in-memory isolation (no persistence needed)
// ---------------------------------------------------------------------------

#[test]
fn cursor_per_project_isolation() {
    // Two cursors are independent structs — `mark_processed` on one
    // does not affect the other.
    let mut c_odilo = MentionCursor::now();
    let mut c_terra = MentionCursor::now();
    c_odilo.mark_processed(100);
    c_terra.mark_processed(200);
    assert!(c_odilo.is_processed(100));
    assert!(!c_odilo.is_processed(200));
    assert!(c_terra.is_processed(200));
    assert!(!c_terra.is_processed(100));
}

#[test]
fn cursor_advance_to_monotonic() {
    let mut c = MentionCursor {
        last_seen_at: "2026-04-19T10:00:00Z".to_string(),
        dispatches_this_tick: 0,
        processed_comment_ids: Default::default(),
    };
    // Older timestamp — should NOT regress.
    c.advance_to("2026-04-19T09:00:00Z");
    assert_eq!(c.last_seen_at, "2026-04-19T10:00:00Z");
    // Newer timestamp — should advance.
    c.advance_to("2026-04-19T11:00:00Z");
    assert_eq!(c.last_seen_at, "2026-04-19T11:00:00Z");
}

// ---------------------------------------------------------------------------
// migrate_legacy_mention_cursor: no-op idempotency in memory-only test env
// ---------------------------------------------------------------------------

#[tokio::test]
async fn migration_is_noop_without_sqlite_backend() {
    // In the test environment DeviceStorage uses the memory backend only;
    // the sqlite operator is absent, so migration must be a safe no-op
    // rather than panicking.
    let projects: Vec<Project> = vec![];
    migrate_legacy_mention_cursor(&projects).await;
    // Calling twice must remain a no-op.
    migrate_legacy_mention_cursor(&projects).await;
}

// ---------------------------------------------------------------------------
// Dispatch wiring: parse_mention_tokens + resolve_mention end-to-end
//
// These tests mirror the logic now wired into the poll and webhook dispatch
// paths. They verify the exact sequence the orchestrator executes for each
// comment body, ensuring qualified `@adf:project/name` mentions route to the
// correct project-scoped agent and unqualified mentions in multi-project mode
// prefer the hinted-project agent.
// ---------------------------------------------------------------------------

/// Simulates the qualified-mention pass added to `poll_mentions_for_project`.
/// A comment containing `@adf:odilo/developer` must route to the odilo-scoped
/// agent even when two `developer` agents exist in different projects.
#[test]
fn dispatch_wiring_qualified_mention_routes_to_correct_project() {
    let agents = vec![
        agent("developer", Some("odilo")),
        agent("developer", Some("terraphim")),
    ];

    let comment_body = "Please @adf:odilo/developer review this PR";
    let project_id = "terraphim"; // the poll is running for the terraphim project

    // This is the exact sequence poll_mentions_for_project now executes:
    // 1. parse_mention_tokens for qualified mentions
    // 2. For each qualified token, call resolve_mention with detected project
    let resolved: Vec<_> = parse_mention_tokens(comment_body)
        .into_iter()
        .filter(|t| t.project.is_some())
        .filter_map(|t| {
            let proj = t.project.as_deref();
            resolve_mention(proj, project_id, &t.agent, &agents)
        })
        .collect();

    assert_eq!(resolved.len(), 1, "exactly one agent should be resolved");
    assert_eq!(resolved[0].name, "developer");
    assert_eq!(
        resolved[0].project.as_deref(),
        Some("odilo"),
        "must resolve to the odilo-scoped developer, not the terraphim one"
    );
}

/// Simulates the unqualified-mention dispatch path (AdfCommandParser produces
/// `agent_name = "developer"`, no detected_project). In multi-project mode the
/// hinted project_id must select the matching agent.
#[test]
fn dispatch_wiring_unqualified_mention_prefers_hinted_project() {
    let agents = vec![
        agent("developer", Some("odilo")),
        agent("developer", Some("terraphim")),
    ];

    // The AdfCommandParser produces agent_name = "developer" with no project prefix.
    let agent_name = "developer";
    let project_id = "odilo"; // poll is running for odilo

    // This is the exact call now used at the SpawnAgent arm in poll_mentions_for_project.
    let resolved = resolve_mention(None, project_id, agent_name, &agents);

    assert!(
        resolved.is_some(),
        "should resolve the hinted-project agent"
    );
    assert_eq!(
        resolved.unwrap().project.as_deref(),
        Some("odilo"),
        "must select the odilo-scoped developer"
    );
}

/// Simulates the webhook dispatch path: qualified mention carried through
/// `WebhookDispatch::SpawnAgent.detected_project` is now resolved correctly.
#[test]
fn dispatch_wiring_webhook_qualified_mention_resolves_by_detected_project() {
    let agents = vec![
        agent("reviewer", Some("odilo")),
        agent("reviewer", Some("terraphim")),
    ];

    // webhook.rs now extracts detected_project via parse_mention_tokens
    let comment_body = "@adf:terraphim/reviewer please check";
    let detected_project = parse_mention_tokens(comment_body)
        .into_iter()
        .find(|t| t.agent == "reviewer")
        .and_then(|t| t.project);

    assert_eq!(
        detected_project.as_deref(),
        Some("terraphim"),
        "webhook handler must extract the project prefix"
    );

    // handle_webhook_dispatch now calls resolve_mention with detected_project
    // and LEGACY_PROJECT_ID as the hinted project (webhook has no repo hint).
    let resolved = resolve_mention(
        detected_project.as_deref(),
        "__global__",
        "reviewer",
        &agents,
    );

    assert!(resolved.is_some());
    assert_eq!(
        resolved.unwrap().project.as_deref(),
        Some("terraphim"),
        "webhook dispatch must resolve to terraphim-scoped reviewer"
    );
}
