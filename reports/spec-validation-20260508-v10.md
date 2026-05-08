# Spec Validation Report -- 2026-05-08 (v10)

**Agent**: Carthos (Domain Architect, pr-spec-validator)
**Run**: Standalone scan -- ADF Middleware variables absent; self-directed sweep of all open PRs

## Results

| PR | Title | Verdict | Status Posted |
|----|-------|---------|---------------|
| #1343 | Fix #1266: update NormalizedTerm initializers to builder pattern | pass (current) | skipped (already current) |
| #1347 | Fix #1340: use unique tempdir in test_tool_index_save_and_load | pass (current) | skipped (already current) |
| #1349 | Fix #251: enforce RetryBound invariant in Symphony on_retry_timer | concerns (current) | skipped (already current) |
| #1356 | fix(test): use unique temp path to prevent parallel test interference | pass (current) | skipped (already current) |
| #1360 | Fix #1355: add --no-fail-fast to cargo test --workspace | n/a (CI YAML only) | success (comment 22774, status id 47) |

## PR #1360 -- n/a (no spec-relevant changes)

Changed paths: `.github/workflows/ci-main.yml`, `.github/workflows/ci-pr.yml`

Only CI workflow YAML modified. Path filter `^plans/|^crates/[^/]+/src/|\.rs$` matches nothing.
Posted `adf/spec: success` with description "n/a no spec-relevant changes".

## Summary

All five open PRs are current against their head SHAs.
Outstanding concern from v9 remains: PR #1349 lacks unit tests for the `on_retry_timer` RetryGiveUp paths (no-slots and poll-failure branches).
