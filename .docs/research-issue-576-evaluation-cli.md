# Research Document: Issue #576 - Ground-truth Evaluation Framework CLI Integration

**Status**: Draft
**Author**: Claude
**Date**: 2026-04-16
**Gitea Issue**: #576

## Executive Summary

The ground-truth evaluation framework (`evaluate_automata`) is **already implemented** in `terraphim_automata/src/evaluation.rs` with comprehensive tests. The missing piece is CLI integration: an `evaluate` subcommand in `terraphim_cli` that accepts `--ground-truth` and `--thesaurus` flags and outputs JSON reports.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Enables measuring automata quality - critical for SFIA/Odilo pipeline |
| Leverages strengths? | Yes | Leverages existing `evaluation.rs` module with 13 unit tests |
| Meets real need? | Yes | Issue explicitly mentions Odilo pipeline, >80% agreement target |

**Proceed**: Yes - at least 2/3 YES

## Problem Statement

### Description
Wire the existing `evaluate_automata()` function to a CLI subcommand so users can run:
```bash
terraphim-cli evaluate --ground-truth ground-truth.json --thesaurus thesaurus.json
```

### What's Already Done
- `terraphim_automata/src/evaluation.rs` - Complete implementation (~613 lines)
- 13 unit tests covering all core functionality
- Types: `GroundTruthDocument`, `EvaluationResult`, `ClassificationMetrics`, `SystematicError`
- Functions: `evaluate()`, `load_ground_truth()`

### What's Missing
- CLI subcommand `evaluate` in `terraphim_cli` with `--ground-truth` and `--thesaurus` flags
- JSON output formatting for `EvaluationResult`

## Current State Analysis

### Existing Implementation
```
crates/terraphim_automata/src/evaluation.rs
├── GroundTruthDocument { id, text, expected_terms }
├── ExpectedMatch { term, category }
├── ClassificationMetrics { precision, recall, f1, tp, fp, fn }
├── TermReport { term, metrics }
├── EvaluationResult { total_documents, overall, per_term, systematic_errors }
├── SystematicError { term, false_positive_count, document_ids }
├── evaluate(ground_truth, thesaurus) -> EvaluationResult
└── load_ground_truth(path) -> Vec<GroundTruthDocument>
```

### Code Locations
| Component | Location | Purpose |
|-----------|----------|---------|
| Evaluation module | `crates/terraphim_automata/src/evaluation.rs` | Core logic (DONE) |
| Evaluation exports | `crates/terraphim_automata/src/lib.rs:116-119` | Public API (DONE) |
| CLI commands | `crates/terraphim_cli/src/main.rs` | Subcommand routing (NEEDS WORK) |
| CliService | `crates/terraphim_cli/src/service.rs` | Command implementation |

### Data Flow
```
User input (CLI)
    -> terraphim_cli evaluate command
    -> load_ground_truth(ground_truth.json)
    -> load_thesaurus(thesaurus.json)
    -> evaluate(ground_truth, thesaurus)
    -> EvaluationResult (JSON serialized)
    -> stdout
```

## Constraints

### Technical Constraints
- Must use existing `evaluate()` and `load_ground_truth()` from terraphim_automata
- Must follow existing CLI patterns in `terraphim_cli`
- JSON output for machine consumption

### Business Constraints
- Target: >80% agreement metric for Odilo pilot
- Must work with any thesaurus (domain-agnostic)

## Vital Few (Essentialism)

### Essential Constraints (Max 3)
| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Wire evaluate to CLI | Core deliverable of issue #576 | Gitea issue acceptance criteria |
| Support --ground-truth and --thesaurus flags | Explicitly requested | Issue specification |
| JSON output | Automation-friendly output | terraphim_cli purpose |

### Eliminated from Scope
| Item | Why Eliminated |
|------|----------------|
| Adding new metrics types | Already implemented in evaluation.rs |
| Confusion matrix | Not in issue acceptance criteria |
| Integration with terraphim_agent | Issue mentions terraphim_agent/main.rs but terraphim_cli is the right place |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim_automata::evaluation | Core evaluation logic | None - already implemented |
| terraphim_automata::load_thesaurus | Thesaurus loading | None - already exists |
| CliService pattern | CLI implementation style | Low |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| CLI output format mismatch | Low | Users expect different format | Follow existing JSON patterns in terraphim_cli |

### Open Questions
1. Should output be human-readable or only JSON? - terraphim_cli is automation-focused (JSON default)

## Research Findings

### Key Insights
1. **Evaluation module is complete** - 13 unit tests, handles all metrics computation
2. **Missing piece is thin** - Just need CLI wrapper around existing functions
3. **Two crates could host this**:
   - `terraphim_cli` - automation-focused, JSON output
   - `terraphim_agent` - interactive use, human-readable output
   - Recommendation: `terraphim_cli` matches the "automation-friendly" purpose

### Relevant Prior Art
- `terraphim_cli Extract` command - similar pattern (text input, JSON output)
- `terraphim_cli Coverage` command - uses `--schema` flag similar to `--thesaurus`

## Recommendations

### Proceed/No-Proceed
**Proceed** - The heavy lifting is done. Only CLI wiring remains.

### Scope
**In Scope:**
- Add `evaluate` subcommand to `terraphim_cli`
- `--ground-truth <path>` flag
- `--thesaurus <path>` flag
- JSON output of `EvaluationResult`

**Out of Scope:**
- New evaluation metrics (already done)
- Confusion matrix (not in issue)
- terraphim_agent integration (issue mentions but terraphim_cli is better fit)

## Implementation Steps (Phase 2 Design)

### Step 1: Add Evaluate subcommand to Commands enum
**File:** `crates/terraphim_cli/src/main.rs`
**Lines:** ~100
**Change:** Add `Evaluate` variant to `Commands` enum with `--ground-truth` and `--thesaurus` flags

### Step 2: Add Evaluate handler in match block
**File:** `crates/terraphim_cli/src/main.rs`
**Lines:** ~400 (estimate)
**Change:** Add match arm for `Commands::Evaluate` that calls evaluation logic

### Step 3: Wire in terraphim_automata dependencies
**File:** `crates/terraphim_cli/Cargo.toml`
**Change:** Add `terraphim_automata` dependency if not present

### Step 4: Test the evaluate command
**File:** `crates/terraphim_cli/tests/`
**Change:** Add integration test for evaluate command

## Next Steps

1. **Approve this research** - Confirm scope is correct
2. **Proceed to Phase 2.5** - Specification interview if needed
3. **Implement** - Wire the CLI command
4. **Test** - Add integration tests
