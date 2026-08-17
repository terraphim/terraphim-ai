//! Golden serialisation vectors for the `evo.*` family.
//!
//! The JSON fixtures in `tests/golden/` are the pinned contract: consumer
//! compatibility jobs (Zed, VS Code) diff against them, so any change to a
//! fixture is a wire-format change and must be treated as such.

use terraphim_engine_events::{
    Disposition, EngineEvent, EvolutionApplied, EvolutionApprove, EvolutionPropose,
    EvolutionReject, TargetKind, TrustLevel,
};

fn propose_event() -> EngineEvent {
    EngineEvent::EvolutionProposed(EvolutionPropose {
        signature: "tools-use-rg-for-search".to_string(),
        target_kind: TargetKind::Tool,
        target_ref: Some("config/tools.toml".to_string()),
        content: "[search]\npreferred = \"rg\"".to_string(),
        trust_level: TrustLevel::L1,
    })
}

fn approve_event() -> EngineEvent {
    EngineEvent::EvolutionApproved(EvolutionApprove {
        signature: "tools-use-rg-for-search".to_string(),
        target_kind: TargetKind::Tool,
        target_ref: Some("config/tools.toml".to_string()),
        trust_level: TrustLevel::L3,
        disposition: Disposition::AllowAlways,
    })
}

fn reject_event() -> EngineEvent {
    EngineEvent::EvolutionRejected(EvolutionReject {
        signature: "behaviour-auto-approve-prs".to_string(),
        target_kind: TargetKind::Behaviour,
        target_ref: Some("rules/review.md".to_string()),
        trust_level: TrustLevel::L2,
        disposition: Disposition::RejectAlways,
    })
}

fn applied_event() -> EngineEvent {
    let EngineEvent::EvolutionApproved(approval) = approve_event() else {
        unreachable!("approve_event is an EvolutionApproved variant");
    };
    EngineEvent::EvolutionApplied(EvolutionApplied::from_approval(
        &approval,
        Some("rg --version && grep -q 'preferred = \"rg\"' config/tools.toml".to_string()),
        "evolution-log/2026-08-16.md#tools-use-rg-for-search".to_string(),
    ))
}

fn assert_matches_golden(event: &EngineEvent, name: &str, golden: &str) {
    let serialised = serde_json::to_string_pretty(event).expect("serialise");
    assert_eq!(
        serialised,
        golden.trim_end(),
        "serialised event drifted from golden vector {name}"
    );
    // The fixture must also deserialise back to the same event.
    let parsed: EngineEvent = serde_json::from_str(golden).expect("parse golden fixture");
    assert_eq!(&parsed, event, "golden vector {name} does not round-trip");
}

#[test]
fn evo_propose_matches_golden_vector() {
    assert_matches_golden(
        &propose_event(),
        "evo_propose",
        include_str!("golden/evo_propose.json"),
    );
}

#[test]
fn evo_approve_matches_golden_vector() {
    assert_matches_golden(
        &approve_event(),
        "evo_approve",
        include_str!("golden/evo_approve.json"),
    );
}

#[test]
fn evo_reject_matches_golden_vector() {
    assert_matches_golden(
        &reject_event(),
        "evo_reject",
        include_str!("golden/evo_reject.json"),
    );
}

#[test]
fn evo_applied_matches_golden_vector() {
    assert_matches_golden(
        &applied_event(),
        "evo_applied",
        include_str!("golden/evo_applied.json"),
    );
}
