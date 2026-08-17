//! Regenerate the golden serialisation vectors in `tests/golden/`.
//!
//! Run from the crate root: `cargo run --example generate_golden`.
//! Only run this when a wire-format change is intentional; the fixtures are
//! the pinned contract for consumer compatibility jobs.

use terraphim_engine_events::{
    Disposition, EngineEvent, EvolutionApplied, EvolutionApprove, EvolutionPropose,
    EvolutionReject, TargetKind, TrustLevel,
};

fn main() {
    let approve = EvolutionApprove {
        signature: "tools-use-rg-for-search".to_string(),
        target_kind: TargetKind::Tool,
        target_ref: Some("config/tools.toml".to_string()),
        trust_level: TrustLevel::L3,
        disposition: Disposition::AllowAlways,
    };
    let events = [
        (
            "evo_propose",
            EngineEvent::EvolutionProposed(EvolutionPropose {
                signature: "tools-use-rg-for-search".to_string(),
                target_kind: TargetKind::Tool,
                target_ref: Some("config/tools.toml".to_string()),
                content: "[search]\npreferred = \"rg\"".to_string(),
                trust_level: TrustLevel::L1,
            }),
        ),
        (
            "evo_approve",
            EngineEvent::EvolutionApproved(approve.clone()),
        ),
        (
            "evo_reject",
            EngineEvent::EvolutionRejected(EvolutionReject {
                signature: "behaviour-auto-approve-prs".to_string(),
                target_kind: TargetKind::Behaviour,
                target_ref: Some("rules/review.md".to_string()),
                trust_level: TrustLevel::L2,
                disposition: Disposition::RejectAlways,
            }),
        ),
        (
            "evo_applied",
            EngineEvent::EvolutionApplied(EvolutionApplied::from_approval(
                &approve,
                Some("rg --version && grep -q 'preferred = \"rg\"' config/tools.toml".to_string()),
                "evolution-log/2026-08-16.md#tools-use-rg-for-search".to_string(),
            )),
        ),
    ];
    for (name, event) in events {
        let json = serde_json::to_string_pretty(&event).expect("serialise");
        std::fs::write(format!("tests/golden/{name}.json"), format!("{json}\n"))
            .expect("write fixture");
        println!("wrote tests/golden/{name}.json");
    }
}
