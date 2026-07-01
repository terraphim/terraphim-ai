# Implementation Plan: Unified Memory Lifecycle CLI + Reliability Rubric

**Status**: Draft
**Canonical Path**: `.docs/design-issue-1899-memory-lifecycle.md`
**Change Slug**: `issue-1899-memory-lifecycle`
**Research**: `.docs/research-issue-1899-memory-lifecycle.md`
**Gitea Issue**: https://git.terraphim.cloud/terraphim/terraphim-ai/issues/1899
**Author**: OpenCode Agent (disciplined-design)
**Date**: 2026-07-01
**Estimated Effort**: ~5 days of agent time (6 PRs)

## Overview

### Summary

Add a `terraphim-agent memory` CLI namespace wrapping the eight-stage agentic memory lifecycle. Six of eight stages already exist as working primitives; this plan consolidates them behind a single discoverable surface and adds the Validate, Retire, and Rubric stages.

### Approach

CLI facade pattern: each `memory` subcommand delegates to an existing handler.

```
terraphim-agent memory capture    -> calls `learn capture/hook` handler
terraphim-agent memory distill    -> calls `learn compile + export-kg` handler
terraphim-agent memory scope      -> reads role + project KG boundaries
terraphim-agent memory provenance -> calls `sessions` handler with --memory-id filter
terraphim-agent memory retrieve   -> calls `search` handler with explicit role
terraphim-agent memory apply      -> calls `terraphim_hooks` diff against prompt
terraphim-agent memory validate   -> NEW: runs rubric scorer on memory items
terraphim-agent memory retire     -> NEW: proposes demotions, writes learned-rules.md
terraphim-agent memory rubric     -> NEW: standalone diagnostic (6 dimensions, markdown)
terraphim-agent memory second-run -> NEW: reads ADF artefacts, computes token delta
```

Seven of ten subcommands are pure routing (delegation to existing handlers). Three are net-new code: `validate`, `rubric`, `second-run`.

### Scope

**In Scope:**
- `terraphim-agent memory` CLI namespace with 10 subcommands
- Wire `terraphim_agent_evolution::{MemoryEvolution, LessonsEvolution}` into CLI list/show/export
- Memory Reliability Rubric (6 dimensions: Faithfulness, Scope, Provenance, Actionability, Decay, Risk)
- `MEMORY_POLICY.md` at repo root distinguishing public commons from permissioned memory
- Second-run signal emission from ADF artefact directory
- Reuse `terraphim_persistence` `Persistable` trait (no new schema)

**Out of Scope:**
- New persistence backend
- Cross-organisation memory federation
- UI/dashboard work
- Replacing `/evolve` skill (stays as human-in-the-loop)
- Re-implementing judge pipeline (we call the existing one)

