You are executing the **disciplined-design** skill (Phase 2 of disciplined development) for Gitea issue #3144 in terraphim-ai.

## Task

Produce a concrete implementation design for wiring terraphim-agent memory/learning into TinyClaw. Read `.docs/adf/3144/research.md` first (written by the research phase). Do NOT write production code — produce a design document with file-level change plans.

## Design requirements (from #3144)

1. New file `crates/terraphim_tinyclaw/src/tools/agent_memory.rs` exposing 4 tools implementing the existing `Tool` trait:
   - `memory_capture` — `terraphim-agent memory capture --format json --robot <text>`
   - `memory_retrieve` — `terraphim-agent memory retrieve --format json --robot <query>` (returns empty gracefully on no match)
   - `memory_apply` — retrieve + apply items (returns items for system-prompt injection)
   - `learn_capture` — `terraphim-agent learn capture --format json --robot <failed command + correction>`
2. Register the tools in `ToolRegistry` (in `src/tools/mod.rs`), feature-safe: if `terraphim-agent` binary is missing at call time, return a structured error and skip for the session.
3. `src/agent/agent_loop.rs`: before the LLM call, if memory is enabled and `memory_apply` returns items, prepend them to the system prompt.
4. `src/config.rs`: add `[memory]` section with `enabled: bool` (default false) and `role: Option<String>` override.
5. Tests: unit tests for JSON parsing of the robot-format output; integration test for capture→retrieve round-trip (can use a fake/scripted `terraphim-agent` shim on PATH in the test to stay hermetic — DO NOT require the real binary or a live memory store).

## Design output

Write `.docs/adf/3144/design.md`:

```markdown
# Design: #3144 TinyClaw memory/learning bridge

## Overview
<one paragraph>

## Files to change
| File | Change |
|------|--------|
| ... | ... |

## Tool trait implementation
<struct definitions, name(), description(), schema(), execute() signatures — match the trait exactly>

## JSON parsing strategy
<robot-format shapes; serde structs; error handling>

## agent_loop integration
<exact insertion point; token-budget guard for large dumps>

## Config
<[memory] fields; defaults; wire into Config struct>

## Test plan
<unit + hermetic integration tests; shim approach>

## Edge cases
<missing binary, empty store, huge dump, JSON failure>

## Risks
<...>
```

Cite file:line anchors from the research report. Output ONLY the design.
