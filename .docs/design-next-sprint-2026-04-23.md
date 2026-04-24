# Implementation Plan: Next Sprint (Post #707)

**Status**: Draft
**Research Doc**: `.docs/research-next-work-2026-04-23.md`
**Date**: 2026-04-23

## Overview

Three issues in priority order. Each is a single PR. Total estimated: ~900 LOC.

## Sprint Plan

### Issue 1: #779 -- Fix test_api_client_search assertion
**Size**: ~50 LOC | **Priority**: 44 | **Type**: Bug fix

**What**: `test_api_client_search` in integration tests has hardcoded `response.results.len() <= 5` assertion that fails because the KG returns more results than expected.

**Steps**:
1. Read `crates/terraphim_agent/tests/integration_test.rs` around line 70
2. Understand test intent
3. Fix assertion to be data-independent (use server-provided limit, or assert on structure not count)
4. Verify `cargo test` passes

### Issue 2: #795 -- Wire robot mode JSON into session commands
**Size**: ~350 LOC | **Priority**: 0 | **Type**: Feature

**What**: Session commands (`/sessions list`, `/sessions search`, `/sessions show`, etc.) currently only output human-readable text. Wire in `--robot` flag support using our `BudgetEngine` + `RobotResponse` envelope.

**Key changes**:
- `crates/terraphim_agent/src/main.rs` -- detect robot mode in session command handling
- Create session-specific output types that wrap results in `RobotResponse<T>`
- Apply `BudgetEngine` for result limiting
- Return appropriate `ExitCode` from session commands

**Dependencies**: #707 (DONE -- BudgetEngine available)

### Issue 3: #697 -- Phase 1 test suite
**Size**: ~500 LOC | **Priority**: 0 | **Type**: Tests

**What**: Comprehensive tests for Phase 1 features: forgiving parser, robot output, integration.

**Categories**:
1. Forgiving parser unit tests (exact match, typo correction, alias expansion)
2. Robot output unit tests (JSON formatting, exit codes, schema validation)
3. Integration tests (end-to-end command dispatch, robot mode flags)

**Dependencies**: #707 (DONE), #795 (ideally done first so robot mode is fully wired)

## In Scope

- #779 fix
- #795 robot mode wiring for sessions
- #697 Phase 1 test suite

## Out of Scope

- #794 session persistence (orthogonal)
- #766 probe architecture (too large)
- #578 agent_evolution wiring (complex)

## Avoid At All Cost

- Adding new dependencies
- Refactoring existing session command handlers beyond what's needed
- Building generic abstractions for robot mode output
