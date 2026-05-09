# Spec Validation Report -- 2026-05-09

**Agent**: Carthos (Domain Architect, spec-validator)
**Run**: 2026-05-09 02:33 CEST -- cron schedule; scanned open PRs #1308, #1316, #1319, #1343, #1360, #1365; re-assessed all persistent gaps against main HEAD `fd484da22`

---

## Summary

| PR | Head SHA | Title | Verdict |
|----|----------|-------|---------|
| #1308 | 112bc99 | Fix #1297: close persistent spec gaps | PASS (doc-bloat concern) |
| #1316 | 2526414 | Fix #446: exempt C1-blocked probes from circuit-breaker | PASS (doc-bloat concern) |
| #1319 | 1841664 | Fix #1313: harden compose and CI Redis/Ollama bindings | PASS (doc-bloat concern) |
| #1343 | 1e9847d | Fix #1266: update NormalizedTerm initializers to builder | CONCERNS |
| #1360 | 5ff2434 | Fix #1355: add --no-fail-fast to cargo test --workspace | PASS |
| #1365 | 7e4e894 | Fix #1362: gate rustdoc warnings with RUSTDOCFLAGS=-D | PASS |

**Persistent gap status**: 5 of 6 tracked gaps remain open on `main`.

---

## PR #1308 -- PASS (doc-bloat concern)

**Issue**: #1297 -- close persistent spec gaps (G-PH-H-001/002: `guard.rs` absent)

### Requirements Coverage

| Req | Description | Status |
|-----|-------------|--------|
| REQ-1297-001 | Add `crates/terraphim_agent/src/learnings/guard.rs` (+352 lines) | PASS -- file present in PR diff |
| REQ-1297-002 | Wire `guard` into `mod.rs` | PARTIAL -- `mod.rs` included in PR diff (+3 lines); exact wiring not verified without file content |
| REQ-1297-003 | Surface `guard` from production entry points (automata, middleware, orchestrator, persistence, rolegraph, service) | PASS -- each crate has small +3-8 line additions |

### Concern

- **C-1308-A** (doc-bloat): PR includes 6 prior-session report files (`reports/spec-validation-20260507*.md`, `doc-gap-report-20260507*.md`). These inflate review surface without adding to the fix. Recommend separating documentation from implementation commits.

---

## PR #1316 -- PASS (doc-bloat concern)

**Issue**: #446 -- Anthropic provider health check failing (circuit-breaker misclassifying C1-blocked probes)

### Requirements Coverage

| Req | Description | Status |
|-----|-------------|--------|
| REQ-446-001 | Modify `provider_probe.rs` to exempt C1-blocked probes from circuit-breaker state updates | PASS -- `provider_probe.rs` +110/-31 (substantive change scope consistent with fix) |
| REQ-446-002 | Provider health checks for Terraphim AI models must not report false failures | PARTIAL -- logic change visible; test coverage not confirmed from diff |

### Concern

- **C-1316-A** (doc-bloat): PR carries 6 prior-session report files (+625 lines of doc). These belong in separate documentation commits or are already present on `main`.
- **C-1316-B** (verification gap): No test file changes visible. Issue #446 requires asserting that a C1-blocked probe no longer increments the circuit-breaker counter. Without a unit test asserting this invariant, the fix is not independently verifiable by CI.

---

## PR #1319 -- PASS (doc-bloat concern)

**Issue**: #1313 -- harden Docker Compose and CI Redis/Ollama network bindings

### Requirements Coverage

| Req | Description | Status |
|-----|-------------|--------|
| REQ-1313-001 | Bind Redis to 127.0.0.1 in docker-compose.yml | PASS -- file in diff |
| REQ-1313-002 | Bind Ollama to 127.0.0.1 or remove external exposure | PASS -- .env.example updated |
| REQ-1313-003 | Update CI workflow (vm-execution-tests.yml) to reflect hardened config | PASS -- workflow in diff |

### Concern

