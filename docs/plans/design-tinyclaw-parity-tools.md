# Implementation Plan: TinyClaw Hermes Parity Tools — Sandbox, Subagents, Browser

**Status**: Review
**Canonical Path**: `docs/plans/design-tinyclaw-parity-tools.md`
**Change Slug**: `tinyclaw-parity-tools`
**Research**: `docs/plans/research-tinyclaw-parity-tools.md`
**Author**: Kokoro (ADF flow)
**Date**: 2026-08-11
**Estimated Effort**: 1 day

## Overview

### Summary
Add three `Tool`-trait tools to `terraphim_tinyclaw`, wired through `create_default_registry`:
- `SandboxTool` (rlm_code / rlm_bash / rlm_query + session ops) wrapping `terraphim_rlm`
- `SubagentTool` (spawn / status / list / terminate / collect) wrapping `terraphim_spawner`
- `BrowserTool` (navigate / extract / api) native reqwest (agent binary lacks web ops)

### Approach
Crate dependencies (path) for RLM + spawner (both verified standalone-buildable). Permissive KG validation for RLM. Hermetic contract tests per tool (mock executor, trivial agent provider, local axum server).

### Scope
**In Scope:**
- SandboxTool: execute_code, execute_bash, recursive_query, session_create, session_status, session_destroy
- SubagentTool: spawn, status, list, terminate, collect
- BrowserTool: navigate, extract, api (HTTP GET/POST)
- `[sandbox]`, `[subagent]`, `[browser]` config sections
- Contract tests (≥5 per tool)

**Out of Scope:**
- Firecracker/E2B backends in tests (no KVM/keys in CI)
- Browser click/type/screenshot (no engine in deployed agent)
- SSH backend (not in RLM BackendType)

**Avoid At All Cost:**
- CLI bridge for RLM (sessions are in-memory — will fail)
- MCP client just for these tools (no client in tinyclaw)
- Rebuilding the RLM/spawner engines

## Architecture

### Component Diagram
```
ToolRegistry (create_default_registry)
├── SandboxTool ──Arc<TerraphimRlm>──► executor/{Local,Docker} + SessionManager
├── SubagentTool ──Arc<Mutex<AgentPool>>──► AgentHandle registry (HashMap<String, AgentHandle>)
└── BrowserTool ──reqwest::Client──► HTTP (browser-like UA, security context)
```

### Data Flow
```
agent_loop → ToolRegistry.execute(name, args) → tool.execute(args)
  SandboxTool: parse op → rlm.execute_code/bash/query → JSON result string
  SubagentTool: parse op → pool.spawn_with_model / handle.status / pool tracking → JSON
  BrowserTool: parse op → reqwest GET/POST → status + text/JSON
```

### Key Design Decisions
| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| RLM as crate dep | Sessions in-memory; CLI can't hold them | CLI bridge (fails) |
| Permissive + no thesaurus | Validator would reject normal code | Strict (broken UX) |
| `with_executor` mock in tests | Hermetic, no live backend | Live docker (not in CI) |
| Browser native reqwest | Deployed agent has web_operations:false | terraphim_agent dep (no manifest) |
| Path deps to non-member crates | Both build standalone | Adding to workspace (out of scope, touches root Cargo.toml) |

## File Changes

### New Files
| File | Purpose |
|------|---------|
| `src/tools/sandbox.rs` | SandboxTool + RLM wrapper |
| `src/tools/subagent.rs` | SubagentTool + handle registry |
| `src/tools/browser.rs` | BrowserTool |
| `tests/sandbox_contracts.rs` | Hermetic sandbox tests (mock executor) |
| `tests/subagent_contracts.rs` | Hermetic subagent tests (trivial agent provider) |
| `tests/browser_contracts.rs` | Browser tests (local axum server) |

### Modified Files
| File | Changes |
|------|---------|
| `Cargo.toml` | + terraphim_rlm (default-features=false, features llm,kg-validation,docker-backend), + terraphim_spawner (path) |
| `src/tools/mod.rs` | + mod sandbox/subagent/browser; register in create_default_registry |
| `src/config.rs` | + SandboxConfig, SubagentConfig, BrowserConfig + defaults + tests |

## API Design

### SandboxTool
```rust
pub struct SandboxTool { rlm: Arc<TerraphimRlm> }
// ops (args.op): "execute_code" {code}, "execute_bash" {command},
//   "recursive_query" {prompt, session_id?}, "session_create" {},
//   "session_status" {session_id}, "session_destroy" {session_id}
// lazy session: auto-create on first execute_* and reuse
```
`RlmConfig` built from `SandboxConfig`:
- `backend_preference: vec![Local]` (config-overridable `"local"|"docker"`)
- `kg_strictness: Permissive`, no thesaurus
- time budget from `timeout_secs`

