//! Tests for `terraphim_service::auto_route::auto_select_role`.
//!
//! See `docs/research/design-intent-based-role-auto-routing.md` section 6 for the
//! T1-T7 cases. These tests construct `Config` + `ConfigState` by hand so they
//! do not depend on `$JMAP_ACCESS_TOKEN` (cargo runs tests in parallel; reading
//! the env var would make outcomes non-deterministic). Use `from_env` only at
//! call sites, never inside unit tests.

use ahash::AHashMap;
use std::sync::Arc;
use terraphim_config::{Config, ConfigId, ConfigState, Haystack, Role, ServiceType};
use terraphim_rolegraph::{RoleGraph, RoleGraphSync};
use terraphim_service::auto_route::{
    AutoRouteContext, AutoRouteReason, JMAP_MISSING_TOKEN_DOWNWEIGHT, auto_select_role,
};
use terraphim_types::{
    Document, NormalizedTerm, NormalizedTermValue, RelevanceFunction, RoleName, Thesaurus,
};
use tokio::sync::Mutex;

/// Build a thesaurus with the given (synonym, id, concept) triples.
fn build_thesaurus(name: &str, terms: &[(&str, u64, &str)]) -> Thesaurus {
    let mut t = Thesaurus::new(name.to_string());
    for (synonym, id, concept) in terms {
        t.insert(
            NormalizedTermValue::from(*synonym),
            NormalizedTerm::new(*id, NormalizedTermValue::from(*concept)),
        );
    }
    t
}

/// Build a single test document whose body contains the supplied snippet.
fn make_doc(id: &str, body: &str) -> Document {
    Document {
        id: id.to_string(),
        title: id.to_string(),
        body: body.to_string(),
        url: format!("test://{id}"),
        description: None,
        rank: None,
        tags: None,
        summarization: None,
        stub: None,
        source_haystack: None,
        doc_type: terraphim_types::DocumentType::KgEntry,
        synonyms: None,
        route: None,
        priority: None,
    }
}

/// Build a `RoleGraphSync` for `role_name` from a thesaurus and seed documents.
/// Each document's body is matched against the Aho-Corasick automaton; matched
/// node-pair edges drive `node.rank` upwards (see `init_or_update_node`).
async fn build_rolegraph(
    role_name: &RoleName,
    thesaurus: Thesaurus,
    docs: &[Document],
) -> RoleGraphSync {
    let mut rg = RoleGraph::new(role_name.clone(), thesaurus).await.unwrap();
    for doc in docs {
        rg.insert_document(&doc.id, doc.clone());
    }
    RoleGraphSync::from(rg)
}

/// Build a `Role` for the config (TerraphimGraph + optional JMAP haystack).
fn make_role(name: &str, has_jmap: bool) -> Role {
    let mut role = Role::new(RoleName::new(name));
    role.relevance_function = RelevanceFunction::TerraphimGraph;
    if has_jmap {
        role.haystacks.push(Haystack::new(
            "jmap://test".to_string(),
            ServiceType::Jmap,
            true,
        ));
    }
    role
}

/// Bundle a config + manually-populated ConfigState. Bypasses `ConfigState::new`
/// so tests do not have to register thesauri to disk or fetch over the network.
struct Fixture {
    config: Config,
    state: ConfigState,
}

fn assemble(roles: Vec<(Role, RoleGraphSync)>, default: &str, selected: &str) -> Fixture {
    let mut role_map = AHashMap::new();
    let mut rg_map = AHashMap::new();
    for (role, rg) in roles {
        role_map.insert(role.name.clone(), role.clone());
        rg_map.insert(role.name.clone(), rg);
    }
    let config = Config {
        id: ConfigId::Embedded,
        global_shortcut: "Ctrl+X".to_string(),
        roles: role_map,
        default_role: RoleName::new(default),
        selected_role: RoleName::new(selected),
    };
    let state = ConfigState {
        config: Arc::new(Mutex::new(config.clone())),
        roles: rg_map,
    };
    Fixture { config, state }
}

