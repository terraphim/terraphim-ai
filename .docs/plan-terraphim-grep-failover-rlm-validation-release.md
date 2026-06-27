# Implementation Plan: Terraphim Grep KG Failover, RLM Validation Tests, and Release Readiness

**Status**: Draft | Pending human approval before implementation
**Research Doc**: `.docs/research-terraphim-grep-failover-rlm-validation-release.md`
**Author**: OpenCode / Terraphim Engineer
**Date**: 2026-06-27
**Estimated Effort**: 1.5 days

## Overview

### Summary

This plan makes `terraphim-grep` usable without a knowledge graph, proves that `terraphim_rlm` validates commands before executing them, and aligns the installed toolchain versions with the `1.21.0` release.

### Approach

1. **Failover is a CLI gate change, not a search rewrite**: `fff-search` is already integrated; we only need to let the CLI start with an empty thesaurus and skip KG boosting.
2. **Validation coverage is test-first**: add focused unit tests using the real `LocalExecutor` and `KnowledgeGraphValidator` so Issue #2491 can be closed with evidence.
3. **Release readiness is bookkeeping**: commit `Cargo.lock`, bump `terraphim-clients` to `1.21.0`, build/install binaries, and tidy untracked docs.

### Scope

**In Scope:**
- `terraphim-grep` no-thesaurus failover to `fff-search` enhanced grep (`terraphim-clients`).
- `terraphim_rlm` QueryLoop validation-order unit test (`terraphim-ai`).
- Commit `terraphim-ai/Cargo.lock` (proptest dev-dependency).
- Bump `terraphim-clients` workspace version to `1.21.0` and update `terraphim_service` dependency.
- Build and install updated binaries locally.
- File Gitea issues and link them to the work.

**Out of Scope:**
- Changing `KgStrictness` semantics or retry policy (unless the spike reveals a gap).
- New release automation or CI changes.
- Refactoring `LlmBridge` for broader testability.

**Avoid At All Cost** (from 5/25 analysis):
- Re-implementing a grep engine when `fff-search` already exists.
- Adding a `--no-kg` manual flag instead of automatic failover.
- Broad `terraphim_rlm` refactors beyond the validation test.

## Architecture

### Component Diagram

```text
terraphim-grep CLI
  ├─ Thesaurus discovery (optional)
  ├─ HybridSearcher
  │   ├─ RoleGraph (empty thesaurus)
  │   └─ fff-search FilePicker.grep
  └─ GrepResult (concepts empty, chunks populated)

terraphim_rlm QueryLoop
  ├─ validate_command(input)
  │   └─ LocalExecutor.validate(input) -> KnowledgeGraphValidator
  └─ execute_command/execute_code
      └─ LocalExecutor (real; pass/fail controlled by thesaurus terms)
```

### Data Flow

**`terraphim-grep` failover:**

```text
query -> main.rs
  -> if thesaurus missing: empty Thesaurus(role_name)
  -> HybridSearcher::new(role_name, thesaurus)  // works with empty thesaurus
  -> HybridSearcher::search
      -> KG path returns [] concepts
      -> code path runs fff-search
  -> boost_chunks_with_kg(chunks, [])  // no-op
  -> GrepResult with empty concepts
```

**`terraphim_rlm` validation test:**

