//! Serde round-trip and type-system enforcement tests for the `evo.*`
//! evolution lifecycle events (TACP spec 5.1). No mocks: real serialisation
//! only.

use terraphim_engine_events::{
    Disposition, EVO_APPLIED_TYPE, EVO_APPROVE_TYPE, EVO_PROPOSE_TYPE, EVO_REJECT_TYPE,
    EngineEvent, EvolutionApplied, EvolutionApprove, EvolutionPropose, EvolutionReject, TargetKind,
    TrustLevel,
};

fn sample_propose() -> EvolutionPropose {
    EvolutionPropose {
        signature: "tools-use-rg-for-search".to_string(),
        target_kind: TargetKind::Tool,
        target_ref: Some("config/tools.toml".to_string()),
        content: "[search]\npreferred = \"rg\"".to_string(),
        trust_level: TrustLevel::L1,
    }
}

fn sample_approve() -> EvolutionApprove {
    EvolutionApprove {
        signature: "tools-use-rg-for-search".to_string(),
        target_kind: TargetKind::Tool,
        target_ref: Some("config/tools.toml".to_string()),
        trust_level: TrustLevel::L3,
        disposition: Disposition::AllowAlways,
    }
}

fn sample_reject() -> EvolutionReject {
    EvolutionReject {
        signature: "behaviour-auto-approve-prs".to_string(),
        target_kind: TargetKind::Behaviour,
        target_ref: Some("rules/review.md".to_string()),
        trust_level: TrustLevel::L2,
        disposition: Disposition::RejectAlways,
    }
}

fn sample_applied() -> EvolutionApplied {
    EvolutionApplied::from_approval(
        &sample_approve(),
        Some("rg --version && grep -q 'preferred = \"rg\"' config/tools.toml".to_string()),
        "evolution-log/2026-08-16.md#tools-use-rg-for-search".to_string(),
    )
}

fn round_trip(event: &EngineEvent) -> EngineEvent {
    let json = serde_json::to_string(event).expect("serialise");
    serde_json::from_str(&json).expect("deserialise")
}

#[test]
fn round_trip_all_four_variants() {
    let events = vec![
        EngineEvent::EvolutionProposed(sample_propose()),
        EngineEvent::EvolutionApproved(sample_approve()),
        EngineEvent::EvolutionRejected(sample_reject()),
        EngineEvent::EvolutionApplied(sample_applied()),
    ];
    for event in &events {
        assert_eq!(&round_trip(event), event);
    }
}

#[test]
fn wire_names_match_tacp_spec_exactly() {
    assert_eq!(
        EngineEvent::EvolutionProposed(sample_propose()).message_type(),
        EVO_PROPOSE_TYPE
    );
    assert_eq!(
        EngineEvent::EvolutionApproved(sample_approve()).message_type(),
        EVO_APPROVE_TYPE
    );
    assert_eq!(
        EngineEvent::EvolutionRejected(sample_reject()).message_type(),
        EVO_REJECT_TYPE
    );
    assert_eq!(
        EngineEvent::EvolutionApplied(sample_applied()).message_type(),
        EVO_APPLIED_TYPE
    );

    // The tag on the wire must be the dotted TACP name, not the Rust variant.
    let json = serde_json::to_string(&EngineEvent::EvolutionProposed(sample_propose()))
        .expect("serialise");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(value["type"], "evo.propose");
}

#[test]
fn trust_level_is_reused_from_terraphim_types() {
    // The shared-learning trust ladder type is re-used, not redeclared:
    // a `terraphim_types` value must be directly assignable.
    let trust: terraphim_types::shared_learning::TrustLevel = TrustLevel::L3;
    let propose = EvolutionPropose {
        trust_level: trust,
        ..sample_propose()
    };
    assert_eq!(propose.trust_level, TrustLevel::L3);
    // Wire form is the spec's `L0`-`L3` ladder.
    assert_eq!(serde_json::to_string(&TrustLevel::L2).unwrap(), "\"L2\"");
}

#[test]
fn behaviour_governing_flag_supports_spec_constraint_2() {
    let mut propose = sample_propose();
    assert!(!propose.is_behaviour_governing());
    propose.target_kind = TargetKind::Behaviour;
    assert!(propose.is_behaviour_governing());
}

#[test]
fn evolution_applied_requires_an_approval_reference() {
    // The only constructor takes `&EvolutionApprove`; the identity fields of
    // the applied event must match the approval it was built from (spec 5.1
    // constraint 3). A compile-time proof that direct construction is
    // impossible lives in the `compile_fail` doctest on `EvolutionApplied`.
    let approval = sample_approve();
    let applied = EvolutionApplied::from_approval(
        &approval,
        None,
        "evolution-log/2026-08-16.md#entry".to_string(),
    );
    assert_eq!(applied.signature(), approval.signature);
    assert_eq!(applied.target_kind(), approval.target_kind);
    assert_eq!(applied.target_ref(), approval.target_ref.as_deref());
    assert_eq!(applied.trust_level(), approval.trust_level);
    assert_eq!(applied.verify(), None);
    assert_eq!(applied.audit_ref(), "evolution-log/2026-08-16.md#entry");
}

#[test]
fn disposition_wire_names_match_spec() {
    assert_eq!(
        serde_json::to_string(&Disposition::AllowOnce).unwrap(),
        "\"allow_once\""
    );
    assert_eq!(
        serde_json::to_string(&Disposition::AllowAlways).unwrap(),
        "\"allow_always\""
    );
    assert_eq!(
        serde_json::to_string(&Disposition::Reject).unwrap(),
        "\"reject\""
    );
    assert_eq!(
        serde_json::to_string(&Disposition::RejectAlways).unwrap(),
        "\"reject_always\""
    );
}

#[test]
fn target_kind_wire_names_match_spec() {
    assert_eq!(
        serde_json::to_string(&TargetKind::Memory).unwrap(),
        "\"memory\""
    );
    assert_eq!(
        serde_json::to_string(&TargetKind::Behaviour).unwrap(),
        "\"behaviour\""
    );
    assert_eq!(
        serde_json::to_string(&TargetKind::Skill).unwrap(),
        "\"skill\""
    );
    assert_eq!(
        serde_json::to_string(&TargetKind::Tool).unwrap(),
        "\"tool\""
    );
}