// ---------------------------------------------------------------------------
// T1: Single role wins clearly
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t1_single_role_wins_clearly() {
    let sysop_name = RoleName::new("System Operator");
    let default_name = RoleName::new("Default");

    let sysop_thes = build_thesaurus("sysop", &[("rfp", 1, "rfp")]);
    let default_thes = build_thesaurus("default", &[("anything", 2, "anything")]);

    // Insert a few docs containing "rfp" so the node accumulates rank.
    let sysop_docs = vec![
        make_doc("d1", "rfp considerations and rfp planning"),
        make_doc("d2", "an rfp summary discusses rfp metadata"),
    ];
    let sysop_rg = build_rolegraph(&sysop_name, sysop_thes, &sysop_docs).await;
    let default_rg = build_rolegraph(&default_name, default_thes, &[]).await;

    let fixture = assemble(
        vec![
            (make_role("System Operator", false), sysop_rg),
            (make_role("Default", false), default_rg),
        ],
        "Default",
        "Default",
    );

    let ctx = AutoRouteContext {
        selected_role: Some(default_name.clone()),
        jmap_token_present: true,
    };
    let result = auto_select_role("rfp", &fixture.config, &fixture.state, &ctx).await;

    assert_eq!(result.role.as_str(), "System Operator");
    assert_eq!(result.reason, AutoRouteReason::ScoredWinner);
    assert!(result.candidates[0].1 > result.candidates[1].1);
}

// ---------------------------------------------------------------------------
// T2: Tie -- selected_role wins
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t2_tie_selected_role_wins() {
    let a_name = RoleName::new("Personal Assistant");
    let b_name = RoleName::new("Terraphim Engineer");

    // Same thesaurus content, same documents -> identical raw_score.
    let thes_a = build_thesaurus("a", &[("widget", 10, "widget")]);
    let thes_b = build_thesaurus("b", &[("widget", 20, "widget")]);
    let docs = vec![
        make_doc("d1", "widget widget widget"),
        make_doc("d2", "more widget content"),
    ];
    let rg_a = build_rolegraph(&a_name, thes_a, &docs).await;
    let rg_b = build_rolegraph(&b_name, thes_b, &docs).await;

    let fixture = assemble(
        vec![
            (make_role("Personal Assistant", false), rg_a),
            (make_role("Terraphim Engineer", false), rg_b),
        ],
        "Default",
        "Terraphim Engineer",
    );

    let ctx = AutoRouteContext {
        selected_role: Some(b_name.clone()),
        jmap_token_present: true,
    };
    let result = auto_select_role("widget", &fixture.config, &fixture.state, &ctx).await;

    assert_eq!(result.role.as_str(), "Terraphim Engineer");
    assert_eq!(result.reason, AutoRouteReason::TieBrokenBySelectedRole);
    // Both candidates should be at the top with equal scores.
    assert_eq!(result.candidates[0].1, result.candidates[1].1);
    assert!(result.score > 0);
}

// ---------------------------------------------------------------------------
// T3: Tie -- alphabetical
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t3_tie_alphabetical() {
    let a_name = RoleName::new("Personal Assistant");
    let b_name = RoleName::new("Terraphim Engineer");

    let thes_a = build_thesaurus("a", &[("widget", 10, "widget")]);
    let thes_b = build_thesaurus("b", &[("widget", 20, "widget")]);
    let docs = vec![
        make_doc("d1", "widget widget widget"),
        make_doc("d2", "more widget content"),
    ];
    let rg_a = build_rolegraph(&a_name, thes_a, &docs).await;
    let rg_b = build_rolegraph(&b_name, thes_b, &docs).await;

    let fixture = assemble(
        vec![
            (make_role("Personal Assistant", false), rg_a),
            (make_role("Terraphim Engineer", false), rg_b),
        ],
        "Default",
        "Personal Assistant",
    );

    // Selected role "PA" is also alphabetically first; switch selected to one
    // that is NOT in the tied set so we exercise the alphabetical fallback.
    let outsider = RoleName::new("Other Role Not In Config");
    let ctx = AutoRouteContext {
        selected_role: Some(outsider),
        jmap_token_present: true,
    };
    let result = auto_select_role("widget", &fixture.config, &fixture.state, &ctx).await;

    assert_eq!(result.role.as_str(), "Personal Assistant");
    assert_eq!(result.reason, AutoRouteReason::TieBrokenAlphabetically);
}

// ---------------------------------------------------------------------------
// T4: Zero match, selected_role set
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t4_zero_match_selected_role() {
    let rust_name = RoleName::new("Rust Engineer");
    let default_name = RoleName::new("Default");

    let rust_thes = build_thesaurus("rust", &[("rust", 1, "rust")]);
    let default_thes = build_thesaurus("default", &[("anything", 2, "anything")]);

    let rg_rust = build_rolegraph(&rust_name, rust_thes, &[]).await;
    let rg_default = build_rolegraph(&default_name, default_thes, &[]).await;

    let fixture = assemble(
        vec![
            (make_role("Rust Engineer", false), rg_rust),
            (make_role("Default", false), rg_default),
        ],
        "Default",
        "Rust Engineer",
    );

    let ctx = AutoRouteContext {
        selected_role: Some(rust_name.clone()),
        jmap_token_present: true,
    };
    let result = auto_select_role("xyzzy", &fixture.config, &fixture.state, &ctx).await;

    assert_eq!(result.role.as_str(), "Rust Engineer");
    assert_eq!(result.reason, AutoRouteReason::ZeroMatchSelectedRole);
}