```text
test -> QueryLoop::execute_command(Command::Run("echo allowed_term"))
  -> LocalExecutor::validate -> KnowledgeGraphValidator (thesaurus=["echo", "allowed_term"])
  -> validation passes -> LocalExecutor::execute_command runs echo
  -> output contains "allowed_term"

test -> QueryLoop::execute_command(Command::Run("echo disallowed_term"))
  -> LocalExecutor::validate -> KnowledgeGraphValidator (thesaurus=["echo"])
  -> validation fails -> LocalExecutor::execute_command is NOT called
  -> output is the validation feedback message
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Make `code-search` a default feature in `terraphim_grep` | Failover must work for end users without remembering to enable features | Keep it optional — would leave failover disabled by default |
| Use empty `Thesaurus` as the no-KG sentinel | `HybridSearcher::new` already accepts it; existing test proves it works | Refactor `HybridSearcher` to be optional — larger change, same outcome |
| Add `info!` log when falling back | Users can see why KG concepts are absent | Silent fallback — harder to debug |
| Test `QueryLoop::execute_command` directly with real `LocalExecutor` + `KnowledgeGraphValidator` | No mocks; uses real validation and execution paths; proves ordering by observing side effects | Mock executor — violates project "no mocks in tests" rule |
| Treat `terraphim-clients` version bump as part of this plan | Installed binaries must match the latest release | Separate release cycle — would leave binaries stale longer |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Manual `--no-kg` flag | Fails the "works out of the box" requirement | Users still hit the error when they forget the flag |
| Bundled default thesaurus | Increases binary size and creates a maintenance hotspot | Diverges from "your knowledge, your boost" model |
| Mock executor for QueryLoop tests | Project rule: never use mocks in tests | Brittle tests that don't exercise real validation/execution paths |
| Full RLM strictness/escalation redesign | Issue #2491 is about validation ordering; behaviour already matches acceptance criteria | Scope creep and risk of changing working security semantics |
| Patching `terraphim-ai/Cargo.toml` for `terraphim-clients` | Separate repos/workspaces; cross-repo patches break resolution | Build instability |

### Simplicity Check

**What if this could be easy?**

- For grep: remove the hard error when thesaurus is missing, pass an empty thesaurus, and log a one-line info message.
- For RLM: add a test module that builds a real `QueryLoop` with `LocalExecutor` and a controlled thesaurus-backed validator, asserting pass/fail by inspecting command output.
- For release: bump one workspace version string and commit the lockfile.

**Senior Engineer Test**: The design is not overcomplicated; it reuses existing paths and adds minimal surface area.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### `terraphim-clients` repository

#### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_grep/Cargo.toml` | Add `code-search` to `default` features; keep `llm` |
| `crates/terraphim_grep/src/main.rs` | Make thesaurus optional; build empty thesaurus when none found; log failover |
| `crates/terraphim_grep/src/hybrid_searcher.rs` | Add `is_kg_configured()` helper or document empty-thesaurus semantics; add unit test |
| `crates/terraphim_grep/src/lib.rs` | Add no-thesaurus failover test |
| `Cargo.toml` | Bump `workspace.package.version` to `1.21.0` |
| `crates/terraphim_grep/Cargo.toml` | Update `terraphim_service` version to `1.21.0` (if published) or keep patch override |
| `RELEASE_NOTES_v1.21.0.md` | Create release notes (optional but recommended) |

### `terraphim-ai` repository

#### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_rlm/src/query_loop.rs` | Add real-executor validation-order unit tests |
| `Cargo.lock` | Commit existing proptest/terraphim_spawner changes |

#### New Files

None in `terraphim-ai` beyond test code inside `query_loop.rs`.

### Untracked Documentation

| Action | Files |
|--------|-------|
| Commit or remove | `.docs/adf-weekly-activity-*.md`, `.docs/design-*.md`, `.docs/plan-*.md`, `.docs/research-*.md`, `docs/handovers/*.md`, `docs/plans/*.md`, `docs/research/*.md`, `implementation-plan-2301.md` |

## API Design

### No new public APIs

The failover is transparent to callers. Existing public types remain unchanged:

```rust
pub struct GrepResult { ... }
pub struct HybridSearcher { ... }
pub async fn TerraphimGrep::search(&self, query: &str, options: GrepOptions) -> Result<GrepResult> { ... }
```

### Internal helper (optional)

In `main.rs`:

```rust
/// Build a thesaurus, falling back to an empty one when no project thesaurus exists.
async fn resolve_thesaurus(role_name: &str, explicit: Option<&Path>) -> Result<Thesaurus> {
    if let Some(path) = explicit {
        let automata_path = AutomataPath::from_local(path);
        return terraphim_automata::load_thesaurus(&automata_path)
            .await
            .with_context(|| format!("Failed to load thesaurus from {:?}", path));
    }
    if let Some(path) = find_default_thesaurus(role_name) {
        let automata_path = AutomataPath::from_local(&path);
        return terraphim_automata::load_thesaurus(&automata_path)
            .await
            .with_context(|| format!("Failed to load thesaurus from {:?}", path));
    }
    tracing::info!("No thesaurus found; running in fff-search enhanced grep mode");
    Ok(Thesaurus::new(role_name.to_string()))
}
```

