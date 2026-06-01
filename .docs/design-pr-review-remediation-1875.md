# Implementation Plan: PR Review Remediation for #1875

**Status**: Draft
**Research Doc**: `.docs/research-pr-review-remediation-1875.md`
**Author**: Terraphim AI
**Date**: 2026-05-27
**Estimated Effort**: 1-2 hours

## Overview

### Summary

Four changes to address the structural PR review findings on branch `task/1875-adf-ctl-local-direct-dispatch`, raising the confidence score from 3/5 to 4/5.

### Approach

Minimal, targeted edits. Each finding is addressed in its own implementation step with its own verification.

### Scope

**In Scope:**
- `#[cfg(unix)]` gating on direct_dispatch module and call sites
- Bounded read on UDS socket
- `.gitignore` for learning artefacts
- PR metadata update

**Out of Scope:**
- Windows named-pipe implementation
- Authentication on UDS beyond file permissions
- Branch splitting (commits are interleaved; not feasible)
- Refactoring the `LoopEvent` enum or `handle_direct_dispatch` (these compile fine cross-platform)

**Avoid At All Cost:**
- Adding a feature flag for direct dispatch (unnecessary complexity -- cfg(unix) is sufficient)
- Restructuring the event loop to remove the DirectDispatch variant on non-Unix (dead code on Windows is harmless and much simpler than conditional enum variants)

### Simplicity Check

The simplest design: add `#[cfg(unix)]` to the module declaration and the two call sites that reference `direct_dispatch::start_direct_dispatch_listener`. Everything else (config struct, LoopEvent variant, handler method) uses no Unix-specific types and compiles everywhere. This avoids conditional compilation creep.

For the bounded read: replace `BufReader<UnixStream>` with `BufReader<Take<OwnedReadHalf>>` and pass `OwnedWriteHalf` separately to the response writer. Three function signatures change; logic stays identical.

