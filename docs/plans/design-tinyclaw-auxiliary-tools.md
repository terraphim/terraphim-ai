# Implementation Plan: TinyClaw Hermes Parity — 15 Auxiliary Tools

**Status**: Draft
**Canonical Path**: `docs/plans/design-tinyclaw-auxiliary-tools.md`
**Change Slug**: `tinyclaw-auxiliary-tools`
**Research**: `docs/plans/research-tinyclaw-auxiliary-tools.md`
**Author**: Kokoro (ADF flow)
**Date**: 2026-08-16
**Estimated Effort**: 2–3 days (15 tools across 3 waves)

## Overview

### Summary
Implement 15 missing Hermes capabilities in `terraphim_tinyclaw`, grouped into
three waves by dependency surface. Wave 1 (pure ports) first, then external
integrations, then complex ML.

### Approach
Follow the existing `Tool` trait + `create_default_registry_with_parity`
registration pattern. Pure tools register unconditionally; external tools are
config-gated under `[tools.<name>]`. Memory/KG leverage reuses `run_agent`
from `tools/agent_memory.rs`.

### Scope
**In Scope:**
- Wave 1: todo, interrupt, debug-helpers, fuzzy-match, patch-parser.
- Wave 1.5: clarify, approval, process-registry.
- Wave 2: clipboard, homeassistant, tts, image-generation, vision.
- Wave 3: mixture-of-agents, rl-training.

**Out of Scope:**
- Full difflib-equivalent (Levenshtein ratio approximation accepted).
- rl-training full training loop in v1 (schema + honest stub first).
- homeassistant full entity model (minimal REST state/call_service).

**Avoid At All Cost** (5/25 rule):
- A new persistence subsystem — reuse `terraphim-agent` memory/learn.
- A new embedding service — reuse `terraphim-agent search/suggest`.
- Porting rl-training's 1380 lines verbatim before the ensemble is proven useful.

## Architecture

### Component Diagram
```
ToolRegistry
├── todo (TodoTool ── Arc<TodoStore>)            Wave 1 registered
├── patch_parse (PatchParseTool ── parse_v4a_patch)  Wave 1 registered
├── (edit tool ── fuzzy_find_and_replace)         Wave 1 library
├── interrupt::set_interrupt / is_interrupted      Wave 1 library (global AtomicBool)
├── debug_helpers::DebugSession                     Wave 1 library (env-gated JSON log)
├── clarify (dispatcher → "awaiting user")          Wave 1.5
├── approval (pending queue)                        Wave 1.5
├── process_registry (Arc<Mutex<…>> + memory persist)  Wave 1.5
├── clipboard / homeassistant / tts / image_gen / vision  Wave 2 (config-gated)
└── mixture_of_agents / rl_training                 Wave 3 (proxy/LLM)
```

### Data Flow
`execute(args)` → in-memory store | pure fn | `run_agent` subprocess | reqwest →
JSON string → agent loop.

### Key Design Decisions
| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Wave 1 pure Rust, no new crates | self-contained ports | adding `strsim`/`similar` |
| Levenshtein ratio for fuzzy similarity | std-only, sufficient | difflib-exact port |
| interrupt/debug/fuzzy/patch as libraries, todo/patch_parse as tools | matches Hermes (libs vs registry) | registering everything |
| memory persistence via `learn` not `memory` | `memory` subcommand absent; `learn` present | targeting stale `memory` subcommand |

## File Changes

### New Files (Wave 1)
| File | Purpose |
|------|---------|
| `src/tools/interrupt.rs` | shared `AtomicBool` signal (done) |
| `src/tools/todo.rs` | `TodoStore` + `TodoTool` (done) |
| `src/tools/debug_helpers.rs` | `DebugSession` JSON log |
| `src/tools/fuzzy_match.rs` | 9-strategy `fuzzy_find_and_replace` |
| `src/tools/patch_parser.rs` | V4A `parse_v4a_patch` + `PatchParseTool` |

### Modified Files (Wave 1)
| File | Changes |
|------|---------|
| `src/tools/mod.rs` | add `pub mod` for 5 modules; register `todo` + `patch_parse` |

## API Design

### `interrupt`
```rust
pub fn set_interrupt(active: bool);
pub fn is_interrupted() -> bool;
```

### `fuzzy_match`
```rust
pub fn fuzzy_find_and_replace(content: &str, old_string: &str, new_string: &str,
                              replace_all: bool) -> (String, usize, Option<String>);
fn ratio(a: &str, b: &str) -> f64; // Levenshtein-based similarity
```

### `patch_parser`
```rust
pub enum OperationType { Add, Update, Delete, Move }
pub struct PatchOperation { operation, file_path, new_path: Option<String>, hunks, content }
pub fn parse_v4a_patch(patch: &str) -> (Vec<PatchOperation>, Option<String>);
```

### `todo`
```rust
pub struct TodoStore { items: Mutex<Vec<TodoItem>> }
impl TodoStore { write, read, has_items, format_for_injection }
pub struct TodoTool { store: Arc<TodoStore> }
```

## Test Strategy

### Unit Tests (per module, hermetic)
| Test | Purpose |
|------|---------|
| `interrupt` set/clear roundtrip | flag semantics |
| `todo` write replace/merge, validate defaults, format_for_injection | store logic |
| `fuzzy_match` exact → line-trim → whitespace → indent → escape → boundary → block → context | strategy chain |
| `fuzzy_match` multi-occurrence without replace_all → error | uniqueness guard |
| `patch_parser` add/update/delete/move parse; missing markers | parser |

### Contract Tests
| Test | Purpose |
|------|---------|
| `tests/parity_tools_contracts.rs` | registry registration + schema shape + execute roundtrip |

## Implementation Steps

### Step 1: interrupt + todo (done, pending registration)
**Files:** `interrupt.rs`, `todo.rs`
**Tests:** module `#[cfg(test)]` suites.

### Step 2: debug_helpers
**Files:** `debug_helpers.rs`
**Tests:** env-gated enable/disable, log_call/save roundtrip.

### Step 3: fuzzy_match
**Files:** `fuzzy_match.rs`
**Tests:** 9-strategy chain + uniqueness + no-match error.

### Step 4: patch_parser
**Files:** `patch_parser.rs`
**Tests:** parse add/update/delete/move; malformed input.

### Step 5: register + wire
**Files:** `tools/mod.rs`
**Tests:** `tests/parity_tools_contracts.rs` (todo + patch_parse registered).

### Step 6: build + clippy + fmt + full suite

## Rollback Plan
- Each tool is an isolated module; revert a module + its `pub mod`/registration
  lines independently. No shared state between tools except `interrupt` (global).

## Dependencies

### New Dependencies
| Crate | Version | Justification |
|-------|---------|---------------|
| none | — | Wave 1 uses std + existing serde/tokio/regex |

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify `memory` subcommand (resolved: absent → use `learn`/`sessions`) | Resolved | Kokoro |
| clarify/approval interaction callback design | Pending (Wave 1.5) | Kokoro |
| Wave 2/3 detailed design | Pending | Kokoro |

## Approval

- [x] Research complete
- [x] Wave 1 design complete
- [ ] Human approval received