### Test helpers (test-only)

In the `#[cfg(test)]` module of `crates/terraphim_rlm/src/query_loop.rs`:

```rust
/// Build a QueryLoop wired to a real LocalExecutor and a thesaurus-backed validator.
fn test_query_loop(
    thesaurus_terms: &[&str],
    strictness: KgStrictness,
) -> (QueryLoop<dyn ExecutionEnvironment>, SessionId) { ... }

/// Build a thesaurus containing the given terms.
fn test_thesaurus(terms: &[&str]) -> Thesaurus { ... }
```

No mock types are introduced. Validation pass/fail is controlled entirely by the terms present in the real `KnowledgeGraphValidator` thesaurus.

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `search_without_thesaurus_uses_fff_mode` | `terraphim_grep/src/lib.rs` | Empty thesaurus yields non-empty fff chunks and empty concepts |
| `main_accepts_missing_thesaurus` | `terraphim_grep/tests/no_thesaurus_cli.rs` (new) | CLI runs successfully with no `--thesaurus` in a temp directory |
| `run_command_validates_before_execution` | `terraphim_rlm/src/query_loop.rs` | Real executor: allowed `RUN` command produces shell output |
| `code_command_validates_before_execution` | `terraphim_rlm/src/query_loop.rs` | Real executor: allowed `CODE` command produces Python output |
| `validation_failure_blocks_run_command` | `terraphim_rlm/src/query_loop.rs` | Real executor: disallowed `RUN` command returns validation feedback, no shell output |
| `validation_failure_blocks_code_command` | `terraphim_rlm/src/query_loop.rs` | Real executor: disallowed `CODE` command returns validation feedback, no Python output |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `terraphim_grep` e2e no-thesaurus | `terraphim_grep/tests/no_thesaurus_cli.rs` | End-to-end CLI invocation returns JSON results with no concepts |

### Existing Tests to Keep Passing

- `terraphim_grep::tests::search_without_llm_degrades_to_search_only`
- `cargo test -p terraphim_rlm`
- `cargo clippy --workspace` for both repos

## Implementation Steps

### Step 1: `terraphim-grep` failover
**Repo**: `terraphim-clients`
**Files**: `crates/terraphim_grep/Cargo.toml`, `crates/terraphim_grep/src/main.rs`, `crates/terraphim_grep/src/lib.rs`
**Description**:
1. Add `"code-search"` to `default` features in `Cargo.toml`.
2. Replace the hard thesaurus error in `main.rs` with `resolve_thesaurus()` helper that returns an empty thesaurus when none is found.
3. Add unit test `search_without_thesaurus_uses_fff_mode`.
**Tests**: New unit test + existing test suite
**Estimated**: 3 hours

### Step 2: `terraphim_rlm` validation-order test
**Repo**: `terraphim-ai`
**Files**: `crates/terraphim_rlm/src/query_loop.rs`
**Description**:
1. Add test helpers in the `#[cfg(test)]` module to build a real `QueryLoop` with a `LocalExecutor` and a `KnowledgeGraphValidator` loaded with a controlled thesaurus.
2. Add four tests covering:
   - Allowed `Command::Run` validates then executes.
   - Allowed `Command::Code` validates then executes.
   - Disallowed `Command::Run` is blocked by validation feedback.
   - Disallowed `Command::Code` is blocked by validation feedback.
3. Assertions inspect the `ExecuteResult::Continue { output }` string for either command output or validation feedback; no mocks are used.
4. Run `cargo test -p terraphim_rlm` and `cargo clippy -p terraphim_rlm -- -D warnings`.
**Tests**: New unit tests
**Estimated**: 3 hours

### Step 3: Commit `terraphim-ai/Cargo.lock`
**Repo**: `terraphim-ai`
**Files**: `Cargo.lock`
**Description**:
1. Inspect the diff to confirm only `proptest` and related transitive deps were added.
2. Commit with message: `chore(deps): commit proptest dev-dependency for terraphim_spawner Refs #<issue>`.
**Tests**: `cargo check --workspace` must still pass
**Estimated**: 30 minutes

