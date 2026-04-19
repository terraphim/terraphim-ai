//! Tests for `terraphim_service::auto_route::auto_select_role`.
//!
//! See `docs/research/design-auto-route-cold-start.md` section 5 for the
//! T1'-T11' cases. These tests construct `Config` + `ConfigState` by hand so
//! they do not depend on `$JMAP_ACCESS_TOKEN` (cargo runs tests in parallel;
//! reading the env var would make outcomes non-deterministic). Use `from_env`
//! only at call sites, never inside unit tests.
//!
//! Scoring is "distinct canonical concept count" against the role's
//! thesaurus-driven Aho-Corasick automaton. No document seeding is required;
//! the prior `insert_document` loop has been dropped to exercise cold-start.

use ahash::AHashMap;
use std::sync::Arc;
use terraphim_config::{Config, ConfigId, ConfigState, Haystack, Role, ServiceType};
use terraphim_rolegraph::{RoleGraph, RoleGraphSync};
use terraphim_service::auto_route::{AutoRouteContext, AutoRouteReason, auto_select_role};
use terraphim_types::{
    NormalizedTerm, NormalizedTermValue, RelevanceFunction, RoleName, Thesaurus,
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

/// Build a `RoleGraphSync` for `role_name` from a thesaurus alone.
/// No document seeding -- routing now depends only on the prebuilt
/// Aho-Corasick automaton (`RoleGraph::new`), not on indexed documents.
async fn build_rolegraph(role_name: &RoleName, thesaurus: Thesaurus) -> RoleGraphSync {
    let rg = RoleGraph::new(role_name.clone(), thesaurus).await.unwrap();
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
// T1': Single role wins clearly
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t1_single_role_wins_clearly() {
    let sysop_name = RoleName::new("System Operator");
    let default_name = RoleName::new("Default");

    let sysop_thes = build_thesaurus(
        "sysop",
        &[("rfp", 1, "acquisition need"), ("acquisition", 1, "acquisition need")],
    );
    let default_thes = build_thesaurus("default", &[("anything", 2, "anything")]);

    let sysop_rg = build_rolegraph(&sysop_name, sysop_thes).await;
    let default_rg = build_rolegraph(&default_name, default_thes).await;

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
    let result = auto_select_role("RFP", &fixture.config, &fixture.state, &ctx).await;

    assert_eq!(result.role.as_str(), "System Operator");
    assert_eq!(result.score, 1);
    assert_eq!(result.reason, AutoRouteReason::ScoredWinner);
}

// ---------------------------------------------------------------------------
// T2': Tie -- selected_role wins
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t2_tie_selected_role_wins() {
    let a_name = RoleName::new("Personal Assistant");
    let b_name = RoleName::new("Terraphim Engineer");

    // Same single matching concept -> identical distinct-concept score (1).
    let thes_a = build_thesaurus("a", &[("widget", 10, "widget")]);
    let thes_b = build_thesaurus("b", &[("widget", 20, "widget")]);
    let rg_a = build_rolegraph(&a_name, thes_a).await;
    let rg_b = build_rolegraph(&b_name, thes_b).await;

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
    assert_eq!(result.candidates[0].1, result.candidates[1].1);
    assert_eq!(result.score, 1);
}

// ---------------------------------------------------------------------------
// T3': Tie -- alphabetical
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t3_tie_alphabetical() {
    let a_name = RoleName::new("Personal Assistant");
    let b_name = RoleName::new("Terraphim Engineer");

    let thes_a = build_thesaurus("a", &[("widget", 10, "widget")]);
    let thes_b = build_thesaurus("b", &[("widget", 20, "widget")]);
    let rg_a = build_rolegraph(&a_name, thes_a).await;
    let rg_b = build_rolegraph(&b_name, thes_b).await;

    let fixture = assemble(
        vec![
            (make_role("Personal Assistant", false), rg_a),
            (make_role("Terraphim Engineer", false), rg_b),
        ],
        "Default",
        "Personal Assistant",
    );

    // Selected role is NOT in the tied set -> alphabetical fallback.
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
// T4': Zero match, selected_role set
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t4_zero_match_selected_role() {
    let rust_name = RoleName::new("Rust Engineer");
    let default_name = RoleName::new("Default");

    let rust_thes = build_thesaurus("rust", &[("rust", 1, "rust")]);
    let default_thes = build_thesaurus("default", &[("anything", 2, "anything")]);

    let rg_rust = build_rolegraph(&rust_name, rust_thes).await;
    let rg_default = build_rolegraph(&default_name, default_thes).await;

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
    assert_eq!(result.score, 0);
}

// ---------------------------------------------------------------------------
// T5': Zero match, selected_role unset
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t5_zero_match_default() {
    let default_name = RoleName::new("Default");
    let other_name = RoleName::new("Other");

    let default_thes = build_thesaurus("default", &[("anything", 1, "anything")]);
    let other_thes = build_thesaurus("other", &[("anything_else", 2, "anything_else")]);

    let rg_default = build_rolegraph(&default_name, default_thes).await;
    let rg_other = build_rolegraph(&other_name, other_thes).await;

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
    assert_eq!(result.score, 0);
}

// ---------------------------------------------------------------------------
// T6': PA loses to a stronger rival under JMAP missing-token penalty
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t6_pa_loses_to_stronger_rival() {
    let pa_name = RoleName::new("Personal Assistant");
    let sysop_name = RoleName::new("System Operator");

    // PA matches one concept; sysop matches two distinct concepts.
    let pa_thes = build_thesaurus("pa", &[("invoice", 1, "invoice")]);
    let sysop_thes = build_thesaurus(
        "sysop",
        &[("invoice", 2, "invoice"), ("procurement", 3, "procurement")],
    );

    let rg_pa = build_rolegraph(&pa_name, pa_thes).await;
    let rg_sysop = build_rolegraph(&sysop_name, sysop_thes).await;

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
    let result =
        auto_select_role("invoice procurement", &fixture.config, &fixture.state, &ctx).await;

    // Raw: PA=1, sysop=2. After penalty: PA=0, sysop=2. Sysop wins outright.
    assert_eq!(result.role.as_str(), "System Operator");
    assert_eq!(result.score, 2);
    assert_eq!(result.reason, AutoRouteReason::ScoredWinner);
}

// ---------------------------------------------------------------------------
// T7': PA wins with sufficient evidence (penalty does not silence it)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t7_pa_wins_when_only_pa_matches() {
    let pa_name = RoleName::new("Personal Assistant");
    let other_name = RoleName::new("Default");

    // PA matches two distinct concepts; rival matches none.
    let pa_thes = build_thesaurus(
        "pa",
        &[("invoice", 1, "invoice"), ("receipt", 2, "receipt")],
    );
    let other_thes = build_thesaurus("default", &[("rust", 3, "rust")]);

    let rg_pa = build_rolegraph(&pa_name, pa_thes).await;
    let rg_other = build_rolegraph(&other_name, other_thes).await;

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
    let result =
        auto_select_role("invoice and receipt", &fixture.config, &fixture.state, &ctx).await;

    // Raw PA=2, after penalty=1. Rival raw=0. PA still wins.
    assert_eq!(result.role.as_str(), "Personal Assistant");
    assert_eq!(result.score, 1);
    assert_eq!(result.reason, AutoRouteReason::ScoredWinner);
}

// ---------------------------------------------------------------------------
// T11': Cold-start regression -- the headline test for #617.
//
// Reproduces the production cold-start scenario: a `Config` shaped like
// `~/.config/terraphim/embedded_config.json` with a "System Operator" role
// whose thesaurus maps `rfp -> rfp`, but with `RoleGraph::new` called on
// thesaurus only -- `insert_document` is NEVER called. Against the prior
// rank-sum scorer this returned score=0 across all roles and Default won by
// fallback; against the new distinct-concept scorer it must return
// "System Operator" with score >= 1.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t11_cold_start_no_documents_indexed() {
    let sysop_name = RoleName::new("System Operator");
    let default_name = RoleName::new("Default");
    let engineer_name = RoleName::new("Terraphim Engineer");

    let sysop_thes = build_thesaurus("sysop", &[("rfp", 1, "rfp")]);
    let default_thes = build_thesaurus("default", &[("readme", 2, "readme")]);
    let engineer_thes = build_thesaurus("engineer", &[("crate", 3, "crate")]);

    // Cold start: build rolegraphs from thesaurus only -- no insert_document.
    let sysop_rg = build_rolegraph(&sysop_name, sysop_thes).await;
    let default_rg = build_rolegraph(&default_name, default_thes).await;
    let engineer_rg = build_rolegraph(&engineer_name, engineer_thes).await;

    let fixture = assemble(
        vec![
            (make_role("System Operator", false), sysop_rg),
            (make_role("Default", false), default_rg),
            (make_role("Terraphim Engineer", false), engineer_rg),
        ],
        "Default",
        "Default",
    );

    // No --role override; selected_role is "Default" (matches embedded_config).
    let ctx = AutoRouteContext {
        selected_role: Some(default_name.clone()),
        jmap_token_present: true,
    };
    let result = auto_select_role("RFP", &fixture.config, &fixture.state, &ctx).await;

    assert_eq!(
        result.role.as_str(),
        "System Operator",
        "cold-start: System Operator must win on 'RFP' without document indexing"
    );
    assert!(
        result.score >= 1,
        "cold-start: score must be >= 1 (was {}). Prior rank-sum scorer would have returned 0.",
        result.score
    );
    assert_eq!(result.reason, AutoRouteReason::ScoredWinner);
}
