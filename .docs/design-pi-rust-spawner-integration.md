# Design Document: pi-rust Spawner Integration

**Status**: Draft
**Author**: opencode (GLM-5.1)
**Date**: 2026-05-24
**Research Doc**: `.docs/research-pi-rust-spawner-integration.md`

## Overview

Add pi-rust as a fully recognised CLI tool across the spawner and orchestrator, enabling the KG router's existing pi-rust routes to produce correct non-interactive spawns with telemetry parsing.

## Scope

4 files changed, ~150 lines added (code + tests). Zero breaking changes to existing CLI tools.

## Step Sequence

### Step 1: Extend spawner `config.rs` — pi-rust arg inference

**File**: `crates/terraphim_spawner/src/config.rs`

#### 1a. `infer_args()` — add `"pi-rust" | "pi"` arm

```rust
"pi-rust" | "pi" => vec![
    "-p".to_string(),
    "--mode".to_string(),
    "json".to_string(),
],
```

Rationale: `-p` enables non-interactive (print) mode. `--mode json` produces structured JSON output that the telemetry parser can consume (confirmed via live test: pi-rust emits `turn_end` and `agent_end` events with full usage data).

Location: after the `opencode` arm (line 119-123), before the `bash | sh` arm (line 127).

#### 1b. `model_args()` — add `"pi-rust" | "pi"` arm

```rust
"pi-rust" | "pi" => vec![
    "--model".to_string(),
    model.to_string(),
],
```

Rationale: pi-rust accepts `--model <model_id>` with canonical model names (e.g. `glm-5.1`, `kimi-for-coding/k2p6`). No prefix composition needed — pi-rust handles provider selection via `--provider` separately.

Location: after the `opencode` arm (line 158), before the catch-all (line 159).

Note: The orchestrator currently does NOT pass `--provider` through the spawner's `model_args()`. The `--provider` flag is set separately by the orchestrator when it constructs the spawn command (see Step 3 below).

#### 1c. `infer_supports_stdin()` — no change needed

pi-rust supports stdin (confirmed: it reads task from stdin when no positional args are given). The current default (`true` for unknown tools) is correct. Since pi-rust is now a known tool, we must explicitly return `true`:

```rust
fn infer_supports_stdin(cli_command: &str) -> bool {
    !matches!(Self::cli_name(cli_command), "opencode")
}
```

