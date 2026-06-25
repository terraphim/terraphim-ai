# Spec Validation Report — Issue #2884

**Agent**: Carthos / spec-validator
**Date**: 2026-06-22 09:32 CEST
**Issue**: [#2884 — validate_path uses string starts_with](https://git.terraphim.cloud/terraphim/terraphim-ai/issues/2884)

## Verdict

| Scope | Verdict |
|-------|---------|
| `main` branch | FAIL |
| PR #2897 (`task/2884-validate-path-dead-code`, mergeable=True) | PASS |
| PR #2888 (`task/2884-validate-path-starts-with`, mergeable=False) | PASS |

## Acceptance Criteria

| # | Criterion | `main` | PR #2897 | PR #2888 |
|---|-----------|--------|----------|----------|
| AC1 | `validate_path` uses `path.starts_with(&self.root)` | FAIL | PASS | PASS |
| AC2 | Dead `key.contains('/')` check removed | FAIL | PASS | PASS |
| AC3 | Sibling-dir regression test added | FAIL | PASS | PASS |
| AC4 | `cargo test -p terraphim_workspace` passes | — | 27/27 | 29/29 |

## Code Evidence

### Main branch (`lib.rs:240`)
```rust
// BUG: string comparison — "/tmp/ws_evil/f" incorrectly passes against root "/tmp/ws"
let path_str = path.to_string_lossy();
let root_str = self.root.to_string_lossy();
if !path_str.starts_with(root_str.as_ref()) { ... }
// Dead code: lines 248-252 key.contains('/') check
```

### PR #2897 (`task/2884-validate-path-dead-code`, commit `70046217`)
```rust
// FIXED: component-aware Path::starts_with
fn validate_path(&self, path: &Path, _identifier: &str) -> Result<()> {
    if !path.starts_with(&self.root) { ... }
    Ok(())
}
```
Regression test: `path_prefix_confusion_is_rejected` at `lib.rs:573`

### PR #2888 (`task/2884-validate-path-starts-with`, commit `08456e26`)
Additional security hardening: `..` component pre-rejection + security-sentinel P1-1/P2-1/P2-2 fixes.
mergeable=False (conflict).

## Recommendation

Merge **PR #2897** — satisfies all acceptance criteria, mergeable=True, compound-review GO.
Security-sentinel findings from PR #2888 (cleanup() path guard, env-var stripping) should be tracked separately.

## Correction to Prior Analysis

Comment #52253 (quality-coordinator) claimed "Path::starts_with was already correct on main." This is incorrect. Direct inspection of `main:crates/terraphim_workspace/src/lib.rs:240` confirms the string `starts_with` bug is present.