### SubagentTool
```rust
pub struct SubagentTool {
    pool: Arc<Mutex<AgentPool>>,
    handles: Arc<Mutex<HashMap<String, AgentHandle>>>, // id -> handle
    provider: Provider,        // from config (agent or llm type)
    ctx: SpawnContext,
}
// ops: "spawn" {task, model?} -> id; "status" {id}; "list" {};
//      "terminate" {id}; "collect" {id} -> captured output
```

### BrowserTool
```rust
pub struct BrowserTool { client: reqwest::Client, security: BrowserSecurity }
// ops: "navigate" {url} -> status+title+first N chars;
//      "extract" {url, selector?} -> text (simple strip);
//      "api" {method, url, headers?, body?} -> status+body
// unsupported ops (click/type/screenshot) -> ToolError::BackendUnavailable
```

### Error Types
Reuse `ToolError`; add `ToolError::BackendUnavailable { tool, message }` for graceful fallback.

## Test Strategy

### Unit Tests (in-module)
| Test | Location | Purpose |
|------|----------|---------|
| sandbox op parsing + session auto-create | sandbox.rs | valid/invalid op args |
| subagent id allocation + registry | subagent.rs | spawn bookkeeping |
| browser op parsing | browser.rs | valid/invalid ops |

### Contract Tests (tests/)
| Test | Location | Purpose |
|------|----------|---------|
| sandbox execute_code via mock executor | sandbox_contracts.rs | hermetic code exec |
| sandbox execute_bash via mock executor | sandbox_contracts.rs | hermetic bash |
| sandbox recursive_query via mock executor | sandbox_contracts.rs | query path |
| sandbox session lifecycle | sandbox_contracts.rs | create/status/destroy |
| sandbox unknown backend → error | sandbox_contracts.rs | graceful |
| subagent spawn trivial agent provider | subagent_contracts.rs | real spawn, harmless cmd |
| subagent status/list/terminate | subagent_contracts.rs | lifecycle |
| subagent collect output | subagent_contracts.rs | output capture |
| browser navigate local server | browser_contracts.rs | GET status+text |
| browser extract | browser_contracts.rs | text extraction |
| browser api POST | browser_contracts.rs | JSON round trip |
| browser click → BackendUnavailable | browser_contracts.rs | graceful |

## Implementation Steps

### Step 1: Cargo deps + config
**Files:** `Cargo.toml`, `src/config.rs`
**Description:** Add path deps; SandboxConfig/SubagentConfig/BrowserConfig with defaults + serde tests.
**Tests:** config parse/round-trip unit tests.

### Step 2: SandboxTool
**Files:** `src/tools/sandbox.rs`
**Description:** RLM wrapper; ops; lazy session; Permissive config; mock-executor support for tests.
**Tests:** unit + contract (mock executor).

### Step 3: SubagentTool
**Files:** `src/tools/subagent.rs`
**Description:** AgentPool wrapper; id registry; spawn/status/list/terminate/collect.
**Tests:** unit + contract (trivial Agent provider, e.g. `sh -c 'echo done'`).

### Step 4: BrowserTool
**Files:** `src/tools/browser.rs`
**Description:** reqwest ops navigate/extract/api; unsupported ops → BackendUnavailable.
**Tests:** unit + contract (axum local server).

### Step 5: Registration + integration
**Files:** `src/tools/mod.rs`
**Description:** register all three in create_default_registry (config-gated: enabled flags).
**Tests:** registry lists new tools; full suite green.

### Step 6: Docs + PRs
**Files:** `.terraphim/flows/tinyclaw-parity-tools.toml` (flow infra), research/design docs
**Description:** commit, push branch `task/3145-3146-3148-parity-tools`, PR #3206, structural review, human merge gate.

## Rollback Plan
1. Remove deps from Cargo.toml + revert tools/mod.rs registration (feature-flag off).
2. Config defaults keep `enabled: false` unless set — no behaviour change on upgrade.

## Dependencies

### New Dependencies
| Crate | Version | Justification |
|-------|---------|---------------|
| `terraphim_rlm` (path) | 1.21.0 | Sandbox execution engine (Local/Docker) |
| `terraphim_spawner` (path) | 1.21.0 | Subagent pool + handles |

### Dependency Updates
None (both new; reqwest already present).

## Performance Considerations
- RLM dep adds ~50s to tinyclaw compile (accepted; default-features=false).
- Sandbox ops bounded by `timeout_secs` (config, default 120s) → RLM time_budget_ms.
- Browser ops bounded by reqwest timeout (default 30s).

## Open Items
| Item | Status | Owner |
|------|--------|-------|
| RLM feature set minimal? | Verify supervision feature needed | Implementer |

## Approval
- [x] Technical review complete (research verified buildability + APIs)
- [x] Test strategy defined
- [ ] Human approval (Alex merge gate)
