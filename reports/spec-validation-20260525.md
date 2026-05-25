# Spec Validation Report — Cron Scan 2026-05-25

**Validator:** Carthos (spec-validator, Domain Architect)
**Mode:** Cron schedule
**Scan date:** 2026-05-25 02:30 CEST

---

## Verdict: PASS — all plans implemented; one follow-up gap found

All six plans in `plans/` remain fully implemented — no regression in the 7-day gap
since the 2026-05-18 report. One new spec item surfaced: the `terraphim_grep`
`RELEASE_DESIGN.md` (status: Draft) is substantially implemented but has a missing
`CHANGELOG.md` that `.release-plz.toml` depends on.

---

## Scope Scanned

| Plan Document | Change Since 2026-05-18 | Verdict |
|---|---|---|
| `plans/design-gitea84-trigger-based-retrieval.md` | None | ✅ PASS (unchanged) |
| `plans/design-gitea82-correction-event.md` | None | ✅ PASS (unchanged) |
| `plans/learning-correction-system-plan.md` | None | ✅ PASS (Phase I deferred by plan) |
| `plans/d3-session-auto-capture-plan.md` | None | ✅ PASS (unchanged) |
| `plans/design-single-agent-listener.md` | None | ✅ OUT OF SCOPE (operational) |
| `plans/research-single-agent-listener.md` | None | ✅ RESEARCH DOC (no AC) |
| `crates/terraphim_grep/RELEASE_DESIGN.md` | **NEW** (Draft) | ⚠️ PARTIAL — CHANGELOG.md missing |

---

## Plans 1–6: No Change Since 2026-05-18

The codebase inspection confirms all implementations are present and unchanged:

| Key Invariant | File | Status |
|---|---|---|
| `CorrectionEvent`, `CorrectionType`, `LearningEntry` in `capture.rs` | `learnings/capture.rs:502,1245` | ✅ |
| `TriggerIndex` TF-IDF in `rolegraph` | `rolegraph/src/lib.rs:78` | ✅ |
| `trigger::` / `pinned::` parsing | `terraphim_automata/src/markdown_directives.rs:235-247` | ✅ |
| `MarkdownDirectives.trigger` / `.pinned` fields | `terraphim_types/src/lib.rs:325-328` | ✅ |
| `procedure.rs` un-gated (`pub(crate) mod procedure`) | `learnings/mod.rs:31` | ✅ |
| `from_session_commands` + `extract_bash_commands_from_session` | `learnings/procedure.rs:412,471` | ✅ |
| `ProcedureSub::FromSession` CLI variant | `main.rs:1213` | ✅ |
| `SharedLearningSub` CLI (feature-gated `shared-learning`) | `main.rs:1087` | ✅ |
| `GuardDecision` (Allow/Sandbox/Block) in `guard_patterns.rs` | `guard_patterns.rs` | ✅ |
| `query_all_entries_semantic` for entity annotation | `learnings/mod.rs:42` | ✅ |
| `replay_procedure` + `StepOutcome` | `learnings/replay.rs`, `mod.rs:38` | ✅ |

---

## New Item: terraphim_grep RELEASE_DESIGN.md

**Plan status:** Draft (awaiting approval)  
**Plan location:** `crates/terraphim_grep/RELEASE_DESIGN.md` (not in top-level `plans/`)

This plan describes a staged release of `terraphim_grep` v1.20.0. Code inspection shows
Stages 1–3 are substantially implemented ahead of the plan's approval.

### Stage Verification

