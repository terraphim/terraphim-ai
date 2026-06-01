# Research Document: PR Review Remediation for #1875 (adf-ctl direct dispatch)

**Status**: Draft
**Author**: Terraphim AI
**Date**: 2026-05-27
**Reviewers**: alex

## Executive Summary

The structural PR review of branch `task/1875-adf-ctl-local-direct-dispatch` identified four findings: one P1 (unconditional `#[cfg(unix)]` gating on `direct_dispatch` module) and three P2s (unbounded `read_line`, committed learning artefacts, bundled PR scope). This research maps the exact code paths affected, evaluates remediation options, and determines feasibility of each fix.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Fixes block merge of a clean feature |
| Leverages strengths? | Yes | Rust platform gating is our core competency |
| Meets real need? | Yes | Cross-platform compilation, defence-in-depth, repo hygiene |

**Proceed**: Yes (3/3 YES)

## Problem Statement

### Description

Four findings from the structural PR review need resolution before the PR can merge at confidence 4/5 or better.

### Success Criteria

1. `cargo check -p terraphim_orchestrator --target x86_64-pc-windows-gnu` compiles (or gracefully stubs the module)
2. `read_line` has a bounded read limit
3. `.terraphim/learnings/` is gitignored and untracked files removed from staging
4. PR title/description reflects actual scope, or features are split

## Current State Analysis

### Finding 1: `pub mod direct_dispatch` not `#[cfg(unix)]`-gated

**Severity**: P1 -- compilation failure on Windows

**Code locations**:

| Component | Location | Issue |
|-----------|----------|-------|
| Module declaration | `lib.rs:41` | `pub mod direct_dispatch;` -- unconditional |
| Module-level import | `direct_dispatch.rs:11` | `use tokio::net::UnixListener;` -- fails on Windows |
| Function signature | `direct_dispatch.rs:148` | `stream: tokio::net::UnixStream` -- fails on Windows |
| Call site: channel init | `lib.rs:1264` | `if self.config.direct_dispatch.is_some()` |
| Call site: listener start | `lib.rs:1316-1331` | `direct_dispatch::start_direct_dispatch_listener(...)` |
| Call site: bridge task | `lib.rs:1413-1428` | `if let Some(direct_rx) = direct_dispatch_rx` |
| Event loop handler | `lib.rs:1445-1446` | `LoopEvent::DirectDispatch(dispatch)` |
| Event loop drain | `lib.rs:1462-1463` | Same in tick drain block |
| Handler method | `lib.rs:3904-3938` | `async fn handle_direct_dispatch(...)` |
| Config struct | `config.rs:229` | `pub direct_dispatch: Option<DirectDispatchConfig>` |
| LoopEvent enum | `lib.rs:1343` | `DirectDispatch(webhook::WebhookDispatch)` |
| Test initialisers | `lib.rs:8388,8851,9893` + `adf.rs:287,391,514,634` + `agent_run_command.rs:770` + `agent_runner.rs:418` + `project_adf.rs:596` | `direct_dispatch: None` |

**Existing patterns**: The crate uses `#[cfg(unix)]` at function/block level (e.g. `agent_runner.rs:259`, `config.rs:1316`) but never at module level. There is no precedent for `#[cfg(unix)] pub mod` in this crate.

**Platform reality**: The CI runs only on self-hosted Linux (`bigbox`). However, `x86_64-pc-windows-gnu` is an installed rustup target, and Windows is a CI target for Python bindings and npm publishing. The orchestrator crate is not published to crates.io, but cross-compilation checks could be added in future.