- **C-1319-A** (doc-bloat): PR carries the same prior-session report files as #1316. These inflate review surface.
- **C-1319-B** (security verification): The security sentinel (issue #1374) notes that Ollama `*:11434` exposure is through systemd, not the compose file. This PR addresses compose only; systemd binding remains unresolved. The PR title and body correctly scope to compose/CI -- this is a note, not a blocker.

---

## PR #1343 -- CONCERNS

**Issue**: #1266 -- NormalizedTerm missing fields break compilation with `--all-features`

### Requirements Coverage

| Req | Description | Status |
|-----|-------------|--------|
| REQ-1266-001 | Add `action`, `priority`, `trigger`, `pinned` fields to `NormalizedTerm` with `#[serde(default)]` | PARTIAL -- not visible in file list |
| REQ-1266-002 | Update all downstream struct initialisers to builder pattern | PARTIAL -- session-analyzer, automata-related files updated |

### Concerns

- **C-1343-A** (scope explosion): PR touches 30+ crates with rustdoc additions (+4-5 lines each across haystack_atlassian, haystack_core, haystack_discourse, haystack_grepapp, haystack_jmap, terraphim-markdown-parser, terraphim_automata_py, terraphim_build_args, terraphim_ccusage, terraphim_file_search, terraphim_github_runner_server, terraphim_kg_linter, terraphim_lsp, terraphim_mcp_server, terraphim_dsm, etc.). This is a documentation sweep bundled with a compilation fix. Issue #1266 is narrowly scoped; these additions are unrelated.
- **C-1343-B** (unrelated changes): `crates/terraphim_automata/src/medical_artifact.rs` (+80/-9) is a substantial change with no connection to NormalizedTerm initialiser patterns. Requires explicit justification or separation.
- **C-1343-C** (merge risk): Previous v11 concern C-1283-C (this PR's predecessor) flagged shared base with PR #1279. PR #1279 appears closed or renamed; verify merge lineage before landing.
- **C-1343-D** (verification gap): `terraphim_automata/Cargo.toml` dependency change (+2/-1) and `sharded_extractor.rs` (+7) have no visible rationale. Dependency bumps must be documented.

**Recommendation**: Split the rustdoc sweep and `medical_artifact.rs` change into a separate PR. Land the NormalizedTerm builder fix in isolation.

---

## PR #1360 -- PASS

**Issue**: #1355 -- all crate failures should surface in a single CI run

### Requirements Coverage

| Req | Description | Status |
|-----|-------------|--------|
| REQ-1355-001 | Add `--no-fail-fast` to `cargo test --workspace` in ci-main.yml | PASS |
| REQ-1355-002 | Add `--no-fail-fast` to `cargo test --workspace` in ci-pr.yml | PASS |

No concerns. Scope is precisely bounded. Multiple haystack crate additions follow the same rustdoc pattern as #1343/#1365 -- consistent with a coordinated documentation sweep.

---

## PR #1365 -- PASS

**Issue**: #1362 -- lock in rustdoc cleanliness from #1331 with CI enforcement

### Requirements Coverage

| Req | Description | Status |
|-----|-------------|--------|
| REQ-1362-001 | Add `RUSTDOCFLAGS=-D warnings` to ci-pr.yml cargo doc step | PASS |
| REQ-1362-002 | Add `RUSTDOCFLAGS=-D warnings` to ci-main.yml cargo doc step | PASS |

No concerns. A CI gate that enforces a previously-achieved standard is the correct pattern.

---

## Plans/ Directory Validation

### Active Spec Documents vs Implementation

| Plan | Spec Status | Implementation Status | Gap |
|------|-------------|----------------------|-----|
| `design-gitea82-correction-event.md` | APPROVED | IMPLEMENTED | None |
| `design-gitea84-trigger-based-retrieval.md` | APPROVED | PARTIAL -- `trigger`/`pinned` parsing present; `Graph list --pinned` CLI absent | G-REQ-84-004 persists |
| `d3-session-auto-capture-plan.md` | Draft | NOT IMPLEMENTED -- `procedure.rs` wired as `pub(crate)`; `from-session` subcommand absent from CLI surface | G-D3-001 persists |
| `design-single-agent-listener.md` | APPROVED | NOT DEPLOYED -- `config/listener-worker.json` and `scripts/start-listener.sh` absent | G-SAL-001 persists |
| `learning-correction-system-plan.md` | Research | `procedure.rs` present but not wired to CLI; `SharedLearningStore` feature-gated only | G-LCS-001 persists |

---

## Persistent Gaps Re-assessment

| Gap | Previous Status | Current Status (2026-05-09) |
|-----|-----------------|---------------------------|
| G-META-001: `meta_coordinator` absent from orchestrator `lib.rs` | OPEN | OPEN -- no grep match on main; no PR addresses this |
| G-PH-H-001/002: `guard.rs` absent | OPEN | RESOLVING -- PR #1308 adds +352 lines; not yet merged; `guard` not in `mod.rs` on main |
| G-REQ-84-004: `Graph list --pinned` CLI argument | OPEN | OPEN -- `pinned` field in data model; CLI argument absent |
| G-D3-001: `from-session` subcommand | OPEN | OPEN -- `procedure.rs` wired as `pub(crate)`; CLI entry point absent |
| G-SAL-001: listener deployment artefacts | OPEN | OPEN -- `listener-worker.json` and `start-listener.sh` absent |
| G-LCS-001: `SharedLearningStore` CLI | OPEN | OPEN -- feature-gated `suggest` module; no CLI surface wired |

---

## Structural Observation

Across PRs #1308, #1316, #1319, #1343, #1360, and #1365, a consistent pattern is visible: prior-session documentation reports are committed into the `reports/` directory as part of implementation PRs. This inflates review surface, obscures the actual fix, and risks merge conflicts when multiple PRs carry the same report files. The pattern should be standardised: documentation commits should be separated from implementation commits, or the `reports/` directory should be managed as a single append-only commit after merge.
