# Spec Validation Report — 2026-06-11 07:30 CEST

**Verdict: CONDITIONAL_PASS**
**Run by:** Carthos (Domain Architect)
**Worktree:** spec-validator-4b3cc5e0
**Plans directory:** `plans/` (6 active plans — unchanged since 05:31 run)
**HEAD:** `eec0aa23cc`
**Previous run:** 2026-06-11 05:31 CEST (PASS at `3e60346740`)

---

## What Changed Since 05:31

Two commits landed after the previous PASS verdict:

| SHA | Time | Description |
|-----|------|-------------|
| `eb50a2c938` | 06:07 | fix(terraphim_rlm): Unicode panic + 2-char word filter bypass — Refs #2411 #2412 |
| `eec0aa23cc` | 06:09 | chore(sessions): add handover for issues 2411 2412 — Refs #2411 #2412 |

Files changed:
- `crates/terraphim_rlm/src/validator.rs` (implementation fix)
- `.sessions/session-20260611-060437.md` (contamination)
- `.sessions/handover-20260611-echo-2411-2412.md` (contamination)

---

## Plan Verdicts (unchanged from 05:31 run)

All 6 plans carry forward from 05:31 PASS. No plan modifications detected.

| Plan | Status |
|------|--------|
| design-gitea82-correction-event | PASS |
| d3-session-auto-capture-plan | PASS |
| design-gitea84-trigger-based-retrieval | PASS |
| research-single-agent-listener | PASS |
| learning-correction-system-plan | CONDITIONAL PASS (Phase I/J deferred) |
| design-single-agent-listener | PASS |

---

## New Implementation Analysis

### `terraphim_rlm` — Issues #2411 and #2412

**Fixes landed:** `crates/terraphim_rlm/src/validator.rs`

**#2411 — Unicode panic in log helper:**
- Root cause: byte-indexed slice on multi-byte UTF-8 input > 100 bytes
- Fix: `char_indices()` based truncation replacing direct byte-index slice
- Evidence: `test_truncate_for_log_unicode_no_panic`, `test_truncate_for_log_mixed_ascii_unicode_no_panic`

**#2412 — 2-char word filter bypass:**
- Root cause: `len > 2` filter silently dropped `rm`, `ls`, `mv`, `cp` from KG validation
- Fix: STOP_WORDS list replacing length gate; 2-char shell commands now visible to validator
- Evidence: `test_extract_words_does_not_skip_rm`, `test_extract_words_preserves_two_char_commands`,
  `test_extract_words_filters_stop_words`

**Test result (with CARGO_REGISTRIES_TERRAPHIM_TOKEN):**
```
test result: ok. 132 passed; 0 failed; 0 ignored; finished in 0.22s
```

All 5 regression tests from commit message confirmed present and passing.

---

## Gap Register

| Gap ID | Severity | Description | New? |
|--------|----------|-------------|------|
| G-P2-1 | P2 | `.sessions/` contamination recurring (now 17 tracked files, +2 since 05:31) | Recurring (5th+) |
| G-P2-2 | P2 | `terraphim_rlm` has no plan in `plans/` — crate received 3 implementation commits (#2407, #2411, #2412) with no spec | NEW |
| G-P2-3 | P2 | `cargo test -p terraphim_rlm` without registry token produces 0 unit tests; kg-validation feature silently disabled without CARGO_REGISTRIES_TERRAPHIM_TOKEN | NEW |
| G-P3-1 | P3 | Phase I agent evolution real LLM wiring deferred | Carry-forward |
| G-P3-2 | P3 | Phase J validation pipeline (#515-517) not traced | Carry-forward |

### G-P2-1: `.sessions/` Contamination Detail

`.gitignore` has NO coverage for `.sessions/` (`git check-ignore` rc=1).

Current tracked files (17 total):
```
.sessions/handoff-1848-terraphim-grep-readme.md
.sessions/handover-20260427.md
.sessions/handover-20260517-2011.md
.sessions/handover-20260518-rlm.md
.sessions/handover-20260518.md
.sessions/handover-20260601-echo-1940.md
.sessions/handover-20260611-echo-2407.md        ← added since 05:31
.sessions/handover-20260611-echo-2411-2412.md   ← added this cycle
.sessions/session-20260121-002908.md
.sessions/session-20260121-130300.md
.sessions/session-20260121-135300.md
.sessions/session-20260122-080604.md
.sessions/session-20260427-083414.md
.sessions/session-20260517-135105.md
.sessions/session-20260517-140155.md
.sessions/session-20260611-050700.md            ← added since 05:31
.sessions/session-20260611-060437.md            ← added this cycle
```

Active issues: #2394 (artefact contamination), #2413 (root-cause source-fix).

### G-P2-2: `terraphim_rlm` Unplanned Implementation

Crate `crates/terraphim_rlm/` is in the workspace (`crates/*` glob in Cargo.toml).
Three implementation commits reference this crate with no corresponding plan in `plans/`:

| Commit | Issue | Change |
|--------|-------|--------|
| `ae9446b166` | #2407 | `feat(terraphim_rlm): implement KG validation in FirecrackerExecutor::validate()` |
| `eb50a2c938` | #2411, #2412 | `fix(terraphim_rlm): fix Unicode panic and 2-char word filter bypass` |

The crate is substantial (29KB `rlm.rs`, 27KB `validator.rs`, 20KB `mcp_tools.rs`).
**Recommendation:** Create a plan or research document for `terraphim_rlm` to establish its
acceptance criteria and allow spec validators to assess completeness.

### G-P2-3: Registry Token Dependency for Test Coverage

Without `CARGO_REGISTRIES_TERRAPHIM_TOKEN`:
- `cargo test -p terraphim_rlm` → 0 unit tests (kg-validation deps not downloaded)
- The regression tests for #2411/#2412 are invisible to CI without the token

With `CARGO_REGISTRIES_TERRAPHIM_TOKEN="Bearer $GITEA_TOKEN"`:
- `cargo test -p terraphim_rlm --features kg-validation` → 132/132 PASS

**Recommendation:** CI pipeline must set `CARGO_REGISTRIES_TERRAPHIM_TOKEN` for `terraphim_rlm`
test jobs, or the private registry deps should be vendored/mirrored for CI.

---

## Overall Verdict: CONDITIONAL_PASS

Plans are sound. New implementation (#2411, #2412) is correctly fixed with regression coverage
verified (132/132 when token available). Two new P2 gaps opened:
- `terraphim_rlm` lacks a plan document
- Registry token required for test coverage is not documented as a CI prerequisite

Recurring P2-1 (`.sessions/` contamination) continues to grow; root-cause fix (#2413) not yet merged.
