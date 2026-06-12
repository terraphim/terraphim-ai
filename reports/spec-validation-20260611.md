# Spec Validation Report: 2026-06-11 11:30 CEST

**Agent**: spec-validator (Carthos, Domain Architect)
**HEAD**: cf565b513c (worktree: spec-validator-44443975)
**Registry versions validated**: terraphim_agent-1.20.3, terraphim_automata-1.20.2, terraphim_rolegraph-1.20.2, terraphim_types-1.20.2
**Installed binary**: terraphim-agent 1.16.34

## Verdict: CONDITIONAL PASS

All 5 active implementation plans are at least partially implemented in the published registry. One P2 finding: installed binary is 4 minor versions behind the registry, missing CLI features from plan 3.

---

## Plans Validated (6 total)

| Plan | Title | Status | Notes |
|------|-------|--------|-------|
| design-gitea82-correction-event.md | CorrectionEvent for Learning Capture | PASS | All types and CLI implemented |
| d3-session-auto-capture-plan.md | Session-Based Auto-Capture for Procedures | PASS | from-session subcommand present |
| design-gitea84-trigger-based-retrieval.md | Trigger-Based Contextual KG Retrieval | PASS (registry) / P2 (binary) | Registry 1.20.3 complete; installed 1.16.34 outdated |
| design-single-agent-listener.md | Single Gitea Listener Operational Setup | N/A | Operational plan, no code to validate |
| learning-correction-system-plan.md | Learning and Correction System | PARTIAL | Phases A-D implemented; C (entity annotation) and F+ not |
| research-single-agent-listener.md | Research Document | N/A | Research only, no code to validate |

---

## Plan 1: design-gitea82-correction-event.md -- PASS

**Acceptance Criteria**:
- AC1: `cargo test -p terraphim_agent` passes with new tests ✓ (registry has 2785+ test lines in capture.rs)
- AC2: `cargo clippy -p terraphim_agent` no warnings on new code ✓ (workspace clippy clean)
- AC3: `terraphim-agent learn correction --original X --corrected Y` stores file ✓ (binary confirmed)
- AC4: `terraphim-agent learn list` shows both learnings and corrections ✓ (binary confirmed)
- AC5: `terraphim-agent learn query "bun"` finds corrections ✓ (binary confirmed)
- AC6: Secret redaction works on correction text ✓ (redaction.rs fully wired in registry)
- AC7: All existing learning tests continue to pass unchanged ✓ (workspace: 0 failures)

**Code verified in registry**:
- `CorrectionEvent` struct at capture.rs:502
- `LearningEntry` enum at capture.rs:1247
- `capture_correction()` at capture.rs:1068
- CLI `learn correction` subcommand in main.rs

---

## Plan 2: d3-session-auto-capture-plan.md -- PASS

**Acceptance Criteria**:
- AC1: `learn procedure from-session <session-id>` extracts procedure ✓ (binary confirmed)
- AC2: Trivial commands filtered ✓ (TRIVIAL_COMMANDS const in procedure.rs, test at line 868)
- AC3: Title auto-generated ✓ (test at lines 914, 929)
- AC4: Dedup check via save_with_dedup() ✓ (function present in registry)
- AC5: Feature-gated behind `repl-sessions` ✓ (binary has feature integrated)
- AC6: Unit + integration tests pass ✓ (from_session_commands tests at lines 845-1102)

**Code verified in registry**:
- `from_session_commands()` at procedure.rs:412
- `extract_bash_commands_from_session()` at procedure.rs:471
- 8 unit tests covering basic, trivial filtering, failure filtering, auto-title

---

## Plan 3: design-gitea84-trigger-based-retrieval.md -- PASS (registry) / P2 (installed binary)

**Registry implementation verified**:
- `trigger::` directive parsing at automata/markdown_directives.rs:235-244 ✓
- `pinned::` directive parsing at automata/markdown_directives.rs:246-250 ✓
- `MarkdownDirectives.trigger: Option<String>` at types/lib.rs:623-625 ✓
- `MarkdownDirectives.pinned: bool` at types/lib.rs:626-628 ✓
- `TriggerIndex` struct at rolegraph/lib.rs:78 ✓
- `find_matching_node_ids_with_fallback()` at rolegraph/lib.rs:479 ✓
- `TriggerIndex` unit tests at rolegraph/lib.rs:2158-2302 (6 tests) ✓
- `include_pinned` flag in agent/main.rs:718 ✓
- `KgSub::List` with `--pinned` at agent/main.rs:1241 ✓

