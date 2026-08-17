//! Approval-gated application of TinyClaw evolution proposals.
//!
//! `evo.propose` is only a durable proposal. This module is the application
//! boundary for #3229: callers must provide the matching `evo.approve` payload,
//! and successful application appends an `evo.applied` event to the audit log.
//! Rejections and unsupported target kinds are audited but do not mutate files.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use terraphim_engine_events::{
    Disposition, EngineEvent, EvolutionApplied, EvolutionApprove, EvolutionPropose, TargetKind,
    TrustLevel,
};

use crate::commands::CommandRegistry;

/// Outcome of applying (or deliberately not applying) an evolution proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvolutionApplyOutcome {
    /// The proposal was applied and an `evo.applied` event was written.
    Applied {
        audit_ref: String,
        target_path: PathBuf,
    },
    /// The proposal was approved for the lifecycle, but this implementation
    /// cannot safely mutate that target kind yet.
    Deferred { audit_ref: String, reason: String },
    /// The approval disposition rejects this proposal; no mutation happened.
    Rejected { audit_ref: String, reason: String },
}

/// Apply an evolution proposal only after a matching approval.
///
/// Behaviour proposals are written through [`CommandRegistry`]'s sanctioned
/// validated writer, then the registry is reloaded. Memory/tool preference
/// proposals are appended as correction records. Skill proposals are deferred
/// until ADR-0009 `SKILL.md` loading is available, because wholesale skill-file
/// writes would recreate the AutoClaw anti-pattern.
pub fn apply_approved_proposal(
    workspace: &Path,
    registry: &mut CommandRegistry,
    proposal: &EvolutionPropose,
    approval: &EvolutionApprove,
) -> Result<EvolutionApplyOutcome> {
    validate_matching_approval(proposal, approval)?;

    if matches!(
        approval.disposition,
        Disposition::Reject | Disposition::RejectAlways
    ) {
        let audit_ref = append_audit_record(
            workspace,
            "evo.reject",
            proposal,
            approval,
            "approval disposition rejected; no files mutated",
        )?;
        return Ok(EvolutionApplyOutcome::Rejected {
            audit_ref,
            reason: "approval disposition rejected".to_string(),
        });
    }

    match proposal.target_kind {
        TargetKind::Behaviour => apply_behaviour(workspace, registry, proposal, approval),
        TargetKind::Memory | TargetKind::Tool => apply_preference(workspace, proposal, approval),
        TargetKind::Skill => defer_skill(workspace, proposal, approval),
    }
}

fn stable_text_eq(left: &str, right: &str) -> bool {
    left.len() == right.len() && left.bytes().zip(right.bytes()).all(|(a, b)| a == b)
}

fn validate_matching_approval(
    proposal: &EvolutionPropose,
    approval: &EvolutionApprove,
) -> Result<()> {
    anyhow::ensure!(
        stable_text_eq(&proposal.signature, &approval.signature)
            && proposal.target_kind == approval.target_kind
            && proposal.target_ref == approval.target_ref
            && proposal.trust_level == approval.trust_level,
        "evo.approve does not match evo.propose identity fields"
    );

    if proposal.is_behaviour_governing() {
        anyhow::ensure!(
            matches!(proposal.trust_level, TrustLevel::L3),
            "behaviour-governing evolution requires L3 approval"
        );
    }

    Ok(())
}

fn apply_behaviour(
    workspace: &Path,
    registry: &mut CommandRegistry,
    proposal: &EvolutionPropose,
    approval: &EvolutionApprove,
) -> Result<EvolutionApplyOutcome> {
    let target_ref = proposal
        .target_ref
        .as_deref()
        .context("behaviour proposal requires target_ref command name")?;
    let commands_dir = workspace.join("commands");
    let target_path = registry
        .write_validated_command_section(&commands_dir, target_ref, &proposal.content)
        .context("failed to write validated command proposal")?;
    registry
        .load_from_dir(&commands_dir)
        .context("failed to reload command registry after evolution apply")?;

    let audit_ref = append_applied_event(
        workspace,
        approval,
        Some(format!("command:{}", target_ref)),
        &format!(
            "behaviour command section merged into {}",
            target_path.display()
        ),
    )?;
    Ok(EvolutionApplyOutcome::Applied {
        audit_ref,
        target_path,
    })
}

fn apply_preference(
    workspace: &Path,
    proposal: &EvolutionPropose,
    approval: &EvolutionApprove,
) -> Result<EvolutionApplyOutcome> {
    let dir = workspace.join(".terraphim").join("evolution");
    std::fs::create_dir_all(&dir)?;
    let target_path = dir.join("corrections.md");
    append_section_scoped_markdown(
        &target_path,
        &proposal.signature,
        &format!(
            "- target_kind: {:?}\n- target_ref: {}\n\n{}\n",
            proposal.target_kind,
            proposal.target_ref.as_deref().unwrap_or("(none)"),
            proposal.content.trim()
        ),
    )?;

    let audit_ref = append_applied_event(
        workspace,
        approval,
        Some("corrections.md contains signature section".to_string()),
        &format!(
            "preference/correction section merged into {}",
            target_path.display()
        ),
    )?;
    Ok(EvolutionApplyOutcome::Applied {
        audit_ref,
        target_path,
    })
}

fn defer_skill(
    workspace: &Path,
    proposal: &EvolutionPropose,
    approval: &EvolutionApprove,
) -> Result<EvolutionApplyOutcome> {
    let reason = "skill evolution deferred until ADR-0009 SKILL.md loading lands";
    let audit_ref = append_audit_record(workspace, "evo.defer", proposal, approval, reason)?;
    Ok(EvolutionApplyOutcome::Deferred {
        audit_ref,
        reason: reason.to_string(),
    })
}