This already returns `true` for pi-rust (it's not "opencode"), so no change is needed.

#### 1d. `infer_api_keys()` — no change needed

pi-rust manages its own per-provider auth (env vars like `ZAI_API_KEY`, `KIMI_API_KEY`, etc.), similar to opencode. Returns empty vec — correct by default.

### Step 2: Add pi-rust telemetry parser

**File**: `crates/terraphim_orchestrator/src/control_plane/output_parser.rs`

#### 2a. Add `parse_pi_rust_line()` function

Parses pi-rust `--mode json` output events. The key event for telemetry is `turn_end`, which contains:

```json
{
  "type": "turn_end",
  "sessionId": "...",
  "turnIndex": 0,
  "message": {
    "role": "assistant",
    "usage": {
      "input": 6938,
      "output": 4,
      "cacheRead": 64,
      "cacheWrite": 0,
      "totalTokens": 6942,
      "cost": { "input": 0.0, "output": 0.0, "cacheRead": 0.0, "cacheWrite": 0.0, "total": 0.0 }
    },
    "stopReason": "stop"
  },
  "latencyBreakdown": {
    "totalMs": 3905,
    "dominantComponent": "provider_streaming"
  }
}
```

Function signature:

```rust
pub fn parse_pi_rust_line(
    line: &str,
    session_id: &str,
    model: &str,
) -> ParsedOutput
```

Implementation plan:
1. Trim line, return `Ignored` if empty
2. Parse as JSON; return `Unparseable` if invalid
3. Match on `type` field:
   - `"turn_end"` → extract tokens from `message.usage`, cost from `message.usage.cost.total`, latency from `latencyBreakdown.totalMs`, success from `message.stopReason == "stop"`
   - `"agent_start"`, `"agent_end"`, `"message_start"`, `"message_end"`, `"message_update"`, `"turn_start"`, `"session"` → `Ignored`
   - Unknown → `Ignored`

Token mapping (pi-rust usage → TokenBreakdown):
| pi-rust field | TokenBreakdown field |
|---------------|---------------------|
| `totalTokens` | `total` |
| `input` | `input` |
| `output` | `output` |
| (not present) | `reasoning` = 0 |
| `cacheRead` | `cache_read` |
| `cacheWrite` | `cache_write` |

#### 2b. Add unit tests for pi-rust parser

- `test_parse_pi_rust_turn_end`: happy path with full usage
- `test_parse_pi_rust_ignored_events`: session, agent_start, message_update
- `test_parse_pi_rust_unparseable`: non-JSON line
- `test_parse_pi_rust_non_stop_reason`: stopReason != "stop" → success=false

### Step 3: Update orchestrator `lib.rs` — 3 hardcode sites

**File**: `crates/terraphim_orchestrator/src/lib.rs`

#### 3a. `supports_model_flag` (line 1937)

Current:
```rust
let supports_model_flag = matches!(cli_name, "claude" | "claude-code" | "opencode");
```

Change to:
```rust
let supports_model_flag = matches!(cli_name, "claude" | "claude-code" | "opencode" | "pi-rust" | "pi");
```

**Important nuance**: `cli_name` at line 1933 is derived from `def.cli_tool` (the original TOML agent definition), NOT from the KG-overridden CLI. This is correct: we want the model routing to apply when the agent definition says it supports model flags, regardless of what the KG router picks. When the KG router overrides the CLI to pi-rust, the `effective_cli` at line 2108 takes over for the actual spawn, and the spawner's own `infer_args` / `model_args` handle pi-rust correctly.

However, if `def.cli_tool` is `opencode` and the KG router overrides to pi-rust, the model routing block WILL execute (because cli_name="opencode" matches `supports_model_flag`). The routed model then gets passed through `effective_cli` to the spawner, where `model_args("pi-rust", model)` handles it correctly. This is the intended behaviour.

#### 3b. Model composition (line 2093)

Current:
```rust
let model = if cli_name == "opencode" {
    // compose provider/model for opencode
}
```

pi-rust does NOT need `provider/model` composition — it uses `--model` with the canonical model ID and `--provider` separately. Since `cli_name` comes from `def.cli_tool` (not the KG override), this block only triggers when the agent definition uses opencode. When KG overrides to pi-rust, this block is skipped (because cli_name is still "opencode", but the model string is already correct from routing — it either has a `/` prefix from KG routing or is passed through unchanged).

**No change needed here.** The `effective_cli` at line 2108 ensures the spawner gets pi-rust, and the spawner's `model_args("pi-rust", model)` handles the model flag correctly regardless of composition.

#### 3c. Telemetry parsing (line 7671-7677)

Current:
```rust
let parsed = match cli_tool {
    "opencode" => {
        control_plane::output_parser::parse_opencode_line(line, session_id, model, None)
    }
    "claude" => control_plane::output_parser::parse_claude_line(line, session_id, model),
    _ => control_plane::output_parser::ParsedOutput::Ignored,
};
```

Change to:
```rust
let parsed = match cli_tool {
    "opencode" => {
        control_plane::output_parser::parse_opencode_line(line, session_id, model, None)
    }
    "claude" => control_plane::output_parser::parse_claude_line(line, session_id, model),
    "pi-rust" | "pi" => {
        control_plane::output_parser::parse_pi_rust_line(line, session_id, model)
    }
    _ => control_plane::output_parser::ParsedOutput::Ignored,
};
```

Note: `cli_tool` here is the effective CLI from the spawn (the actual binary that was run), so it correctly matches "pi-rust" when the KG router overrides the CLI.

### Step 4: Provider flag wiring

The orchestrator currently does not pass `--provider` as a separate flag through the spawner. For pi-rust, the provider is needed because pi-rust uses it to select the correct API endpoint and auth credentials.

**Approach**: The KG router's routing decision already includes the provider in the `candidate.model` field (e.g., `"zai-coding-plan/glm-5.1"`). The spawner's `model_args("pi-rust", "zai-coding-plan/glm-5.1")` will pass `--model zai-coding-plan/glm-5.1`, but pi-rust expects `--provider zai-coding-plan --model glm-5.1` separately.

**Solution**: Add provider extraction to the spawner's `model_args()` for pi-rust:

```rust
"pi-rust" | "pi" => {
    let mut args = Vec::new();
    if let Some((provider, model_id)) = model.split_once('/') {
        args.push("--provider".to_string());
        args.push(provider.to_string());
        args.push("--model".to_string());
        args.push(model_id.to_string());
    } else {
        args.push("--model".to_string());
        args.push(model.to_string());
    }
    args
}
```

This handles both:
- `"zai-coding-plan/glm-5.1"` → `--provider zai-coding-plan --model glm-5.1`
- `"glm-5.1"` (no slash) → `--model glm-5.1`

## Test Strategy

### Unit Tests (spawner config.rs)

| Test | Validates |
|------|-----------|
| `test_infer_args_pi_rust` | `infer_args("pi-rust")` returns `["-p", "--mode", "json"]` |
| `test_infer_args_pi_rust_full_path` | Full path extraction works |
| `test_infer_args_pi` | `infer_args("pi")` alias works |
| `test_model_args_pi_rust_composed` | `"zai-coding-plan/glm-5.1"` → `["--provider", "zai-coding-plan", "--model", "glm-5.1"]` |
| `test_model_args_pi_rust_bare` | `"glm-5.1"` → `["--model", "glm-5.1"]` |
| `test_infer_supports_stdin_pi_rust` | Returns `true` |
| `test_infer_api_keys_pi_rust` | Returns empty vec |

### Unit Tests (output_parser.rs)

| Test | Validates |
|------|-----------|
| `test_parse_pi_rust_turn_end` | Full `turn_end` event → CompletionEvent with correct tokens/cost/latency |
| `test_parse_pi_rust_ignored_events` | `session`, `agent_start`, `message_update` → Ignored |
| `test_parse_pi_rust_unparseable` | Non-JSON → Unparseable |
| `test_parse_pi_rust_non_stop_reason` | `stopReason != "stop"` → success=false |
| `test_parse_pi_rust_full_output` | Multi-line stdout with session+turn_end → 1 CompletionEvent |

### Integration Verification

```bash
cargo test -p terraphim_spawner -- --test-threads=1
cargo test -p terraphim_orchestrator -- output_parser --test-threads=1
cargo build --workspace
cargo clippy --workspace
```

## Step Execution Order

1. `config.rs`: Add pi-rust arms to `infer_args()`, `model_args()` (Steps 1a, 1b)
2. `output_parser.rs`: Add `parse_pi_rust_line()` + tests (Step 2)
3. `lib.rs`: Update `supports_model_flag` match and `parse_stdout_for_telemetry` match (Steps 3a, 3c)
4. `config.rs`: Add pi-rust unit tests (Step 4 of test strategy)
5. Build + test + clippy (Step 5)

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| pi-rust JSON format changes | Parser is tolerant: missing fields default to 0; unknown event types are Ignored |
| Provider/model split breaks for models without `/` | `split_once('/')` returns None, falls back to bare `--model` flag |
| `supports_model_flag` at L1937 uses `def.cli_tool` not KG override | This is correct: the model routing should apply based on the agent definition, and the spawner handles the KG-overridden CLI separately |
| `cli_tool` in telemetry might be full path `/home/alex/.local/bin/pi-rust` | The `cli_tool` passed to `parse_stdout_for_telemetry` comes from the spawn record which uses the Provider's `cli_command` (full path). Need to extract basename in the match. |

Wait — let me verify how `cli_tool` is passed to `parse_stdout_for_telemetry`. Let me check the call site.

The call site at line 6445 passes `&cli_tool` which comes from the AgentRunRecord. Let me verify what value is stored there.

Actually, looking at the spawn code in lib.rs, the `cli_tool` in telemetry comes from the effective_cli path stored in the agent run record. This is the full path (e.g., `/home/alex/.local/bin/pi-rust`). The match in `parse_stdout_for_telemetry` uses exact string match, so `"pi-rust"` would NOT match `/home/alex/.local/bin/pi-rust`.

**Fix**: Extract basename before matching, consistent with how `cli_name()` works in config.rs:

```rust
let cli_basename = std::path::Path::new(cli_tool)
    .file_name()
    .and_then(|n| n.to_str())
    .unwrap_or(cli_tool);

let parsed = match cli_basename {
    "opencode" => { ... }
    "claude" | "claude-code" => { ... }
    "pi-rust" | "pi" => { ... }
    _ => ParsedOutput::Ignored,
};
```

**But wait**: Let me check the existing code. The opencode and claude matches currently work, which means either: (a) `cli_tool` is already the basename, or (b) opencode/claude paths never have directories. Looking at the agent definitions, `cli_tool = "opencode"` (bare name, resolved via PATH) and `cli_tool = "claude"` (bare name). But when the KG router overrides to a full path like `/home/alex/.local/bin/pi-rust`, the `cli_tool` stored in the run record would be the full path.

Let me verify by checking how `cli_tool` is stored in the AgentRunRecord and passed to `parse_stdout_for_telemetry`.

The `parse_stdout_for_telemetry` at line 6445 receives `&run.cli_tool`. The `run` comes from `active_runs` which stores the `AgentRunRecord`. The `cli_tool` field in `AgentRunRecord` is set during spawn — it's the effective CLI path.

Since the existing code matches `"opencode"` and `"claude"` as bare names, and those work, it means `cli_tool` in the run record stores the bare name (resolved via PATH). But when KG overrides to a full path, it would store the full path.

**Safest approach**: Always extract basename before matching. This is a minor refactor that makes the telemetry parser path-agnostic:

```rust
fn parse_stdout_for_telemetry(
    cli_tool: &str,
    line: &str,
    session_id: &str,
    model: &str,
) -> Option<control_plane::telemetry::CompletionEvent> {
    let cli_basename = std::path::Path::new(cli_tool)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(cli_tool);
    let parsed = match cli_basename {
        "opencode" => { ... }
        "claude" | "claude-code" => { ... }
        "pi-rust" | "pi" => { ... }
        _ => ParsedOutput::Ignored,
    };
    ...
}
```

This is a safe, backwards-compatible change — bare names like "opencode" pass through `Path::file_name()` unchanged.

## File Change Summary

| File | Change | Lines Added (est.) |
|------|--------|-------------------|
| `crates/terraphim_spawner/src/config.rs` | pi-rust arms in `infer_args()`, `model_args()` + 7 unit tests | ~60 |
| `crates/terraphim_orchestrator/src/control_plane/output_parser.rs` | `parse_pi_rust_line()` + 5 unit tests | ~100 |
| `crates/terraphim_orchestrator/src/lib.rs` | `supports_model_flag` match (L1937), basename extraction + pi-rust arm in `parse_stdout_for_telemetry` (L7665-7682) | ~10 |

**Total**: ~170 lines, 3 files.

## Approval Required

This design modifies:
1. Spawner config (additive, no existing behaviour change)
2. Orchestrator lib.rs (extends 2 match arms, adds basename extraction)
3. Output parser (new function + tests)

All changes are additive. No existing CLI tool behaviour is affected.