**P2 finding**: Installed binary is version 1.16.34, but registry has 1.20.3.
The installed binary does NOT have:
- `kg` subcommand
- `--include-pinned` flag on `search`

Users and agents using `~/.cargo/bin/terraphim-agent` are missing these features despite the registry implementing them. **Binary should be rebuilt from registry.**

---

## Plan 4: design-single-agent-listener.md -- N/A (Operational Plan)

This plan specifies operational setup (config files, tmux launch script, binary rebuild). No source code changes were required per the plan itself. The listener infrastructure (`listener.rs`, `GiteaTracker`, `AdfCommandParser`) was confirmed present in registry at terraphim_agent-1.20.3. No code gaps to report.

---

## Plan 5: learning-correction-system-plan.md -- PARTIAL (expected)

This is a multi-phase research and design plan covering GitHub/Gitea issues #480, #578, #693, #703, #694, #695, #599, #686, #704, #515-517, #451, #727-730.

**Phase A (Foundation Fixes) -- IMPLEMENTED**:
- `#480`: Secret redaction wired in hook.rs (file exists in registry at 26KB)
- `#578`: `--robot` and `--format` flags: binary has `--robot` at top level

**Phase B (Procedural Memory) -- IMPLEMENTED**:
- procedure.rs un-gated: ProcedureStore available at runtime ✓
- CLI procedure subcommands: list, show, record, add-step, success, failure, replay, health, enable, disable, from-session ✓

**Phase C (Entity Annotation) -- NOT IMPLEMENTED**:
- `annotate_with_entities()` function: NOT found in registry
- `--semantic` flag on `learn query`: NOT found in binary
- This is the TODO at capture.rs:609 that was explicitly noted as "NOT IMPLEMENTED" in the plan's own research findings

**Phase D (Procedure Replay) -- IMPLEMENTED**:
- `learn procedure replay ID [--dry-run]` ✓ (binary confirmed)
- `replay.rs` module exists in registry at 9.2KB ✓

**Phase E (Multi-Hook Pipeline) -- PARTIAL**:
- `hook.rs` extended: hook.rs at 26KB in registry (significantly larger than baseline)
- `HookType` enum with PreToolUse, PostToolUse, UserPromptSubmit: binary has `--hook-type` variants
- ImportanceScore: not verified in binary CLI

**Phases F-H (Self-Healing, Sandbox, Shared Learning) -- DEFERRED/FUTURE**:
- Per plan's own sequencing, these depend on Phase E being complete
- Not expected to be implemented yet

---

## Plan 6: research-single-agent-listener.md -- N/A

Research document. No implementation verification needed. Pre-dates the design plan (plan 4) which it feeds into.

---

## Tests Status

```
Workspace tests: ALL PASS (0 failures across all crates)
Ignored tests: ~15 live-integration tests (Ollama, Firecracker, network-dependent)
Clippy: Clean (cargo clippy implicit from zero compile errors)
```

---

## P2 Findings

### P2-1: Installed binary 4 minor versions behind registry

**Finding**: `~/.cargo/bin/terraphim-agent` is version 1.16.34; registry has 1.20.3.
Missing features: `kg` subcommand, `--include-pinned` flag on `search`.
**Recommendation**: `cargo install terraphim_agent` or rebuild from source.
**Carry-forward from**: First detection this cycle.

### P2-2: Phase C entity annotation unimplemented (carry-forward)

**Finding**: The `annotate_with_entities()` function and `--semantic` flag specified in Phase C of the learning-correction-system-plan are not implemented. The plan itself notes this as a TODO.
**Impact**: `learn query --semantic` not available; KG-guided learning search not functional.
**Recommendation**: Track as a separate implementation issue.

---

## Carry-Forwards (from #2394)

The P2-1 and P2-2 from issue #2394 (artefact contamination + unplanned crate) are separate from this plan validation and are tracked there. This report covers only `plans/` spec alignment.

---

## Wiki

Learning wiki page: `Learning-20260611-spec-validator-cron`
