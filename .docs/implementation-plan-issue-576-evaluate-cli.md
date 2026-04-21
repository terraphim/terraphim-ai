# Implementation Plan: Issue #576 - Ground-truth Evaluation Framework CLI Integration

**Status**: Draft
**Research Doc**: `.docs/research-issue-576-evaluation-cli.md`
**Author**: Claude
**Date**: 2026-04-16
**Estimated Effort**: 2-3 hours

## Overview

### Summary
Wire the existing `evaluate_automata()` function from `terraphim_automata` to a CLI subcommand in `terraphim_cli`. The core evaluation logic is already implemented; this is CLI integration only.

### Approach
Add an `Evaluate` subcommand to `terraphim_cli` that:
1. Loads ground truth JSON from `--ground-truth` flag
2. Loads thesaurus from `--thesaurus` flag
3. Calls `evaluate()` function
4. Outputs JSON `EvaluationResult`

### Scope
**In Scope:**
- Add `Evaluate` command variant to `Commands` enum in `terraphim_cli`
- Implement `evaluate` subcommand handler in `CliService`
- Add `terraphim_automata` dependency to `terraphim_cli`
- Integration tests for evaluate command

**Out of Scope:**
- New evaluation metrics (already in evaluation.rs)
- Confusion matrix output (not in issue spec)
- terraphim_agent integration (terraphim_cli is better fit for automation)

**Avoid At All Cost:**
- Adding human-readable output (automation-focused, JSON only)
- Modifying evaluation.rs (already complete and tested)
- Adding schema-based evaluation (future work)

## Architecture

### Component Diagram
```
terraphim_cli
└── Evaluate command
    ├── load_ground_truth(path)  --> GroundTruthDocument[]
    ├── load_thesaurus(path)      --> Thesaurus
    └── evaluate(gt, thesaurus)  --> EvaluationResult
                                    ├── overall: ClassificationMetrics
                                    ├── per_term: TermReport[]
                                    └── systematic_errors: SystematicError[]
```

### Data Flow
```
CLI args (--ground-truth, --thesaurus)
    -> CliService::evaluate()
    -> terraphim_automata::load_ground_truth()
    -> terraphim_automata::load_thesaurus()
    -> terraphim_automata::evaluate()
    -> EvaluationResult (JSON serialized)
    -> stdout
```

## File Changes

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_cli/Cargo.toml` | Add `terraphim_automata` dependency |
| `crates/terraphim_cli/src/main.rs` | Add `Evaluate` variant to `Commands` enum |
| `crates/terraphim_cli/src/service.rs` | Add `evaluate()` method to `CliService` |
| `crates/terraphim_cli/tests/integration_tests.rs` | Add integration test |

## API Design

### CLI Command Signature
```rust
/// Evaluate automata classification accuracy against ground truth
Evaluate {
    /// Path to ground truth JSON file
    #[arg(long)]
    ground_truth: String,

    /// Path to thesaurus JSON file
    #[arg(long)]
    thesaurus: String,
}
```

### Types (already in terraphim_automata::evaluation)
```rust
// GroundTruthDocument - already exists
// EvaluationResult - already exists
// ClassificationMetrics - already exists
// SystematicError - already exists
```

## Implementation Steps

### Step 1: Add Evaluate to Commands enum
**File:** `crates/terraphim_cli/src/main.rs`
**Lines:** ~100-110 (after `Coverage` variant)
**Change:**
```rust
/// Evaluate automata classification against ground truth
Evaluate {
    /// Path to ground truth JSON file
    #[arg(long)]
    ground_truth: String,

    /// Path to thesaurus JSON file
    #[arg(long)]
    thesaurus: String,
},
```

### Step 2: Add terraphim_automata dependency
**File:** `crates/terraphim_cli/Cargo.toml`
**Change:**
```toml
terraphim-automata = { path = "../terraphim_automata", features = [] }
```

### Step 3: Add evaluate() method to CliService
**File:** `crates/terraphim_cli/src/service.rs`
**Change:** Add method that:
1. Loads ground truth via `terraphim_automata::load_ground_truth()`
2. Loads thesaurus via `terraphim_automata::load_thesaurus()`
3. Calls `terraphim_automata::evaluate()`
4. Serializes and prints result as JSON

### Step 4: Wire Evaluate in main.rs match
**File:** `crates/terraphim_cli/src/main.rs`
**Location:** ~400 (in the Command::run() match)
**Change:**
```rust
Commands::Evaluate { ground_truth, thesaurus } => {
    cli_service.evaluate(&ground_truth, &thesaurus).await?;
}
```

### Step 5: Add integration test
**File:** `crates/terraphim_cli/tests/integration_tests.rs`
**Change:** Add test that:
1. Creates temporary ground truth JSON
2. Creates temporary thesaurus JSON
3. Runs evaluate command
4. Verifies JSON output contains expected fields

## Test Strategy

### Unit Tests
No new unit tests needed - evaluation logic is already tested in `terraphim_automata`

### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_evaluate_command_success` | `integration_tests.rs` | Full flow with valid files |
| `test_evaluate_command_missing_ground_truth` | `integration_tests.rs` | Error handling |
| `test_evaluate_command_missing_thesaurus` | `integration_tests.rs` | Error handling |

### Test Data
Use temporary files created in tests (no external fixtures needed)

## Dependencies

### New Dependencies
| Crate | Version | Justification |
|-------|---------|---------------|
| terraphim-automata | path | Required for evaluation module |

## Performance Considerations

- Evaluation is O(n*m) where n=documents, m=thesaurus terms
- Expected: <100ms for typical workloads
- No benchmarks needed for CLI wrapper

## Rollback Plan

If issues discovered:
1. Remove `Evaluate` variant from `Commands` enum
2. Remove `evaluate()` method from `CliService`
3. Remove `terraphim_automata` dependency

## Simplicity Check

**What if this could be easy?**
This IS simple - just 4 files to modify, no new types, no complex logic. The evaluation module is already complete. This is a thin CLI wrapper.

## Approval Gate

- [ ] Research document reviewed and approved
- [ ] Implementation plan reviewed
- [ ] File changes defined
- [ ] Test strategy defined
- [ ] Human approval received