## Architecture

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Gate only the module + listener call sites | Minimises `#[cfg]` spread; config/handler/enum compile on all platforms | Gating entire sections of lib.rs event loop (too much cfg noise) |
| Use `into_split()` + `take()` | Cleanly separates read-limited and write concerns | `BufReader::with_capacity` (doesn't actually bound), manual byte loop (more code) |
| 8192-byte read limit | 40x larger than typical command; generous for any reasonable JSON | 1024 (too tight if context is long), unlimited (current problem) |
| `.terraphim/learnings/` in gitignore | Consistent with `.beads/` pattern; machine-local artefacts | Tracking them (pollutes repo with 265+ auto-generated files) |

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Feature flag `direct-dispatch` | cfg(unix) is cleaner for platform code | Feature flag implies optionality within Unix too |
| Conditional LoopEvent enum | Adds `#[cfg(unix)]` to every match arm | Massive code churn for dead-variant elimination |
| PR branch splitting | Commits interleaved with merge commits | Would require rebase surgery on 47 commits |

## File Changes

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/lib.rs` | Add `#[cfg(unix)]` to `pub mod direct_dispatch;` (line 41), gate channel init block (lines 1264-1269), gate listener startup block (lines 1316-1336), gate bridge task block (lines 1413-1428) |
| `crates/terraphim_orchestrator/src/direct_dispatch.rs` | Refactor `handle_connection` to use `into_split()` + `take(MAX_COMMAND_SIZE)`, change `write_response` signature to accept `OwnedWriteHalf` |
| `.gitignore` | Add `.terraphim/learnings/` entry |

### No New or Deleted Files

## API Design

### Changed Function Signatures in `direct_dispatch.rs`

```rust
const MAX_COMMAND_SIZE: u64 = 8192;

async fn handle_connection(
    stream: tokio::net::UnixStream,
    dispatch_tx: &tokio::sync::mpsc::Sender<WebhookDispatch>,
    agent_names: &HashSet<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::{AsyncBufReadExt, AsyncReadExt};

    let (reader, writer) = stream.into_split();
    let limited_reader = reader.take(MAX_COMMAND_SIZE);
    let mut buf_reader = tokio::io::BufReader::new(limited_reader);
    let mut line = String::new();

    let bytes_read = buf_reader.read_line(&mut line).await?;
    if bytes_read == 0 {
        return Ok(());
    }

    // ... parsing and dispatch logic unchanged ...

    let response = DispatchResponse::ok();
    write_response(writer, response).await?;
    Ok(())
}

async fn write_response(
    mut writer: tokio::net::unix::OwnedWriteHalf,
    response: DispatchResponse,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::AsyncWriteExt;
    let json = serde_json::to_string(&response)?;
    writer.write_all(json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    Ok(())
}
```

### `#[cfg(unix)]` Gating in `lib.rs`

Only three sites need gating -- the module declaration and the two blocks that call into it:

```rust
// Line 41: module declaration
#[cfg(unix)]
pub mod direct_dispatch;

// Lines 1264-1269: channel creation (inside run())
#[cfg(unix)]
let direct_dispatch_rx = if self.config.direct_dispatch.is_some() {
    let (tx, rx) = tokio::sync::mpsc::channel(64);
    Some((tx, rx))
} else {
    None
};
#[cfg(not(unix))]
let direct_dispatch_rx: Option<(
    tokio::sync::mpsc::Sender<webhook::WebhookDispatch>,
    tokio::sync::mpsc::Receiver<webhook::WebhookDispatch>,
)> = None;

// Lines 1316-1336: listener startup
#[cfg(unix)]
let direct_dispatch_rx = if let Some(ref direct_cfg) = self.config.direct_dispatch {
    // ... existing code unchanged ...
} else {
    None
};
#[cfg(not(unix))]
let direct_dispatch_rx: Option<tokio::sync::mpsc::Receiver<webhook::WebhookDispatch>> = None;

// Lines 1413-1428: bridge task
// This block already has `if let Some(direct_rx) = direct_dispatch_rx`
// which evaluates to None on non-unix, so no cfg needed -- the compiler
// will see direct_dispatch_rx is always None and dead-code-eliminate the block.
```

**Critical insight**: The bridge task (lines 1413-1428), LoopEvent::DirectDispatch variant (line 1343), match arms (lines 1445, 1462), and `handle_direct_dispatch` method (lines 3904-3938) do NOT need `#[cfg(unix)]` because they contain no Unix-specific types. The `direct_dispatch_rx` variable is typed as `Option<Receiver<WebhookDispatch>>` which compiles on all platforms. On non-Unix it's always `None`, so the bridge task never spawns and the match arms are dead code -- Rust compiles them fine.

## Test Strategy

### Verification Tests

| Test | Method | Purpose |
|------|--------|---------|
| Cross-compile check | `cargo check -p terraphim_orchestrator --target x86_64-pc-windows-gnu` | Confirms P1 fix -- module compiles on Windows |
| Existing UDS tests | `cargo test -p terraphim_orchestrator --lib direct_dispatch` | Confirms P2 fix doesn't break existing round-trip tests |
| Existing orchestrator tests | `cargo test -p terraphim_orchestrator --lib test_direct_dispatch` | Confirms lib.rs integration tests still pass |
| adf-ctl tests | `cargo test -p terraphim_orchestrator --bin adf-ctl` | Confirms binary tests still pass |
| Clippy | `cargo clippy -p terraphim_orchestrator` | Zero warnings |
| Gitignore | `git status -- .terraphim/learnings/` | Shows no tracked files |

### New Test

Add one unit test in `direct_dispatch.rs` to verify the read limit:

```rust
#[cfg(unix)]
#[tokio::test]
async fn test_direct_dispatch_rejects_oversized_command() {
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("adf.sock");
    let (tx, _rx) = mpsc::channel::<WebhookDispatch>(1);
    let agent_names = ["meta-learning".to_string()].into_iter().collect();

    let handle = start_direct_dispatch_listener(socket_path.clone(), tx, agent_names);
    wait_for_socket(&socket_path).await;

    // Send a command larger than MAX_COMMAND_SIZE without a newline
    let oversized = "x".repeat(16384);
    let stream = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        tokio::net::UnixStream::connect(&socket_path),
    )
    .await
    .expect("connect timed out")
    .expect("connect failed");

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (_, mut write_half) = stream.into_split();
    // Write oversized payload without newline -- should be truncated by take()
    let _ = write_half.write_all(oversized.as_bytes()).await;
    drop(write_half);

    // The listener should not crash -- it will read up to MAX_COMMAND_SIZE
    // and then fail to parse the truncated JSON, returning an error response
    // or closing the connection. Either way, the listener keeps running.
    tokio::task::yield_now().await;

    // Verify listener is still alive by sending a valid command
    let response = send_command(
        &socket_path,
        r#"{"agent":"meta-learning","context":"after-oversize"}"#,
    )
    .await;
    assert_eq!(response["status"], "ok", "listener must survive oversized input");

    handle.abort();
}
```

## Implementation Steps

### Step 1: Gate `direct_dispatch` module with `#[cfg(unix)]`

**Files**: `crates/terraphim_orchestrator/src/lib.rs`
**Description**: Add `#[cfg(unix)]` to the module declaration at line 41. Add `#[cfg(unix)]` and `#[cfg(not(unix))]` stubs to the two channel/listener blocks inside `run()`.
**Test**: `cargo check -p terraphim_orchestrator --target x86_64-pc-windows-gnu`
**Estimated**: 15 minutes

**Exact changes**:

1. Line 41: `pub mod direct_dispatch;` becomes `#[cfg(unix)] pub mod direct_dispatch;`

2. Lines 1264-1269 (channel init): Wrap in `#[cfg(unix)]` and add `#[cfg(not(unix))]` type-annotated `None`:
   ```rust
   #[cfg(unix)]
   let direct_dispatch_rx = if self.config.direct_dispatch.is_some() {
       let (tx, rx) = tokio::sync::mpsc::channel(64);
       Some((tx, rx))
   } else {
       None
   };
   #[cfg(not(unix))]
   let direct_dispatch_rx: Option<(
       tokio::sync::mpsc::Sender<webhook::WebhookDispatch>,
       tokio::sync::mpsc::Receiver<webhook::WebhookDispatch>,
   )> = None;
   ```

3. Lines 1316-1336 (listener startup): Wrap in `#[cfg(unix)]` and add `#[cfg(not(unix))]` typed `None`:
   ```rust
   #[cfg(unix)]
   let direct_dispatch_rx = if let Some(ref direct_cfg) = self.config.direct_dispatch {
       // ... existing body unchanged ...
       Some(direct_rx)
   } else {
       None
   };
   #[cfg(not(unix))]
   let direct_dispatch_rx: Option<tokio::sync::mpsc::Receiver<webhook::WebhookDispatch>> = None;
   ```

4. Lines 1413-1428 (bridge task), 1343 (LoopEvent variant), 1445/1462 (match arms), 3904-3938 (handler): **No changes needed**. These contain no Unix-specific types. On non-Unix, `direct_dispatch_rx` is `None` so the bridge task is never spawned and the match arms are dead code (compiles fine).

### Step 2: Bound the `read_line` with `take()`

**Files**: `crates/terraphim_orchestrator/src/direct_dispatch.rs`
**Description**: Refactor `handle_connection` to split the stream, limit reads to 8192 bytes, and pass write half separately to `write_response`.
**Test**: `cargo test -p terraphim_orchestrator --lib direct_dispatch` (all 12 existing tests must pass)
**Dependencies**: None (independent of Step 1)
**Estimated**: 20 minutes

**Exact changes**:

1. Add constant at module level (after the imports, before `DispatchCommand`):
   ```rust
   const MAX_COMMAND_SIZE: u64 = 8192;
   ```

2. Replace `handle_connection` body:
   ```rust
   async fn handle_connection(
       stream: tokio::net::UnixStream,
       dispatch_tx: &tokio::sync::mpsc::Sender<WebhookDispatch>,
       agent_names: &HashSet<String>,
   ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
       use tokio::io::{AsyncBufReadExt, AsyncReadExt};

       let (read_half, write_half) = stream.into_split();
       let mut reader = tokio::io::BufReader::new(read_half.take(MAX_COMMAND_SIZE));
       let mut line = String::new();

       let bytes_read = reader.read_line(&mut line).await?;
       if bytes_read == 0 {
           return Ok(());
       }

       let cmd: DispatchCommand = match serde_json::from_str(line.trim()) {
           Ok(cmd) => cmd,
           Err(e) => {
               let response = DispatchResponse::error(&format!("invalid JSON: {}", e));
               write_response(write_half, response).await?;
               return Ok(());
           }
       };

       if !agent_names.contains(&cmd.agent) {
           let response = DispatchResponse::error(&format!("unknown agent: {}", cmd.agent));
           write_response(write_half, response).await?;
           return Ok(());
       }

       let dispatch = WebhookDispatch::SpawnAgent {
           agent_name: cmd.agent,
           detected_project: None,
           issue_number: 0,
           comment_id: 0,
           context: cmd.context.unwrap_or_default(),
       };

       if dispatch_tx.send(dispatch).await.is_err() {
           let response = DispatchResponse::error("orchestrator channel closed");
           write_response(write_half, response).await?;
           return Ok(());
       }

       let response = DispatchResponse::ok();
       write_response(write_half, response).await?;
       Ok(())
   }
   ```

3. Replace `write_response`:
   ```rust
   async fn write_response(
       mut writer: tokio::net::unix::OwnedWriteHalf,
       response: DispatchResponse,
   ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
       use tokio::io::AsyncWriteExt;
       let json = serde_json::to_string(&response)?;
       writer.write_all(json.as_bytes()).await?;
       writer.write_all(b"\n").await?;
       Ok(())
   }
   ```

4. Add new test `test_direct_dispatch_rejects_oversized_command` (as specified in Test Strategy above).

### Step 3: Gitignore learning artefacts

**Files**: `.gitignore`
**Description**: Add `.terraphim/learnings/` to `.gitignore` and remove the 31 tracked files from the index.
**Test**: `git status -- .terraphim/learnings/` shows no tracked files
**Dependencies**: None
**Estimated**: 5 minutes

**Exact changes**:

1. Add to `.gitignore` after the `.beads/` block (line 48):
   ```
   # Learning capture artefacts (auto-generated, machine-local)
   .terraphim/learnings/
   ```

2. Remove tracked files from index:
   ```bash
   git rm --cached .terraphim/learnings/*.md
   ```

### Step 4: Update PR metadata

**Description**: Update the PR title and description to reflect the multi-feature scope.
**Dependencies**: Steps 1-3 committed
**Estimated**: 5 minutes

**New PR title**: `feat: adf-ctl direct dispatch (#1875), FffIndexer migration (#1873), local .terraphim config (#1862)`

**New PR body** (structured summary):

```markdown
## Summary

Multi-feature branch consolidating three related improvements:

### 1. adf-ctl direct dispatch (#1875)
- Unix domain socket listener for low-latency local agent dispatch
- `adf-ctl --local trigger --direct` bypasses HTTP webhook + HMAC
- Socket permissions 0600, bounded reads (8 KiB), agent name validation
- Separate `LoopEvent::DirectDispatch` variant (no mention config required)

### 2. FffIndexer migration (#1873)
- Replaces `RipgrepIndexer` with pure-Rust `fff-search` middleware
- KG scorer helper for TerraphimGraph relevance function
- 722-line integration test suite

### 3. Local .terraphim config (#1862)
- `ProjectConfig::load_from_dir()` for `.terraphim/` directory scanning
- Role file discovery (`role-*.json`), thesaurus/KG path helpers
- MCP server and terraphim-agent integration

### Housekeeping
- Cargo.toml metadata (description, readme, homepage) for ~20 crates
- `#[cfg(unix)]` gating on direct_dispatch module for cross-platform compilation

## Test plan
- [ ] `cargo test -p terraphim_orchestrator` -- all tests pass
- [ ] `cargo check -p terraphim_orchestrator --target x86_64-pc-windows-gnu` -- cross-compile check
- [ ] `cargo clippy -p terraphim_orchestrator` -- zero warnings
- [ ] `cargo test -p terraphim_middleware --test fff_indexer` -- FffIndexer tests pass
- [ ] `cargo test -p terraphim_config` -- project config tests pass
```

## Rollback Plan

Each step is independently revertable via `git revert`. No migrations, no data changes, no external system dependencies.

## Dependencies

### No New Dependencies

All changes use existing crate APIs:
- `tokio::net::UnixStream::into_split()` (already available via `tokio = { features = ["full"] }`)
- `tokio::io::AsyncReadExt::take()` (same)
- `tokio::net::unix::OwnedWriteHalf` (same)

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Confirm learning files should NOT be versioned | Assumed yes per research | alex (approve/reject) |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Human approval received
