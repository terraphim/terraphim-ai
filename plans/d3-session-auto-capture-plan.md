# D3: Session-Based Auto-Capture for Procedures

## Problem

Issue #693 specifies `--steps-from-session <id>` for automatic procedure capture from
AI coding session history. Currently procedures must be built manually via
`learn procedure record` + `add-step`. This plan designs the auto-capture path.

## Prerequisites

- terraphim_sessions crate (exists, feature-gated `repl-sessions`)
- SessionsSub CLI (exists: sources, list, search, stats)
- ProcedureStore (exists, un-gated in Phase B)
- Hook pipeline with success capture (exists, Phase E LearnHookType)

## Design

### Approach 1: Session-to-Procedure Extraction (Recommended)

Extract procedures from completed session history stored in `sessions.json`.

#### New CLI subcommand

```
terraphim-agent learn procedure from-session <session-id> [--title TITLE]
```

Behavior:
1. Load session by ID from `terraphim_sessions` cache
2. Extract all successful Bash commands from the session (exit_code == 0)
3. Filter out trivial commands (cd, ls, cat, echo) via configurable ignore list
4. Group sequential commands into logical steps
5. Create CapturedProcedure with extracted steps
6. Dedup via ProcedureStore::save_with_dedup()
7. Print created procedure ID

#### Implementation files

| File | Change |
|------|--------|
| `crates/terraphim_agent/src/learnings/procedure.rs` | Add `from_session()` function |
| `crates/terraphim_agent/src/main.rs` | Add `FromSession` variant to ProcedureSub |
| `crates/terraphim_sessions/` | Ensure session data includes tool results with exit codes |

#### Session data model

The sessions module stores JSONL with fields like:
```json
{
  "id": "session-uuid",
  "tool_uses": [
    { "tool": "Bash", "command": "cargo build", "exit_code": 0, "timestamp": "..." },
    { "tool": "Bash", "command": "cargo test", "exit_code": 0, "timestamp": "..." }
  ]
}
```

The `from_session()` function:
```rust
pub fn from_session(
    session_id: &str,
    title: Option<String>,
    sessions_path: &Path,
) -> io::Result<CapturedProcedure> {
    // 1. Load session from JSONL
    // 2. Filter to successful Bash commands
    // 3. Skip trivial commands (TRIVIAL_COMMANDS const)
    // 4. Map to ProcedureStep with ordinals
    // 5. Generate title from first+last commands if not provided
    // 6. Return CapturedProcedure
}
```

#### Trivial command filter

```rust
const TRIVIAL_COMMANDS: &[&str] = &[
    "cd ", "ls", "pwd", "echo ", "cat ", "head ", "tail ",
    "wc ", "which ", "type ", "date", "whoami",
];
```

### Approach 2: Real-Time Capture via Hook (Future)

Capture procedures in real-time during a session using the PostToolUse hook.

This requires:
- Session boundary detection (when does a "workflow" start/end?)
- Command grouping heuristic (which commands belong together?)
- User confirmation before saving

This is significantly more complex and deferred to a future phase. The session-to-procedure extraction (Approach 1) provides the same value with less complexity.

## Complexity

**M** (3-5 days)

## Dependencies

- `repl-sessions` feature flag must be enabled
- Session data must include Bash tool results with exit codes
- ProcedureStore must be available (done in Phase B)

## Test Strategy

1. Create a test session JSONL with known successful commands
2. Call `from_session()`, verify procedure has correct steps
3. Verify trivial commands are filtered out
4. Verify title generation when --title not provided
5. CLI integration test: `learn procedure from-session <id>` with test session

## Acceptance Criteria

- [ ] `learn procedure from-session <session-id>` extracts procedure from session
- [ ] Trivial commands (cd, ls, echo) are filtered
- [ ] Title auto-generated from first/last commands if not provided
- [ ] Dedup check via save_with_dedup()
- [ ] Feature-gated behind `repl-sessions`
- [ ] Unit + integration tests pass
- [ ] cargo clippy clean
