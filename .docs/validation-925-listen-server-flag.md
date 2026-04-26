# Validation Report: Bug #925 listen --server flag error

**Status**: Validated
**Date**: 2026-04-26
**Verification Report**: `.docs/verification-925-listen-server-flag.md`

## End-to-End Scenarios

| ID | Command | Expected | Actual | Status |
|----|---------|----------|--------|--------|
| E2E-001 | `--server listen --identity test-id` | stderr error, exit 2 | stderr error, exit 2 | PASS |
| E2E-002 | `listen --server test-id` (wrong flag position) | clap error, exit 2 | clap error, exit 2 | PASS |
| E2E-003 | `listen --identity test-id` (no --server) | Normal startup | Starts normally | PASS |

## NFR Verification

| Requirement | Target | Actual | Status |
|-------------|--------|--------|--------|
| No regressions | All tests pass | 228/228 | PASS |
| No new dependencies | 0 | 0 | PASS |
| Backward compatibility | Existing commands unchanged | Confirmed | PASS |

## Sign-off

Bug fixed and validated. Exit code now correctly returns ERROR_USAGE (2) instead of 1.
