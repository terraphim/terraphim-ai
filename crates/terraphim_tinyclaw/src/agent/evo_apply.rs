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

    // **P1#5 fix (production wiring)**: use the merge writer instead of
    // the create-only writer. `merge_section_scoped` will create the
    // file on first write (emitting `## {section_key}` itself) and
    // replace the section on subsequent re-applies. The proposal
    // signature is the stable section key (one signature per evolution
    // type, e.g. `prefer-rg-search`); operators re-approving a
    // behaviour evolution targeting an existing command now lands as
    // a merge rather than deterministically failing with
    // "refusing wholesale overwrite".
    //
    // **Defensive normalization (P2 fix from r14 review)**: the
    // signature is LLM-supplied and only advisory-constrained to
    // kebab-case. `write_or_merge_command_section` calls
    // `validate_command_name` on the section key — a non-kebab
    // signature (`Prefer_RG`, `evo:prefer-rg-search`, etc.) would
    // hard-fail the apply step *after* operator approval, surfacing
    // as a requeue loop. Normalize first: lowercase, then replace any
    // non-kebab-character with `-`, then collapse consecutive `-`s.
    let section_key = normalize_kebab_case(&proposal.signature);
    let target_path = registry
        .write_or_merge_command_section(&commands_dir, target_ref, &section_key, &proposal.content)
        .context("failed to write or merge command proposal")?;

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