**Impact of gating the module**: The `LoopEvent::DirectDispatch` variant and `handle_direct_dispatch` method live inside `lib.rs`, not inside the module. If we gate only the module, the LoopEvent enum variant and handler remain on all platforms. The handler never gets called on non-Unix because `config.direct_dispatch` would be `None` (the config struct field is `Option<DirectDispatchConfig>` and DirectDispatchConfig itself is platform-agnostic -- it's just a PathBuf).

**Recommended approach**: Gate the module declaration and all call sites that reference `direct_dispatch::` functions. The `LoopEvent::DirectDispatch` variant, `handle_direct_dispatch` method, and `DirectDispatchConfig` can remain unconditional -- they compile fine on all platforms (they don't use Unix-specific types). Only the listener startup and its channel wiring need gating.

### Finding 2: Unbounded `read_line` in `direct_dispatch.rs:158`

**Severity**: P2 -- robustness / defence-in-depth

**Current code**:
```rust
let mut reader = tokio::io::BufReader::new(stream);
let mut line = String::new();
let bytes_read = reader.read_line(&mut line).await?;
```

`read_line` reads until `\n` with no upper bound. A client sending data without a newline could consume unbounded memory.

**Mitigations already in place**:
- Socket permissions are 0600 (owner-only), set immediately after bind
- The socket path defaults to `/tmp/adf-ctl.sock`
- Only the orchestrator process owner can connect

**Comparison with webhook**: The webhook handler uses axum which has built-in request body size limits.

**Options**:

| Option | Mechanism | Pros | Cons |
|--------|-----------|------|------|
| A: `stream.take(limit)` | Wrap the UnixStream in `tokio::io::AsyncReadExt::take(8192)` before creating BufReader | Simple, one-line change, limits total bytes | Changes type -- need to adjust `write_response` since `take()` wraps the stream |
| B: Manual read with limit | Use `read_buf` in a loop with a fixed-size buffer | Full control | More code, error-prone |
| C: `BufReader::with_capacity` + check | Set capacity and check `line.len()` after read | Doesn't actually limit -- BufReader grows internally | False sense of security |

**Recommended approach**: Option A -- wrap the stream in `take(8192)` before creating BufReader. 8 KiB is generous for a JSON command (`{"agent":"meta-learning","context":"..."}` is typically < 200 bytes). The write_response function needs adjustment since it currently accesses the underlying stream via `reader.get_mut()`.

The cleanest fix: split the stream into read/write halves using `stream.into_split()`, wrap the read half in `take()`, and use the write half directly for responses.

### Finding 3: 265 auto-generated `.terraphim/learnings/*.md` files tracked in git

**Severity**: P2 -- repository hygiene

**Current state**:
- `.terraphim/learnings/` is NOT in `.gitignore`
- 265 files exist on disk, 31 are new in this PR (added across 4 commits)
- None exist on `main` branch -- all 31 were introduced by this branch
- The remaining 234 are untracked (shown in `git status` at conversation start)
- Learning files are auto-generated by the `terraphim-agent learn hook` PostToolUse hook
- Format: frontmatter with `id`, `command`, `exit_code`, `source` + error context

**Intent**: These are local development artefacts from the learning capture system. They record failed commands and their corrections for the developer's personal use.

**Evidence they should NOT be committed**:
- They contain machine-specific paths and error output
- They are auto-generated per-session (265 files in the working tree)
- No other branch/PR includes them
- The existing `.gitignore` pattern for `.beads/` (analogous local task state) is already present

**Recommended approach**: Add `.terraphim/learnings/` to `.gitignore` and unstage the 31 files introduced by this branch using `git rm --cached`.

### Finding 4: PR bundles three independent features

**Severity**: P2 -- reviewability

**Commit analysis** (47 commits total):
- **#1862 local config**: 13 commits (de500b2..6a3db18) -- earliest, already merged context
- **#1873 FffIndexer**: 12 commits (aad2016..2a08b87) -- merged via PR #1874
- **#1875 direct dispatch**: 7 commits (2e0b2bf..66026fc) -- the nominal feature
- **Metadata/misc**: 4 commits (Cargo.toml metadata, merge commits, Cargo.lock)
- **Docs**: 11 commits (research/design/verification/validation docs for all three features)

**Splitting feasibility**: Low. The features are interleaved chronologically and share merge commits. The #1873 FffIndexer work was already merged to main via PR #1874 (commit 2a08b87e7), so those changes will be in the diff because they're on this branch but were independently merged. The #1862 local config changes were the foundational work before the direct dispatch feature was layered on.

**Recommended approach**: Do NOT attempt to split the branch. Instead:
1. Update the PR title to reflect actual scope: "feat: adf-ctl direct dispatch, FffIndexer migration, local .terraphim config"
2. Add a structured PR description listing each feature area with its issue reference
3. The individual features have their own research/design/verification docs already

## Constraints

### Vital Few (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Must compile on installed targets | `x86_64-pc-windows-gnu` is an installed rustup target; future CI matrix expansion | `rustup target list --installed` shows it |
| Must not break existing tests | 14 direct-dispatch tests + full orchestrator suite | All currently pass |
| Defence-in-depth on local socket | Even with 0600 perms, unbounded reads are a poor practice | OWASP input validation guidelines |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Full Windows support for direct dispatch | UDS is inherently Unix; Windows named pipes would be a different feature |
| Authentication on UDS | Socket permissions (0600) are sufficient for local-only use |
| PR splitting | Commits are interleaved; cost exceeds benefit |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `#[cfg(unix)]` gating misses a call site | Low | Build failure on Windows | Verify with `cargo check --target x86_64-pc-windows-gnu` |
| `take()` wrapper breaks write path | Low | Compile error | Use `into_split()` to separate read/write |
| Unstaging learnings changes commit history | None | N/A | `git rm --cached` only affects index |

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `DirectDispatchConfig` compiles on Windows | Only contains `PathBuf` | Very low -- PathBuf is cross-platform | Yes |
| `handle_direct_dispatch` compiles on Windows | Only uses `WebhookDispatch` and `AgentDefinition` | Very low -- no Unix types | Yes |
| Learning files are not intentionally versioned | No `.gitignore` rule exists but no other branch tracks them | Low -- ask user | No |

## Recommendations

### Proceed: Yes

All four findings have clear, low-risk remediations:

1. **P1 cfg(unix)**: Gate `pub mod direct_dispatch;` and the three call sites in `run()` that reference `direct_dispatch::start_direct_dispatch_listener`. Keep `LoopEvent::DirectDispatch`, `handle_direct_dispatch`, and `DirectDispatchConfig` unconditional.

2. **P2 read_line**: Use `stream.into_split()` + `take(8192)` on the read half. Adjust `write_response` to accept the write half directly.

3. **P2 learnings**: Add `.terraphim/learnings/` to `.gitignore`, `git rm --cached` the 31 staged files.

4. **P2 scope**: Update PR title and description. No branch splitting.

## Next Steps

If approved, proceed to Phase 2 (Design) to specify exact file changes, function signatures, and test strategy for each remediation.