### Step 4: Version-align `terraphim-clients`
**Repo**: `terraphim-clients`
**Files**: `Cargo.toml`, `crates/terraphim_grep/Cargo.toml`, `crates/terraphim_agent/Cargo.toml`, etc.
**Description**:
1. Bump `workspace.package.version` from `1.20.5` to `1.21.0`.
2. Update `terraphim_service` dependency version in `terraphim_grep/Cargo.toml` to `1.21.0` if available on the registry; otherwise keep the workspace patch override and document it.
3. Optionally create `RELEASE_NOTES_v1.21.0.md`.
**Tests**: `cargo check --workspace` passes
**Estimated**: 1 hour

### Step 5: Build and install binaries
**Repo**: both
**Description**:
1. `cargo install --path terraphim-clients/crates/terraphim_grep --force`
2. `cargo install --path terraphim-clients/crates/terraphim_agent --force`
3. `cargo install --path terraphim-ai/crates/terraphim_rlm --force`
4. Verify versions: `terraphim-grep --version`, `terraphim-agent --version`, `terraphim-rlm --version`.
**Tests**: Manual version checks
**Estimated**: 1 hour

### Step 6: Documentation cleanup
**Repo**: `terraphim-ai`
**Description**:
1. Review untracked `.docs/`, `docs/handovers/`, `docs/plans/`, `docs/research/` files.
2. Commit design/research artefacts that are still relevant; remove obsolete drafts.
**Tests**: `git status --short` should show only intentional changes
**Estimated**: 1 hour

### Step 7: Gitea issue/PR workflow
**Repo**: both
**Description**:
1. Create Gitea issues for Step 1, Step 2, and release alignment.
2. Link Step 2 to Issue #2491.
3. Open PRs with `Refs #<issue>` commits.
4. Close issues after merge and binary verification.
**Estimated**: 1 hour

## Rollback Plan

If issues are discovered:

1. Revert the `code-search` default feature change in `terraphim_grep/Cargo.toml`.
2. Revert `main.rs` thesaurus resolution to the previous mandatory path.
3. Remove new test modules (safe — additive only).
4. Restore previous `Cargo.lock` from git if needed.
5. Re-install previous binaries with `cargo install --version 1.20.5 terraphim_grep terraphim_agent`.

No database migrations or stateful changes are involved.

## Dependencies

### New Dependencies

None.

### Feature Changes

| Crate | Feature | Change |
|-------|---------|--------|
| `terraphim_grep` | `code-search` | Move from optional to default |
| `terraphim_grep` | `llm` | Remains default |

### Version Updates

| Crate/Workspace | From | To | Reason |
|-----------------|------|-----|--------|
| `terraphim-clients` workspace | `1.20.5` | `1.21.0` | Align with `terraphim-ai` release |
| `terraphim_service` (grep dep) | `1.20.4/1.20.5` | `1.21.0` | Version parity |

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| `terraphim-grep` no-thesaurus startup | < 500 ms | Manual timing on repo root |
| `terraphim-grep` binary size delta | < +10 % | `ls -la target/release/terraphim-grep` before/after |
| `terraphim_rlm` test duration | < 5 s for new tests | `cargo test -p terraphim_rlm` |

### Benchmarks to Add

None required; existing `hybrid_search` bench remains valid.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Confirm `terraphim_service 1.21.0` is published on Gitea registry | Pending | Release owner |
| Decide whether to keep or delete each untracked doc file | Pending | Project owner |
| Approve making `code-search` a default feature | Pending | Human reviewer |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received

## Cross-Repository Notes

| Change | Repository | Branch naming |
|--------|------------|---------------|
| Grep failover | `terraphim-clients` | `task/<idx>-grep-kg-failover` |
| RLM validation test | `terraphim-ai` | `task/<idx>-rlm-validation-test` |
| Cargo.lock + docs | `terraphim-ai` | `task/<idx>-release-hygiene` |
| Version bump clients | `terraphim-clients` | `task/<idx>-bump-1.21.0` |

Push order (per Remote Sync Protocol):

1. Push functional branches to `origin` first.
2. After merge, push `main` to `origin`, then to `gitea`.
3. Verify `git diff origin/main gitea/main --stat` is empty.