| Stage | Requirement | Evidence | Status |
|---|---|---|---|
| 1.1 | `fff-search` 0.8.2 on crates.io | `terraphim_grep/Cargo.toml:32` `fff-search = { version = "0.8.2", ... }` | ✅ |
| 1.2 | API drift fixed (0.5.1 → 0.8.2) | `hybrid_searcher.rs:303-315` — comments document 6 API changes; imports updated | ✅ |
| 1.3 | Workspace version 1.20.0 | `Cargo.toml:47` `version = "1.20.0"` | ✅ |
| 1.4 | `[package.metadata.deb]` block | `terraphim_grep/Cargo.toml:65-83` — maintainer/deps/assets present | ✅ |
| 2.1 | `terraphim_grep-v*` tag trigger | `.github/workflows/release-comprehensive.yml:10` | ✅ |
| 2.2 | `terraphim_grep` in BINARIES list | `release-comprehensive.yml:75` | ✅ |
| 2.3 | Cargo build steps for 7 targets | `release-comprehensive.yml:239` | ✅ |
| 2.4 | Artifact prep (Unix + Windows) | `release-comprehensive.yml:300-332` | ✅ |
| 2.5 | Debian packaging step | `release-comprehensive.yml` + `Cargo.toml:65` | ✅ |
| 2.6 | Sign/notarise terraphim-grep | `release-comprehensive.yml:458` | ✅ |
| 3.1 | `terraphim_grep` in `publish-crates.sh` | `scripts/publish-crates.sh:66` | ✅ |
| 3.2 | `terraphim_grep` in `.release-plz.toml` | `.release-plz.toml:45-46` | ✅ |
| 3.2 | `CHANGELOG.md` at `crates/terraphim_grep/CHANGELOG.md` | **FILE MISSING** — referenced by `.release-plz.toml:46` but not yet created | ❌ |
| 3.3 | Homebrew formula (separate repo) | Out of workspace scope | ⚠️ N/A |
| 4–6 | Dry-run, tag, post-release verify | Operational — cannot verify without CI execution | ⚠️ PENDING |

---

## Requirements Traceability

| Req ID | Plan | Impl Ref | Status |
|---|---|---|---|
| TRG-01..07 | gitea84 | `markdown_directives.rs:235`, `rolegraph/lib.rs:78`, `main.rs:718` | ✅ (unchanged) |
| COR-01..07 | gitea82 | `capture.rs:502`, `main.rs:989` | ✅ (unchanged) |
| PRO-01..02 | learn-plan Phase B | `mod.rs:31`, `main.rs:1213` | ✅ (unchanged) |
| ANN-01 | learn-plan Phase C | `mod.rs:46` | ✅ (unchanged) |
| REP-01 | learn-plan Phase D | `replay.rs`, `mod.rs:38` | ✅ (unchanged) |
| HKE-01, IMP-01 | learn-plan Phase E | `hook.rs:33`, `capture.rs:102` | ✅ (unchanged) |
| GRD-01 | learn-plan Phase H | `guard_patterns.rs` | ✅ (unchanged) |
| SHR-01 | learn-plan Phase G | `main.rs:1087` | ✅ (unchanged) |
| GRP-01 | RELEASE_DESIGN.md S1-S2 | `terraphim_grep/Cargo.toml`, `release-comprehensive.yml` | ✅ |
| GRP-02 | RELEASE_DESIGN.md S3.1 | `scripts/publish-crates.sh:66` | ✅ |
| GRP-03 | RELEASE_DESIGN.md S3.2 | `.release-plz.toml:45-46` | ⚠️ PARTIAL — CHANGELOG.md missing |

---

## Gaps

| Gap | Severity | Recommended Action |
|---|---|---|
| `crates/terraphim_grep/CHANGELOG.md` missing — referenced by `.release-plz.toml:46` | ⚠️ Follow-up | Create empty CHANGELOG.md or initial entry; `release-plz update` will populate it automatically once the file exists |
| Phase I agent evolution CLI wiring | ⚠️ Deferred | Explicitly deferred by plan; not a spec violation |
| Stages 4–6 (dry-run, tag, post-release) | ⚠️ Pending | Operational; dependent on decision to proceed with v1.20.0 release |
| Homebrew formula (separate repo) | ⚠️ Out of scope | Tracked in `terraphim/homebrew-terraphim` — not verifiable from this workspace |
| CI issue #1846: `verify-versions` fails on workflow_dispatch from branches | ℹ️ Pre-existing | Separate tracking; does not affect tag-triggered workflow |
| CI issue #1845: docker multiarch fails | ℹ️ Pre-existing | Separate tracking |

---

## Recommendations

1. Create `crates/terraphim_grep/CHANGELOG.md` (even empty, with `# Changelog` header) to unblock `release-plz update` before the v1.20.0 tag is pushed.
2. The RELEASE_DESIGN.md plan is ready for formal approval — all code prerequisites for Stage 1–3 are satisfied.
3. Run Stage 4 dry-run (`gh workflow run release-comprehensive.yml -f test_run=true`) once CHANGELOG.md is present and plan is approved.
4. No action needed for plans 1–6: all implementations stable.