**Avoid At All Cost:**
- New crate (59th) -- CLI surface lives in existing `terraphim_agent`
- New persistence schema -- lifecycle metadata travels in existing `Persistable` types
- Dependency on buildable main -- mock/spike in isolation if main is still broken (#3030)

## Architecture

### Component Diagram

```
┌──────────────────────────────────────────────────────────────────┐
│                    terraphim-agent binary                         │
│                                                                  │
│  ┌──────────────────┐   ┌──────────────────┐                    │
│  │  learn command   │   │ sessions command │                    │
│  │  (capture,       │   │  (provenance)    │                    │
│  │   distill,       │   │                  │                    │
│  │   retrieve)      │   │                  │                    │
│  └────────┬─────────┘   └────────┬─────────┘                    │
│           │                      │                               │
│  ┌────────▼──────────────────────▼──────────────────┐           │
│  │              memory command (NEW)                │           │
│  │  ┌──────────┬──────────┬──────────┬───────────┐ │           │
│  │  │ capture  │ distill  │ scope    │ provenance│ │           │
│  │  │ retrieve │ apply    │ validate │ retire    │ │           │
│  │  │ rubric   │second-run│          │           │ │           │
│  │  └──────────┴──────────┴──────────┴───────────┘ │           │
│  └─────────────────────┬────────────────────────────┘           │
│                        │                                        │
│  ┌─────────────────────▼────────────────────────────┐           │
│  │            memory::handler module (NEW)           │           │
│  │  ┌─────────┐ ┌──────────┐ ┌──────────────────┐  │           │
│  │  │ handlers│ │ rubric   │ │ second_run       │  │           │
│  │  │ (router)│ │ (scorer) │ │ (token_delta)    │  │           │
│  │  └────┬────┘ └────┬─────┘ └────────┬─────────┘  │           │
│  └───────┼───────────┼───────────────┼─────────────┘           │
│          │           │               │                          │
└──────────┼───────────┼───────────────┼──────────────────────────┘
           │           │               │
           ▼           ▼               ▼
  ┌────────────┐ ┌──────────┐ ┌───────────────┐
  │ learn      │ │ judge    │ │ ADF artefacts │
  │ sessions   │ │ pipeline │ │ directory     │
  │ hooks      │ │ (Kimi    │ │               │
  │ search     │ │ K2.5)    │ │               │
  └────────────┘ └──────────┘ └───────────────┘
```

### Data Flow

```
User/ADF agent runs: terraphim-agent memory capture

  terraphim-agent memory capture
      -> memory::handler::handle_capture()
          -> delegates to existing learn::handle_capture()
              -> writes to captured-corrections store
                  -> persistable

User runs: terraphim-agent memory rubric --project ~/project

  terraphim-agent memory rubric
      -> memory::rubric::run_rubric()
          -> reads memory items from persistence
          -> calls judge pipeline with rubric prompt
              -> judge scores 6 dimensions per item
          -> formats markdown readout
          -> prints to stdout
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|---|---|---|
| CLI facade, not new crate | #1899 explicitly forbids 59th crate; reuse existing patterns | New `terraphim_memory_cli` crate -- rejected, governance cost |
| Route through existing handlers for 7/10 subcommands | Zero risk of breaking existing behaviour | Rewrite capture/distill/etc -- rejected, too large |
| Use `terraphim_agent_evolution` directly, not via orchestrator's EvolutionManager | CLI is lightweight; orchestrator wrapper is heavyweight (tokio runtime, HashMap of systems) | Route through EvolutionManager -- rejected, unnecessary for CLI |
| Judge pipeline called as external process | Judge is a separate component in cto-executive-system; calling it preserves independence | Inline judge in Rust -- rejected, duplication |
| Second-run signal reads from ADF artefact directory (filesystem) | Simplest; no telemetry dependency | Query telemetry store -- rejected, tighter coupling |

### Simplicity Check

**What if this could be easy?**

The simplest design is a flat `mod memory` inside `terraphim_agent` with 10 match arms routing to existing `learn`, `sessions`, and `search` handlers plus 3 new functions for rubric/second-run. No new crate. No new traits. No new dependency graph changes (except `terraphim_agent_evolution` if opted). This is 2-3 files of ~200 lines each plus the rubric prompt.

**Senior Engineer Test**: Would this be called overcomplicated? No. It's a well-understood facade pattern following the existing `learn` and `sessions` precedent.

**Nothing Speculative Checklist:**
- [x] No features the user didn't request -- all 10 subcommands from #1899
- [x] No abstractions "in case we need them later" -- flat module, match arms
- [x] No flexibility "just in case" -- no plugin system for memory backends
- [x] No error handling for scenarios that cannot occur -- standard `anyhow::Result`
- [x] No premature optimisation -- Aho-Corasick is already fast enough

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|---|---|---|
| New `terraphim_memory` crate | #1899 explicitly "do NOT introduce a 59th crate" | Governance cost, publish overhead |
| REST/HTTP API surface for memory | Out of scope; CLI-only per #1899 | Scope creep, authentication complexity |
| Memory dashboard UI | Out of scope; separate project | UI work unrelated to CLI consolidation |
| New memory storage backend (SQLite, Postgres) | Keep `terraphim_persistence` v1.20.x | Schema migration risk, scope creep |
| Streaming/async rubric (per-capture validation) | Rubric is on-demand; per-capture costs too much | Judge pipeline cost spike |

## Expected Lifecycle Artefacts

| Artefact | Path | Required? |
|---|---|---|
| Specification | `.docs/spec-issue-1899-memory-lifecycle.spec.md` | Yes (CLI behaviour spec) |
| ADR | Not needed -- no architectural decision beyond what's in this plan | No |
| Contract | Not needed -- no network interface changes | No |
| Verification | `.docs/verification-issue-1899-memory-lifecycle.md` | Yes |
| Validation | `.docs/validation-issue-1899-memory-lifecycle.md` | Yes (user-acceptance of memory commands) |

## File Changes

### New Files

| File | Purpose |
|---|---|
| `crates/terraphim_agent/src/commands/memory.rs` | Memory command definitions and CLI subcommand types |
| `crates/terraphim_agent/src/commands/memory/handler.rs` | Memory subcommand handlers (router + new logic) |
| `crates/terraphim_agent/src/commands/memory/rubric.rs` | Rubric scorer: 6-dimension judge prompt + markdown formatter |
| `crates/terraphim_agent/src/commands/memory/second_run.rs` | Second-run token delta from ADF artefacts |
| `MEMORY_POLICY.md` | Public commons vs permissioned memory boundary |

### Modified Files

| File | Changes |
|---|---|
| `crates/terraphim_agent/src/commands/mod.rs` | Add `pub mod memory;` (assuming module exists; if not, add to command tree) |
| `crates/terraphim_agent/src/repl/commands.rs` | Add `Memory` variant to `ReplCommand` enum + `MemorySubcommand` enum |
| `crates/terraphim_agent/src/repl/handler.rs` | Add match arm for `Memory` => route to `memory::handler` |
| CLI binary entry point (recovered from agents repo) | Register `memory` as top-level subcommand |
| `README.md` | Link to `MEMORY_POLICY.md` |

Note: The exact file locations depend on the source recovered from the terraphim-agents repo since `main` is missing `Cargo.toml` and `lib.rs` for `terraphim_agent`. The structure follows the existing `learn`/`sessions` pattern found in the recovered binary.

## API Design

### CLI Surface

```
terraphim-agent memory
  capture      [--provenance-tag <TAG>]                  # Alias of `learn hook`
  distill      [--format json|markdown]                   # Alias of `learn compile` + `learn export-kg`
  scope        [--role <ROLE>] [--project <PATH>] [--check] # Show/edit role + project boundaries
  provenance   [--memory-id <ID>]                         # Alias of `sessions search`
  retrieve     [--role <ROLE>] [--scope <SCOPE>] <QUERY>  # Alias of `search`
  apply        [--prompt <TEXT>]                          # Show what hooks would inject
  validate     [--all | --lesson-id <ID>]                 # Run rubric on items
  retire       [--lesson-id <ID>] [--reason <TEXT>]       # Propose demotion (CTO flag)
  rubric       --project <PATH> [--output <FILE>]         # Standalone diagnostic
  second-run   --issue <GITEA_ISSUE_N>                    # Token delta between runs
  help         [SUBCOMMAND]                                # Print help
```

### Public Types (Rust)

```rust
/// Memory lifecycle subcommands
#[derive(Debug, Clone, clap::Subcommand)]
pub enum MemoryCommand {
    /// Capture a failed command or session event as a memory item
    Capture {
        #[arg(long)]
        provenance_tag: Option<String>,
    },
    /// Distill captured learnings into thesaurus and KG entries
    Distill {
        #[arg(long, default_value = "markdown")]
        format: String,
    },
    /// Show or check role and project memory boundaries
    Scope {
        #[arg(long)]
        role: Option<String>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        check: bool,
    },
    /// Search session provenance for a memory ID
    Provenance {
        #[arg(long)]
        memory_id: Option<String>,
    },
    /// Retrieve memory items by query within role scope
    Retrieve {
        #[arg(long)]
        role: Option<String>,
        #[arg(long)]
        scope: Option<String>,
        query: String,
    },
    /// Show hooks that would apply to a given prompt
    Apply {
        #[arg(long)]
        prompt: Option<String>,
    },
    /// Validate memory items against reliability rubric
    Validate {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        lesson_id: Option<String>,
    },
    /// Propose retirement of a memory item
    Retire {
        #[arg(long)]
        lesson_id: Option<String>,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Run full reliability rubric diagnostic on a project
    Rubric {
        #[arg(long)]
        project: String,
        #[arg(long)]
        output: Option<String>,
    },
    /// Compute token delta between two ADF runs of the same issue
    SecondRun {
        #[arg(long)]
        issue: u64,
    },
}

/// Reliability rubric dimensions and scores
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RubricScore {
    pub faithfullness: f64,     // 0.0-1.0
    pub scope: f64,             // 0.0-1.0
    pub provenance: f64,        // 0.0-1.0
    pub actionability: f64,    // 0.0-1.0
    pub decay: f64,            // 0.0-1.0 (1.0 = fully current)
    pub risk: f64,             // 0.0-1.0 (0.0 = no risk)
    pub composite: f64,         // weighted average
}

impl RubricScore {
    pub fn composite(&self) -> f64 {
        // Weights: faithfulness 0.3, actionability 0.25, scope 0.15,
        //          provenance 0.10, decay 0.10, risk 0.10
        0.30 * self.faithfullness +
        0.25 * self.actionability +
        0.15 * self.scope +
        0.10 * self.provenance +
        0.10 * self.decay +
        0.10 * (1.0 - self.risk) // invert risk so high risk = low score
    }
}

/// Second-run signal: difference between two ADF runs
#[derive(Debug, Clone, serde::Serialize)]
pub struct SecondRunSignal {
    pub gitea_issue: u64,
    pub run_1: RunMetrics,
    pub run_2: RunMetrics,
    pub delta: RunDelta,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RunMetrics {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub wall_time_seconds: f64,
    pub retry_count: u32,
    pub hook_injected_bytes: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RunDelta {
    pub tokens_saved: i64,          // negative = saved (fewer in run 2)
    pub retries_avoided: i32,       // negative = fewer retries
    pub hooks_newly_applicable: u32, // new hooks that fired in run 2
    pub wall_time_delta_seconds: f64, // negative = faster
}
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|---|---|---|
| `test_memory_command_help` | `memory/handler.rs` | Verify --help lists 10 subcommands |
| `test_capture_routes_to_learn` | `memory/handler.rs` | Verify capture => learn delegate |
| `test_distill_routes_to_learn` | `memory/handler.rs` | Verify distill => learn delegate |
| `test_rubric_score_calculation` | `memory/rubric.rs` | Verify composite scoring math |
| `test_rubric_score_range` | `memory/rubric.rs` | All dimension scores in [0,1] |
| `test_second_run_delta_calculation` | `memory/second_run.rs` | Verify delta arithmetic |
| `test_second_run_empty_artefacts` | `memory/second_run.rs` | Graceful error on missing data |
| `test_scope_check_permissioned_violation` | `memory/handler.rs` | Detect permissioned item in public location |

### Integration Tests

| Test | Location | Purpose |
|---|---|---|
| `test_memory_capture_e2e` | `terraphim_agent/tests/` | Full capture flow via memory command |
| `test_memory_rubric_creates_markdown` | `terraphim_agent/tests/` | Rubric produces valid markdown output |
| `test_memory_reliability_rubric_smoke` | `terraphim_agent/tests/` | < 10 min for a small project |
| `test_second_run_signal_json_output` | `terraphim_agent/tests/` | Valid JSON with required fields |
| `test_memory_policy_exists` | Build-time check | Verify `MEMORY_POLICY.md` at root |

## Implementation Steps

### Step 0: Prerequisites -- Fix main branch (#3030)

**Files:** `crates/terraphim_agent/Cargo.toml`, `crates/terraphim_agent/src/lib.rs`, `crates/terraphim_agent/src/main.rs`

**Description:** Recover missing crate files from the terraphim-agents repository so that the workspace builds. This is a hard prerequisite: without a buildable workspace, no integration tests can run.

**Tests:** `cargo check -p terraphim_agent` passes.

**Dependencies:** None.

**Estimated:** 2 hours.

Note: If fixing #3030 removes the god-file decomposition artefacts (i.e., restores a working single-file lib.rs rather than the incomplete module split), we may need to adjust the module structure in Steps 1-3 accordingly.

### Step 1: Scaffold Memory CLI Namespace

**Files:** `crates/terraphim_agent/src/commands/memory/mod.rs`, `crates/terraphim_agent/src/commands/memory/handler.rs`, `crates/terraphim_agent/src/commands/mod.rs`

**Description:** Create the `memory` module with empty subcommand definitions using clap derive. Register `memory` as a top-level subcommand in the binary. Wire 7 routing subcommands (capture, distill, scope, provenance, retrieve, apply, retire) to their existing `learn`/`sessions`/`search`/`hooks` handler delegates.

**Tests:** Unit tests for subcommand parse + routing. Integration test: `terraphim-agent memory capture --help` works.

**Dependencies:** Step 0.

**Estimated:** 4 hours.

```rust
// crates/terraphim_agent/src/commands/memory/mod.rs
pub mod handler;
pub mod rubric;
pub mod second_run;

#[derive(Debug, Clone, clap::Subcommand)]
pub enum MemoryCommand {
    Capture { ... },
    Distill { ... },
    Scope { ... },
    Provenance { ... },
    Retrieve { ... },
    Apply { ... },
    Validate { ... },
    Retire { ... },
    Rubric { ... },
    SecondRun { ... },
}
```

### Step 2: Wire MemoryEvolution + LessonsEvolution into CLI

**Files:** `crates/terraphim_agent/src/commands/memory/handler.rs`, `crates/terraphim_agent/Cargo.toml`

**Description:** Add `terraphim_agent_evolution` as an optional dependency (gated behind `evolution` feature, matching orchestrator pattern). Implement `memory list`, `memory show`, and `memory export` that read from `MemoryEvolution` and `LessonsEvolution`. Implement `memory capture` (provenance tags) and `memory scope` (show/edit role boundaries).

**Tests:** Integration tests exercising `MemoryEvolution` read path through CLI. Feature-gated tests (with `evolution` feature).

**Dependencies:** Step 1.

**Estimated:** 8 hours.

### Step 3: Rubric Subcommand + Judge Pipeline Integration

**Files:** `crates/terraphim_agent/src/commands/memory/rubric.rs`

**Description:** The rubric subcommand:
1. Reads memory items from the project's persistence store
2. Constructs a judge prompt template with the 6 dimensions and scoring criteria
3. Calls the judge pipeline (CTO's Kimi K2.5 deep tier) via process spawning
4. Parses judge response into `RubricScore` per item
5. Computes composite scores
6. Formats a markdown readout: per-dimension scores, top three offenders, three recommended retirements

**Tests:** Unit tests for score calculation and prompt template formatting. Integration test: rubric run on a small project completes in < 10 min.

**Dependencies:** Step 2 (needs MemoryEvolution for item reading).

**Estimated:** 12 hours.

### Step 4: Second-Run Signal Emission

**Files:** `crates/terraphim_agent/src/commands/memory/second_run.rs`

**Description:** The second-run subcommand:
1. Takes `--issue <GITEA_ISSUE_N>` parameter
2. Reads ADF artefact directory for that issue (telemetry JSON per run)
3. Extracts input_tokens, output_tokens, wall_time, retries, hook_bytes
4. Computes deltas: tokens_saved, retries_avoided, hooks_newly_applicable, wall_time_delta
5. Outputs `SecondRunSignal` as JSON to stdout

**Tests:** Unit tests for delta arithmetic. Integration test with fixture ADF artefact directory.

**Dependencies:** Step 2.

**Estimated:** 8 hours.

### Step 5: MEMORY_POLICY.md + Scope Enforcement

**Files:** `MEMORY_POLICY.md` (repo root), `crates/terraphim_agent/src/commands/memory/handler.rs`

**Description:** Write `MEMORY_POLICY.md` documenting:
- Public commons memory: terraphim-skills repo, Gitea wiki KG entries, shared automata thesauri (Apache-2.0, no PII)
- Permissioned memory: per-project KGs, agent corrections, session transcripts (local only)

Implement `memory scope --check` that warns when a capture would write permissioned data into a public location.

**Tests:** Check `MEMORY_POLICY.md` exists and is valid markdown.

**Dependencies:** Step 1.

**Estimated:** 4 hours.

### Step 6: README Link + Scorecard Entry

**Files:** `README.md`

**Description:** Add a "Memory Lifecycle" section to README linking to `MEMORY_POLICY.md` and documenting the `terraphim-agent memory` command family.

**Tests:** Markdown link check.

**Dependencies:** Step 5.

**Estimated:** 1 hour.

## Rollback Plan

If issues discovered:
1. The `memory` namespace is purely additive -- no existing commands are renamed or removed. Revert by removing the `memory` module and its `pub mod memory;` registration.
2. If rubric judge pipeline integration fails, publish it as a standalone script in `cto-executive-system/automation/` instead.
3. `MEMORY_POLICY.md` can exist independently without any code changes.

## Dependencies

### New Dependencies

| Crate | Version | Justification |
|---|---|---|
| `terraphim_agent_evolution` | from registry (terraphim-agents repo) | Provides `MemoryEvolution`, `LessonsEvolution`, `MemoryItem`, `Lesson` |

### Existing Dependencies Reused

| Crate | How Used |
|---|---|
| `terraphim_sessions` | Provenance subcommand delegates to sessions search |
| `terraphim_hooks` | Apply subcommand delegates to hook diff |
| `terraphim_persistence` | All subcommands read/write via Persistable trait |
| `terraphim_automata` | Retrieve subcommand delegates to search |

## Performance Considerations

| Metric | Target | Measurement |
|---|---|---|
| `memory capture` latency | < 100ms (same as `learn hook`) | Existing benchmark |
| `memory retrieve` latency | < 50ms (same as `search`) | Existing benchmark |
| `memory rubric` runtime | < 10 min per project | Integration test |
| `memory second-run` runtime | < 2s (filesystem read) | Integration test |

## Open Items

| Item | Status | Owner |
|---|---|---|
| Main branch buildability (#3030) | Must be fixed as Step 0 | Agent |
| `terraphim_agent_evolution` registry availability | Confirm crate is published and accessible | Agent |
| Judge pipeline API contract (prompt format, response schema) | Needs confirmation from cto-executive-system | CTO |
| ADF artefact directory path convention | Confirm standard location for telemetry JSON | Agent |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
