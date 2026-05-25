# Validation Report: Local `.terraphim/` Config Priority

**Status**: Validated
**Date**: 2026-05-25
**Stakeholders**: Maintainer request via issue #1862 / PR #1865
**Research Docs**:
- `.docs/research-1862-local-terraphim-config.md`
- `.docs/research-1862-review-fixes.md`
**Design Docs**:
- `.docs/design-1862-local-terraphim-config.md`
- `.docs/design-1862-review-fixes.md`
**Verification Report**: `.docs/verification-1862-local-terraphim-config.md`
**Validation Commit**: `1ae43f1ce`

## Executive Summary

The implementation meets the original requirement that local `.terraphim/` configuration is discovered before global/default configuration across the CLI tools in scope. The review-fix implementation validates the intended user outcomes: local role/thesaurus discovery is deterministic, grep supports non-LLM builds, agent config merges preserve shortcut-only project settings, and MCP project merges preserve selected/default role validity.

## Acceptance Criteria

| ID | Acceptance Criterion | Evidence | Status |
|----|---------------------|----------|--------|
| AC-001 | `terraphim_grep` can compile without LLM while still supporting project config discovery | `cargo check -p terraphim_grep --no-default-features --features code-search` | PASS |
| AC-002 | `terraphim_grep` resolves project role before local thesaurus/role lookup | `ProjectConfig::resolve_role_name()` tests and grep compile/tests | PASS |
| AC-003 | Multi-role project config does not silently choose a random role | `test_resolve_role_ambiguous_multi_role_without_default` | PASS |
| AC-004 | Explicit CLI role remains highest priority | `test_resolve_role_prefers_explicit_role` | PASS |
| AC-005 | `terraphim_agent` merges project config with only `global_shortcut` | `ProjectConfig::is_empty()` semantics and agent clippy/build | PASS |
| AC-006 | `terraphim_mcp_server` selected/default role remains valid after project merge | MCP tests for repair and single-project-role selection | PASS |

## System Test Results

### End-to-End Scenarios

| ID | Workflow | Steps | Result | Status |
|----|----------|-------|--------|--------|
| E2E-001 | Grep non-LLM CLI build | Build `terraphim_grep` with `--no-default-features --features code-search` | Build passed | PASS |
| E2E-002 | Project role resolution | Run project-config unit scenarios for explicit, selected, default, single, and ambiguous roles | 25 project tests passed | PASS |
| E2E-003 | Grep LLM-enabled compatibility | Run grep lib tests with `code-search llm` | 24 tests passed | PASS |
| E2E-004 | MCP project merge validity | Run MCP binary tests | 2 tests passed | PASS |

### Non-Functional Requirements

| Category | Target | Actual | Status |
|----------|--------|--------|--------|
| Build health | All relevant builds pass | Passed | PASS |
| Static quality | Clippy `-D warnings` clean | Passed | PASS |
| Formatting | Rustfmt clean | Passed | PASS |
| Security | No new auth/credential/network surface | No new security boundary introduced | PASS |
| Performance | No hot-path regression | Startup-only config discovery; no search loop changes | PASS |

## Acceptance Testing Plan and Execution

### Environment

- Repository: `/Users/alex/projects/terraphim/terraphim-ai`
- Branch: `task/1862-local-terraphim-config-priority`
- Commit: `1ae43f1ce`
- Execution mode: local CLI/library validation

### Scenarios

#### AT-001: Local project config discovery does not require LLM

**Preconditions:** `terraphim_grep` is built without default features and with `code-search`.

**Steps:**
1. Run `cargo check -p terraphim_grep --no-default-features --features code-search`.

**Expected:** Build succeeds.

**Actual:** Build succeeded.

**Status:** PASS.

#### AT-002: Project role resolution is deterministic

**Preconditions:** ProjectConfig contains role files and optional default/selected role metadata.

**Steps:**
1. Run `cargo test -p terraphim_config -- project --nocapture`.
2. Confirm explicit role, single role, selected role, default role, and ambiguous multi-role cases pass.

**Expected:** Deterministic role resolution; ambiguous multi-role without default errors.

**Actual:** 25 tests passed.

**Status:** PASS.

#### AT-003: MCP server preserves valid selected/default roles after project merge

**Preconditions:** Project config contributes a role to an existing MCP base profile.

**Steps:**
1. Run `cargo test -p terraphim_mcp_server --bin terraphim_mcp_server -- --nocapture`.

**Expected:** Selected/default roles point at existing roles after merge.

**Actual:** 2 tests passed.

**Status:** PASS.

#### AT-004: Existing grep LLM-enabled behaviour remains compatible

**Preconditions:** `terraphim_grep` is tested with `code-search llm` features.

**Steps:**
1. Run `cargo test -p terraphim_grep --features "code-search llm" --lib -- --nocapture`.

**Expected:** Existing grep tests pass.

**Actual:** 24 tests passed.

**Status:** PASS.

## Requirements Traceability

| Requirement | Acceptance Evidence | Verification Evidence | Status |
|-------------|---------------------|-----------------------|--------|
| Local `.terraphim/` config shall be first priority after CLI flags | AC-002, AT-002 | `resolve_role_name()` and grep lookup changes | PASS |
| CLI flags shall override auto-discovery | AC-004 | `test_resolve_role_prefers_explicit_role` | PASS |
| All CLI tools shall support local project config | AC-001, AC-005, AC-006 | grep, agent, MCP checks | PASS |
| Prior review findings shall be resolved | AC-001 to AC-006 | Verification report defect register | PASS |

## Defect Register

| ID | Description | Origin Phase | Severity | Resolution | Status |
|----|-------------|--------------|----------|------------|--------|
| V001 | UBS whole-crate scan reports pre-existing findings in `terraphim_config` | Outside this PR scope | Medium | Documented as caveat; no changed-code blocker identified | Deferred |

## Stakeholder Sign-off

Formal human sign-off is pending PR review/merge decision. Technical validation is complete and supports proceeding to re-review.

| Stakeholder | Role | Decision | Conditions | Date |
|-------------|------|----------|------------|------|
| Maintainer | Reviewer / approver | Pending | Structural re-review recommended | 2026-05-25 |

## Gate Checklist

- [x] All end-to-end CLI workflows in scope validated
- [x] NFRs from research validated where applicable
- [x] Requirements traced to acceptance evidence
- [x] Critical/high review defects resolved and re-verified
- [x] Deployment conditions documented
- [x] Ready for structural re-review

## Validation Decision

Validated. The change meets the stated functional acceptance criteria and is ready for PR re-review. The only deferred concern is pre-existing UBS noise from a whole-crate scan, which should be tracked separately if the team wants a dedicated cleanup issue.
