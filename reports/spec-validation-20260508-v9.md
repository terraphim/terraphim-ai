# Spec Validation Report -- 2026-05-08 (v9)

**Agent**: Carthos (Domain Architect, pr-spec-validator)
**Run**: ADF variables absent; self-directed scan of three PRs without adf/spec status

## Results

| PR | Title | Verdict | Status Posted |
|----|-------|---------|---------------|
| #1360 | Fix #1355: add --no-fail-fast to cargo test --workspace | n/a (CI YAML only) | success |
| #1356 | fix(test): use unique temp path to prevent parallel test interference | pass | success |
| #1349 | Fix #251: enforce RetryBound invariant in Symphony on_retry_timer | concerns | success |

## PR#1360 -- Skipped (no spec-relevant changes)
Only `.github/workflows/*.yml` changed. No Rust sources or `plans/` paths.

## PR#1356 -- Pass
- REQ-001 (#1340): unique temp path in `test_tool_index_save_and_load` -- impl + self-verifying
- REQ-002/003/004 (#1331): rustdoc intra-doc link fixes in `terraphim_Database` and `terraphim_roleGraph` -- verified by `cargo doc` zero-warnings in `reports/doc-gap-report-20260508.md`
- Minor note: `subsec_nanos()` uniqueness is adequate but `tempfile::NamedTempFile` would be OS-guaranteed

## PR#1349 -- Concerns
- REQ-003 (#251): `max_retry_attempts()` config accessor -- covered by two unit tests
- REQ-001/002 (#251, TLA+ RetryBound): orchestrator `on_retry_timer()` RetryGiveUp paths -- **no test coverage**
- Recommendation: add unit tests for both no-slots and poll-failure RetryGiveUp paths

## Comments Posted
- PR#1356 comment id: 22755
- PR#1349 comment id: 22758
