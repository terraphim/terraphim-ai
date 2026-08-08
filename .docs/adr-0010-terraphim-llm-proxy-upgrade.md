# ADR-0010: Upgrade TinyClaw proxy to use `terraphim-llm-proxy`

**Status**: PROPOSED (per user investigation 2026-08-08)
**Deciders**: Hermes Agent
**Date**: 2026-08-08
**Issue**: terraphim-ai#3166 (Wave 5 partial) + investigation-report item 4

## Context

The TinyClaw `terraphim_tinyclaw/src/proxy/chat.rs` currently returns `[tinyclaw echo] <user message>` instead of an actual LLM proxy. This was documented as an intentional stub in the wave-2 commit, with a comment claiming `terraphim-llm-proxy` is "unbuildable in isolation".

**Investigation finding (2026-08-08)**: The "unbuildable" claim is **outdated**. The constraint that introduced the block (transitive `terraphim_types` git dep branching `github.com`) has been resolved by the polyrepo extraction. `terraphim-llm-proxy 0.1.12` builds successfully with `git clone --recurse-submodules`. The user's instruction "upgrade and update as needed" motivated verifying this.

## Decision

**Adopt `terraphim-llm-proxy` as the LLM proxy backend in TinyClaw**, replacing the echo stub. Use it as a git dep (not a terraphim-registry dep, since it's not published there).

Per the user "ignore licensing" directive, FSL-1.1-MIT is accepted.

## Verification (already done)

1. ✅ Cloned `https://github.com/terraphim/terraphim-llm-proxy.git` with `--recurse-submodules`
2. ✅ `cargo check` on v0.1.12 returns 0 (clean build)
3. ✅ Identified the blocker: `claude_code_agents` git submodule (now resolved via `--recurse-submodules`)
4. ✅ Confirmed `v1.0.0-priority-routing` is the latest tag (newer than v0.1.12). v0.1.12 chosen for stability (recently bumped, bottom of stable branch).

## Plan (in 4 steps)

### Step 1: Add `terraphim-llm-proxy` to TinyClaw's Cargo.toml

```toml
[dependencies]
terraphim-llm-proxy = { git = "https://github.com/terraphim/terraphim-llm-proxy", tag = "v0.1.12" }
```

Requires `--recurse-submodules` flag at clone time. CI workflow needs update.

### Step 2: Configure deny.toml (already done)

- ✅ Added `FSL-1.1-MIT` to `allow` list
- ✅ Added `https://github.com/terraphim/terraphim-llm-proxy.git` to `allow-git`
- ✅ cargo deny check still ok (no new failures)

### Step 3: Replace proxy stub with real LLM proxy

The current proxy at `terraphim_tinyclaw/src/proxy/chat.rs` returns an echo. Replace with:

```rust
use terraphim_llm_proxy::server::run_listen;
use terraphim_llm_proxy::config::Config;

async fn start_real_proxy(state: ProxyState) -> Result<Server, anyhow::Error> {
    let config = Config::from_env()?;
    run_listen(config, state).await
}
```

Preserve OpenAI-compatible `/v1/models` + `/v1/chat/completions` shape at the TinyClaw layer.

### Step 4: Tests + commit

- Existing 7 proxy contract tests verify Hermes shape — keep
- Add hermetic env scrub (Wave 0)
- Add 1 integration test: when proxy hits real LLM endpoint, response shape matches OpenAI

## Rejected Alternatives

### Use crates.io `genai` 0.6.5 directly

- **Pros**: No git submodule dependency.
- **Cons**: Doesn't have terraphim's "intelligent routing" or provider-specific routing logic.
- **Verdict**: Rejected. Loses the value-add over plain LLM client.

### Keep echo stub

- **Pros**: Zero new deps, no build risk.
- **Cons**: TinyClaw is not a real LLM proxy and blocks downstream work.
- **Verdict**: Rejected. Per user "upgrade and update as needed".

### Use `terraphim-llm-proxy` via `path = "/home/alex/..."`

- **Pros**: No git submodule issue.
- **Cons**: Not portable, hardcodes a path (dcg guard would block in CI).
- **Verdict**: Rejected. Git dep is the right pattern for sibling repos.

## Risk

| Risk | Mitigation |
|------|------------|
| Git submodule breaks CI on shallow clones | Document `--recurse-submodules` in CI workflow |
| `terraphim-llm-proxy` adds 50+ transitive deps | Acceptable; `kache` warms the build cache |
| FSL-1.1-MIT not OSI-approved | Already accepted in deny.toml per user |
| Breaking changes in v1.0.0 | Pin to v0.1.12 (stable). Upgrade separately. |

## References

- `https://github.com/terraphim/terraphim-llm-proxy` (v0.1.12, v1.0.0-priority-routing)
- `https://github.com/terraphim/rust-genai` (used internally by llm-proxy)
- `crates/terraphim_tinyclaw/src/proxy/chat.rs` (current echo stub)
- git log: investigation 2026-08-08

## Status

- ✅ Investigation: complete (v0.1.12 builds)
- ✅ deny.toml: updated (FSL-1.1-MIT, allow-git)
- ⏳ Step 1: TODO (add git dep to Cargo.toml)
- ⏳ Step 3: TODO (replace echo stub)
- ⏳ Step 4: TODO (tests + commit)

This ADR documents the upgrade path; the actual implementation is a separate refactor (not in this turn).