/// Defensive normalization for LLM-supplied evolution signatures.
///
/// The signature is emitted by the model (`evo.propose`) and only
/// advisory-constrained to kebab-case (prompt at `evo_trigger.rs:285`).
/// `write_or_merge_command_section` calls `validate_command_name` on the
/// section key — a non-kebab signature (`Prefer_RG`,
/// `evo:prefer-rg-search`, etc.) would hard-fail the apply step after
/// operator approval. Normalize first: lowercase, replace any
/// non-kebab-character with `-`, collapse consecutive `-`s, trim leading
/// and trailing `-`s.
///
/// This is robust against realistic LLM drift (case, punctuation,
/// prefixes) but does NOT change the *identity* of an evolution: the
/// approval's `stable_text_eq` (line ~81) still compares against the
/// original (un-normalized) signature, so two proposals with
/// semantically-equal but textually-different signatures would still be
/// distinct identities. Normalization is purely cosmetic for the
/// section key; the audit trail continues to record the original
/// signature.
fn normalize_kebab_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_dash = false;
    for c in s.chars() {
        let lc = c.to_ascii_lowercase();
        if lc.is_ascii_lowercase() || lc.is_ascii_digit() {
            out.push(lc);
            prev_dash = false;
        } else {
            // Any non-kebab character collapses into a single `-`.
            if !prev_dash {
                out.push('-');
                prev_dash = true;
            }
        }
    }
    // Trim leading/trailing `-`s (from `:` or `_` at the edges).
    out.trim_matches('-').to_string()
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

    /// **P1#5 production wiring regression test**: a second behaviour
    /// evolution applied to the same `target_ref` MUST NOT fail with
    /// "refusing wholesale overwrite". This was the r12 commit
    /// message's claim that the r13 reviewer found to be test-only —
    /// `apply_behaviour` was still using the create-only writer.
    ///
    /// Behaviour proposals require L3 trust per `validate_matching_approval`,
    /// so both applies use L3.
    #[test]
    fn behaviour_re_approval_merges_in_place() {
        let dir = tempdir().unwrap();
        let mut registry = CommandRegistry::new();
        let prop = proposal(TargetKind::Behaviour, TrustLevel::L3);
        let app = approval(
            TargetKind::Behaviour,
            TrustLevel::L3,
            Disposition::AllowOnce,
        );

        // First apply: should succeed (creates the file).
        let outcome1 = apply_approved_proposal(dir.path(), &mut registry, &prop, &app).unwrap();
        assert!(
            matches!(outcome1, EvolutionApplyOutcome::Applied { .. }),
            "first apply must succeed"
        );

        // Read the file to confirm exactly one `## prefer-rg-search`
        // heading was written (the round-12 reviewer's bug shape).
        let path = dir.path().join("commands").join("prefer-rg-search.md");
        let after_first = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            after_first
                .lines()
                .filter(|l| l.trim() == "## prefer-rg-search")
                .count(),
            1,
            "exactly one `## prefer-rg-search` heading after first apply"
        );

        // Second apply on the same target_ref: MUST succeed via the
        // merge writer, NOT fail with "refusing wholesale overwrite".
        // (Pre-r13 the create-only writer would error here, deterministically.)
        let prop2 = proposal(TargetKind::Behaviour, TrustLevel::L3);
        let app2 = approval(
            TargetKind::Behaviour,
            TrustLevel::L3,
            Disposition::AllowOnce,
        );
        let outcome2 = apply_approved_proposal(dir.path(), &mut registry, &prop2, &app2).unwrap();
        assert!(
            matches!(outcome2, EvolutionApplyOutcome::Applied { .. }),
            "second apply must merge, not error: {:?}",
            outcome2
        );

        // After the merge: exactly one heading still (replace, not append).
        let after_second = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            after_second
                .lines()
                .filter(|l| l.trim() == "## prefer-rg-search")
                .count(),
            1,
            "exactly one `## prefer-rg-search` heading after second apply (no duplication)"
        );
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

    // ---- P2 fix (round-14 review): kebab-case normalization ----

    #[test]
    fn normalize_kebab_case_handles_common_llm_drift() {
        // Pure kebab passes through unchanged.
        assert_eq!(normalize_kebab_case("prefer-rg-search"), "prefer-rg-search");
        // Underscores and spaces become `-`.
        assert_eq!(normalize_kebab_case("Prefer_RG"), "prefer-rg");
        assert_eq!(normalize_kebab_case("prefer rg"), "prefer-rg");
        // Colons and other punctuation collapse.
        assert_eq!(
            normalize_kebab_case("evo:prefer-rg-search"),
            "evo-prefer-rg-search"
        );
        // Consecutive non-kebab chars collapse to a single `-`.
        assert_eq!(normalize_kebab_case("a__b"), "a-b");
        // Edge dashes trimmed.
        assert_eq!(normalize_kebab_case(":foo:"), "foo");
        assert_eq!(normalize_kebab_case("_foo_"), "foo");
        // Empty stays empty.
        assert_eq!(normalize_kebab_case(""), "");
    }

    /// **P2 fix regression test**: a non-kebab LLM-supplied signature
    /// (e.g. `evo:prefer-rg-search`) must NOT hard-fail the behaviour
    /// apply step after operator approval. `normalize_kebab_case` is
    /// called on the signature before passing it to the merge writer.
    #[test]
    fn behaviour_apply_sanitizes_non_kebab_signature() {
        let dir = tempdir().unwrap();
        let mut registry = CommandRegistry::new();

        // Build a proposal with a colon-bearing signature — the
        // round-14 reviewer flagged this exact string.
        let mut prop = proposal(TargetKind::Behaviour, TrustLevel::L3);
        prop.signature = "evo:prefer-rg-search".to_string();
        prop.target_ref = Some("prefer-rg-search".to_string());
        // content WITHOUT the heading (new contract).
        prop.content = "Use rg for repository content search.".to_string();

        // Approval's signature must match the proposal's (stable_text_eq).
        let mut app = approval(
            TargetKind::Behaviour,
            TrustLevel::L3,
            Disposition::AllowOnce,
        );
        app.signature = "evo:prefer-rg-search".to_string();

        let outcome = apply_approved_proposal(dir.path(), &mut registry, &prop, &app).unwrap();
        assert!(
            matches!(outcome, EvolutionApplyOutcome::Applied { .. }),
            "non-kebab signature must be sanitized, not hard-fail: {:?}",
            outcome
        );

        let path = dir.path().join("commands").join("prefer-rg-search.md");
        let content = std::fs::read_to_string(&path).unwrap();
        // Section heading is the sanitized kebab (`evo-prefer-rg-search`).
        assert!(
            content.contains("## evo-prefer-rg-search"),
            "sanitized section heading missing; got: {}",
            content
        );
        // Exactly one such heading (no duplication from sanitization).
        assert_eq!(
            content
                .lines()
                .filter(|l| l.trim() == "## evo-prefer-rg-search")
                .count(),
            1,
            "exactly one sanitized heading after apply"
        );
    }
}
