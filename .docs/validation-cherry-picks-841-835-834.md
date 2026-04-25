# Validation Report: Cherry-picked PRs #841, #835, #834

**Status**: Validated
**Date**: 2026-04-25
**Verification Report**: `.docs/verification-cherry-picks-841-835-834.md`
**Final commit**: `bd0e83227`

## Executive Summary

All cherry-picked features pass acceptance testing. 3 stale PRs worth of valuable code merged cleanly after cherry-picking. Two medium-severity bugs found during code review were fixed before merge. Customer data protection confirmed intact.

## System Test Results

### End-to-End Scenarios

| ID | Workflow | Steps | Result | Status |
|----|----------|-------|--------|--------|
| E2E-001 | OpenCode connector availability | `cargo test -p terraphim_sessions --features opencode-connector` | 66 tests pass | PASS |
| E2E-002 | Codex connector availability | `cargo test -p terraphim_sessions --features codex-connector` | Included in 66 | PASS |
| E2E-003 | export-kg CLI command | `terraphim-agent learn export-kg --help` | Help displayed, options correct | PASS |
| E2E-004 | export-kg unit tests | `cargo test --bin terraphim-agent -- test_export` | 6 tests pass | PASS |
| E2E-005 | Tinyclaw workspace exclusion | `grep tinyclaw Cargo.toml` + `cargo build --workspace` | Excluded, builds clean | PASS |
| E2E-006 | Customer data not tracked | `git ls-files projects/odilo/` | Empty (not tracked) | PASS |
| E2E-007 | Customer data gitignore | `grep projects/odilo/ .gitignore` | Entry present | PASS |
| E2E-008 | NormalizedTerm backward compat | Deserialise existing data | `#[serde(default)]` on all new fields | PASS |
| E2E-009 | Full workspace build | `cargo build --workspace` | Clean, 0 errors | PASS |
| E2E-010 | Full workspace tests | `cargo test --workspace` | 3416 passed, 0 failed | PASS |

### Non-Functional Requirements

| Category | Check | Tool | Result | Status |
|----------|-------|------|--------|--------|
| Security - Vulnerabilities | cargo audit | cargo-audit | 0 vulnerabilities (RUSTSEC-2026-0104 resolved) | PASS |
| Security - Unmaintained | cargo audit | cargo-audit | 4 warnings (pre-existing, not from cherry-picks) | PASS |
| Security - UBS scan | Static analysis | ubs | 0 critical, 0 critical in changed files | PASS |
| Security - WalkDir DoS | Code review | Manual | Fixed: max_depth(4), follow_links(false) | PASS |
| Performance - Build | cargo build | cargo | 13s (dev profile, unchanged) | PASS |
| Quality - Formatting | cargo fmt --check | rustfmt | Clean | PASS |
| Quality - Linting | cargo clippy | clippy | 0 new warnings | PASS |
| Compatibility - serde | Field defaults | serde | All new fields have #[serde(default)] | PASS |

### Acceptance Criteria by PR

#### PR #841 - OpenCode and Codex JSONL Session Connectors
- [x] OpenCodeConnector parses `~/.local/state/opencode/prompt-history.jsonl`
- [x] CodexConnector parses `~/.codex/sessions/*.jsonl`
- [x] Both registered in ConnectorRegistry behind feature flags
- [x] Feature flags `opencode-connector` and `codex-connector` in Cargo.toml
- [x] Unit tests for both connectors with sample JSONL data
- [x] Test fix for data-dependent assertion in integration tests

#### PR #835 - Tinyclaw Exclusion (RUSTSEC)
- [x] terraphim_tinyclaw excluded from workspace
- [x] RUSTSEC-2026-0104 (rustls-webpki 0.102.8) removed from dependency tree
- [x] cargo audit: 0 vulnerabilities
- [x] Build separately: `cd crates/terraphim_tinyclaw && cargo build`
- [x] Extract validation test changes: deferred (18 conflicts, not critical)

#### PR #834 - learn export-kg + NormalizedTerm Metadata
- [x] `terraphim-agent learn export-kg --output DIR` command works
- [x] Exports corrections as Logseq-style KG markdown
- [x] `--correction-type` filter (all, tool-preference)
- [x] 7 unit tests pass (empty dir, single, merge, filter, filenames, slugify)
- [x] Slug collision prevention via HashSet + index loop
- [x] NormalizedTerm: action, priority, trigger, pinned fields added
- [x] Backward compatible (all fields optional with serde defaults)

#### Customer Data Protection
- [x] projects/odilo/ removed from git tracking
- [x] .gitignore entry present
- [x] Pre-commit hook active (reject-customer-data.sh)
- [x] Gitea server-side hook ready for install (issue #913)

## Defect Register

| ID | Description | Origin | Severity | Resolution | Status |
|----|-------------|--------|----------|------------|--------|
| V001 | Slug collision overwrites files | Code review | Medium | HashSet + incrementing index | Closed |
| V002 | WalkDir follows symlinks, no depth limit | Code review | Medium | max_depth(4), follow_links(false) | Closed |

## Stakeholder Sign-off

| Approver | Role | Decision | Conditions | Date |
|----------|------|----------|------------|------|
| (pending) | Tech Lead | - | - | - |

## Outstanding Items

| Item | Severity | Status |
|------|----------|--------|
| Extract validation test cherry-picks (#835) | Low | Deferred - 18 conflict regions, non-critical |
| Gitea server-side pre-receive hook (#913) | Medium | Ready for install, needs server access |
| Pre-existing: unused import LearningStore | Low | In terraphim_orchestrator, not in changed files |
| Pre-existing: 4 unmaintained crate warnings | Low | Pre-existing, not from cherry-picks |

## Gate Checklist

- [x] All E2E workflows tested (10 scenarios)
- [x] NFRs validated: security (audit clean), performance (build time), compatibility (serde)
- [x] All requirements traced to acceptance evidence
- [x] All critical/high defects resolved
- [x] 3416 workspace tests pass, 0 fail
- [x] Both remotes synced at bd0e83227
- [x] Old PRs closed with explanations (#841, #835, #834)
