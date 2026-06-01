# ADF Behaviour Specifications

Specification documents for the ADF (AI Dark Factory) orchestrator behavioural features.
Created as part of issue #1924 (re-scoping Slice 8 of PR #1788).

---

## Specifications

| Feature | Document | Source Module |
|---------|----------|---------------|
| Local project skill path resolution | [local-skill-path-resolution.md](local-skill-path-resolution.md) | `crates/terraphim_orchestrator/src/local_skills.rs` |
| Redacted output capture and timeout reporting | [redacted-output-capture.md](redacted-output-capture.md) | `crates/terraphim_spawner/src/redaction.rs`, `crates/terraphim_spawner/src/output.rs` |
| Webhook group alias dispatch and expansion limits | [webhook-group-alias-dispatch.md](webhook-group-alias-dispatch.md) | `crates/terraphim_orchestrator/src/webhook.rs` |
| Worktree fail-closed behaviour | [worktree-fail-closed.md](worktree-fail-closed.md) | `crates/terraphim_orchestrator/src/worktree_guard.rs` |
| Provider probe timeout and health classification | [provider-probe-timeout.md](provider-probe-timeout.md) | `crates/terraphim_orchestrator/src/provider_probe.rs` |

---

## Conventions

Each specification describes:

- **Behaviour** — what the system does, step by step
- **Invariants** — properties that always hold, with a source-code reference
- **Failure Modes** — what goes wrong, the observable effect, and recovery
- **Verification Note** — which tests cover the invariants and how to run them

These are standalone documentation artefacts. They do not contain runtime code changes
and do not include generated `.terraphim/learnings/*.md` session files.

---

## Authoritative Locations

The specifications in this directory are the single source of truth for the described
behaviours. When the source code changes, these documents must be updated to remain
consistent. Cross-references from issue comments or PR descriptions should link here
rather than to ephemeral session artefacts.

---

## Validation

No TLA+ model-checking toolbox is configured for this codebase. Validation is performed
through the Rust unit test suites listed in each specification's verification note.
All tests referenced were verified to pass on `gitea/main` as of 2026-06-01.
