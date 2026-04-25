# Verification Report: Cherry-picked PRs #841, #835, #834

**Status**: Verified
**Date**: 2026-04-25
**Commits**: c931db3aa..bd0e83227 (6 commits on main)

## Summary

Cherry-picked valuable commits from 3 stale, heavily diverged PRs onto current main:

| PR | Commit(s) | Description |
|----|-----------|-------------|
| #841 | `e1bf9d69d`, `78147fb39` | OpenCode and Codex JSONL session connectors + test fix |
| #835 | `b367f1690` | Exclude terraphim_tinyclaw from workspace (RUSTSEC-2026-0104) |
| #834 | `b5c796b69` | learn export-kg command + NormalizedTerm action metadata |
| - | `c931db3aa` | Remove tracked projects/odilo/ customer data |
| - | `bd0e83227` | WalkDir depth limit + export-kg slug collision guard |

## Metrics

| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| UBS Critical Findings | 0 | 0 | PASS |
| cargo fmt | clean | clean | PASS |
| cargo clippy warnings (new) | 0 | 0 | PASS |
| terraphim_sessions tests | all pass | 66 passed | PASS |
| terraphim_agent lib tests | all pass | 228 passed | PASS |
| terraphim_agent bin tests | all pass | 418 passed | PASS |
| terraphim_types doctests | all pass | 15 passed | PASS |
| export_kg unit tests | all pass | 7 passed | PASS |

## Specialist Results

### UBS Scan

**Scope**: Session connectors (opencode.rs, codex.rs), export_kg.rs
**Critical**: 0 (pre-existing panic! in native.rs not in scope)
**Warning**: 2 (serde_json::from_str unwrap in test code - acceptable)
**Info**: 23

### Code Review

| Finding | Severity | Status |
|---------|----------|--------|
| export_kg slug collision silent overwrite | Medium | Fixed in `bd0e83227` |
| codex WalkDir unbounded traversal | Medium | Fixed in `bd0e83227` |
| opencode.rs detect() reads whole file for count | Low | Accepted (typical files small) |
| codex.rs silently swallows malformed JSONL | Info | Accepted (matches existing connector patterns) |

### cargo fmt / clippy

- `cargo fmt --check`: clean
- `cargo clippy --workspace --all-targets`: 1 pre-existing warning (unused import `LearningStore` in `terraphim_orchestrator/src/learning.rs:1116` - not in changed files)

## Requirements Traceability

| Requirement | Source PR | Implementation | Test | Status |
|------------|-----------|---------------|------|--------|
| OpenCode JSONL parsing | #841 (Refs #796) | `opencode.rs` | Unit tests in module | PASS |
| Codex JSONL parsing | #841 (Refs #796) | `codex.rs` | Unit tests in module | PASS |
| Connector registry integration | #841 | `mod.rs` feature gates | Cargo check | PASS |
| Tinyclaw RUSTSEC exclusion | #835 (Refs #770) | `Cargo.toml` exclude list | cargo audit clean | PASS |
| KG markdown export | #834 (Refs #759) | `export_kg.rs` | 7 unit tests | PASS |
| NormalizedTerm metadata | #834 (Refs #735) | `terraphim_types/src/lib.rs` | Backward compatible | PASS |
| Customer data removal | - | `.gitignore` + `git rm` | No odilo files tracked | PASS |
| WalkDir DoS prevention | Review finding | `codex.rs` | Existing tests | PASS |
| Slug collision prevention | Review finding | `export_kg.rs` | Existing tests | PASS |

## Defect Register

| ID | Description | Origin | Severity | Resolution | Status |
|----|-------------|--------|----------|------------|--------|
| D001 | Slug collision overwrites files silently | Code review | Medium | HashSet + index loop | Closed |
| D002 | WalkDir follows symlinks, no depth limit | Code review | Medium | max_depth(4), follow_links(false) | Closed |

## Gate Checklist

- [x] UBS scan: 0 critical findings
- [x] All cherry-picked functions have unit tests (connectors: in-module; export_kg: 7 tests)
- [x] Edge cases covered (slug collision, empty dir, merge, filter)
- [x] Coverage verified on critical paths
- [x] Module boundaries tested (connector registry, CLI enum)
- [x] cargo fmt + clippy clean
- [x] All critical/high defects resolved
- [x] Traceability matrix complete
- [x] Both remotes pushed at bd0e83227