fn append_applied_event(
    workspace: &Path,
    approval: &EvolutionApprove,
    verify: Option<String>,
    note: &str,
) -> Result<String> {
    let audit_ref = audit_log_ref(workspace);
    let applied = EvolutionApplied::from_approval(approval, verify, audit_ref.clone());
    append_engine_event(workspace, &EngineEvent::EvolutionApplied(applied), note)?;
    Ok(audit_ref)
}

fn append_audit_record(
    workspace: &Path,
    event_type: &str,
    proposal: &EvolutionPropose,
    approval: &EvolutionApprove,
    note: &str,
) -> Result<String> {
    let payload = serde_json::json!({
        "type": event_type,
        "proposal": proposal,
        "approval": approval,
        "note": note,
    });
    append_jsonl(workspace, &payload)
}

fn append_engine_event(workspace: &Path, event: &EngineEvent, note: &str) -> Result<String> {
    let payload = serde_json::json!({
        "event": event,
        "note": note,
    });
    append_jsonl(workspace, &payload)
}

fn append_jsonl(workspace: &Path, payload: &serde_json::Value) -> Result<String> {
    use std::io::Write;
    let dir = workspace.join(".terraphim").join("evolution");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("audit.jsonl");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    let line = serde_json::to_string(payload)?;
    writeln!(file, "{line}")?;
    Ok(audit_log_ref(workspace))
}

fn audit_log_ref(workspace: &Path) -> String {
    format!(
        "{}#append",
        workspace
            .join(".terraphim")
            .join("evolution")
            .join("audit.jsonl")
            .display()
    )
}

fn append_section_scoped_markdown(path: &Path, signature: &str, body: &str) -> Result<()> {
    let marker = format!("## {}", signature);
    let mut content = std::fs::read_to_string(path).unwrap_or_default();
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    if content.contains(&marker) {
        anyhow::bail!("section `{signature}` already exists; refusing wholesale overwrite");
    }
    if content.is_empty() {
        content.push_str("# TinyClaw evolution corrections\n\n");
    }
    content.push_str(&marker);
    content.push_str("\n\n");
    content.push_str(body.trim());
    content.push('\n');
    std::fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn proposal(kind: TargetKind, trust: TrustLevel) -> EvolutionPropose {
        EvolutionPropose {
            signature: "prefer-rg-search".to_string(),
            target_kind: kind,
            target_ref: Some("prefer-rg-search".to_string()),
            content: "Use `rg` for repository content search.".to_string(),
            trust_level: trust,
        }
    }

    fn approval(kind: TargetKind, trust: TrustLevel, disposition: Disposition) -> EvolutionApprove {
        EvolutionApprove {
            signature: "prefer-rg-search".to_string(),
            target_kind: kind,
            target_ref: Some("prefer-rg-search".to_string()),
            trust_level: trust,
            disposition,
        }
    }

    #[test]
    fn rejects_mismatched_approval() {
        let prop = proposal(TargetKind::Tool, TrustLevel::L1);
        let mut app = approval(TargetKind::Tool, TrustLevel::L1, Disposition::AllowOnce);
        app.signature = "other".to_string();
        assert!(validate_matching_approval(&prop, &app).is_err());
    }

    #[test]
    fn behaviour_requires_l3_approval() {
        let prop = proposal(TargetKind::Behaviour, TrustLevel::L1);
        let app = approval(
            TargetKind::Behaviour,
            TrustLevel::L1,
            Disposition::AllowOnce,
        );
        assert!(validate_matching_approval(&prop, &app).is_err());
    }

    #[test]
    fn approved_preference_appends_correction_and_applied_audit() {
        let dir = tempdir().unwrap();
        let mut registry = CommandRegistry::new();
        let prop = proposal(TargetKind::Tool, TrustLevel::L1);
        let app = approval(TargetKind::Tool, TrustLevel::L1, Disposition::AllowOnce);
        let outcome = apply_approved_proposal(dir.path(), &mut registry, &prop, &app).unwrap();
        assert!(matches!(outcome, EvolutionApplyOutcome::Applied { .. }));
        let corrections = std::fs::read_to_string(
            dir.path()
                .join(".terraphim")
                .join("evolution")
                .join("corrections.md"),
        )
        .unwrap();
        assert!(corrections.contains("## prefer-rg-search"));
        let audit = std::fs::read_to_string(
            dir.path()
                .join(".terraphim")
                .join("evolution")
                .join("audit.jsonl"),
        )
        .unwrap();
        assert!(audit.contains("evo.applied"));
    }

    #[test]
    fn skill_proposals_defer_without_applied_event() {
        let dir = tempdir().unwrap();
        let mut registry = CommandRegistry::new();
        let prop = proposal(TargetKind::Skill, TrustLevel::L1);
        let app = approval(TargetKind::Skill, TrustLevel::L1, Disposition::AllowOnce);
        let outcome = apply_approved_proposal(dir.path(), &mut registry, &prop, &app).unwrap();
        assert!(matches!(outcome, EvolutionApplyOutcome::Deferred { .. }));
        let audit = std::fs::read_to_string(
            dir.path()
                .join(".terraphim")
                .join("evolution")
                .join("audit.jsonl"),
        )
        .unwrap();
        assert!(audit.contains("evo.defer"));
        assert!(!audit.contains("evo.applied"));
    }
}
