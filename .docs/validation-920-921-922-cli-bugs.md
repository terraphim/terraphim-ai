# Validation Report: CLI Bugs #920, #921, #922

**Status**: Validated
**Date**: 2026-04-26
**Verification Report**: `.docs/verification-920-921-922-cli-bugs.md`

## End-to-End Scenarios

| ID | Command | Expected | Actual | Status |
|----|---------|----------|--------|--------|
| E2E-001 | `replace --json --fail-open` with no KG | Unchanged text, exit 0 | `{"result":"npm install express","replacements":0}` exit 0 | PASS |
| E2E-002 | `replace --json` with no KG | Unchanged text, exit 0 | `{"result":"npm install express","replacements":0}` exit 0 | PASS |
| E2E-003 | `suggest "rust" --json` | Empty array, exit 0 | `[]` exit 0 | PASS |
| E2E-004 | `validate "text" --json` | 0 matches, exit 0 | `{"matched_count":0}` exit 0 | PASS |
| E2E-005 | `extract "text"` | "No matches found", exit 0 | "No matches found in the text." exit 0 | PASS |
| E2E-006 | `check-update` (rate limited) | Helpful error message | Shows "GitHub API rate limit exceeded. Set GITHUB_TOKEN..." exit 0 | PASS |

## NFR Verification

| Requirement | Target | Actual | Status |
|-------------|--------|--------|--------|
| No regressions | All existing tests pass | 259/259 tests pass | PASS |
| Build time | < 60s incremental | ~10s | PASS |
| No new dependencies | 0 added | 0 added | PASS |
| Backward compatibility | KG-configured roles unchanged | Thesaurus still loads normally | PASS |

## Sign-off

All three bugs fixed and validated. No regressions detected.