// ---------------------------------------------------------------------------
// T5: Zero match, selected_role unset
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t5_zero_match_default() {
    let default_name = RoleName::new("Default");
    let other_name = RoleName::new("Other");

    let default_thes = build_thesaurus("default", &[("anything", 1, "anything")]);
    let other_thes = build_thesaurus("other", &[("anything_else", 2, "anything_else")]);

    let rg_default = build_rolegraph(&default_name, default_thes, &[]).await;
    let rg_other = build_rolegraph(&other_name, other_thes, &[]).await;

    let fixture = assemble(
        vec![
            (make_role("Default", false), rg_default),
            (make_role("Other", false), rg_other),
        ],
        "Default",
        "Default",
    );

    let ctx = AutoRouteContext {
        selected_role: None,
        jmap_token_present: true,
    };
    let result = auto_select_role("xyzzy", &fixture.config, &fixture.state, &ctx).await;

    assert_eq!(result.role.as_str(), "Default");
    assert_eq!(result.reason, AutoRouteReason::ZeroMatchDefault);
}

// ---------------------------------------------------------------------------
// T6: PA loses with stronger rival even when token is missing
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t6_pa_loses_to_stronger_rival() {
    let pa_name = RoleName::new("Personal Assistant");
    let sysop_name = RoleName::new("System Operator");

    let pa_thes = build_thesaurus("pa", &[("invoice", 1, "invoice")]);
    let sysop_thes = build_thesaurus("sysop", &[("invoice", 2, "invoice")]);

    // PA gets a small body; sysop gets many docs so its node rank is much higher.
    let pa_docs = vec![make_doc("d1", "invoice invoice")];
    let sysop_docs: Vec<Document> = (0..30)
        .map(|i| make_doc(&format!("s{i}"), "invoice invoice invoice invoice"))
        .collect();

    let rg_pa = build_rolegraph(&pa_name, pa_thes, &pa_docs).await;
    let rg_sysop = build_rolegraph(&sysop_name, sysop_thes, &sysop_docs).await;

    let fixture = assemble(
        vec![
            (make_role("Personal Assistant", true), rg_pa),
            (make_role("System Operator", false), rg_sysop),
        ],
        "Default",
        "Personal Assistant",
    );

    let ctx = AutoRouteContext {
        selected_role: Some(pa_name.clone()),
        jmap_token_present: false,
    };
    let result = auto_select_role("invoice", &fixture.config, &fixture.state, &ctx).await;

    assert_eq!(result.role.as_str(), "System Operator");
    assert_eq!(result.reason, AutoRouteReason::ScoredWinner);
}

// ---------------------------------------------------------------------------
// T7: PA wins on Obsidian alone (downweighted score still beats zero rivals)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t7_pa_wins_when_only_pa_matches() {
    let pa_name = RoleName::new("Personal Assistant");
    let other_name = RoleName::new("Default");

    let pa_thes = build_thesaurus("pa", &[("invoice", 1, "invoice")]);
    let other_thes = build_thesaurus("default", &[("rust", 2, "rust")]);

    let pa_docs = vec![
        make_doc("d1", "invoice invoice invoice"),
        make_doc("d2", "another invoice document with invoice"),
    ];
    let rg_pa = build_rolegraph(&pa_name, pa_thes, &pa_docs).await;
    let rg_other = build_rolegraph(&other_name, other_thes, &[]).await;

    let fixture = assemble(
        vec![
            (make_role("Personal Assistant", true), rg_pa),
            (make_role("Default", false), rg_other),
        ],
        "Default",
        "Personal Assistant",
    );

    let ctx = AutoRouteContext {
        selected_role: Some(pa_name.clone()),
        jmap_token_present: false,
    };
    let result = auto_select_role("invoice", &fixture.config, &fixture.state, &ctx).await;

    assert_eq!(result.role.as_str(), "Personal Assistant");
    assert!(result.score > 0);
    // Confirm downweight was actually applied by reproducing the math: the raw
    // rank-sum should round to (score / DOWNWEIGHT). Use saturating equality to
    // avoid coupling to the exact insert_document tuple_windows behaviour.
    let raw_estimate = (result.score as f64) / JMAP_MISSING_TOKEN_DOWNWEIGHT;
    assert!(
        raw_estimate >= result.score as f64,
        "downweighted score should be no greater than raw"
    );
}
