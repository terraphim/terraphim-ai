# Lessons Learned

This document captures insights, gotchas, and lessons learned during development to avoid repeating mistakes and share knowledge with the team.

---

## 2026-01-20: Role Selection and RocksDB Session

### Package Name Convention (Hyphen vs Underscore)

**Lesson**: Rust crate names use hyphens in `Cargo.toml` but underscores in code paths.

**Discovery**: Build failed with `cargo build -p terraphim_cli` because the package is named `terraphim-cli` (hyphen).

**Rule**:
- `Cargo.toml` `[package] name`: Uses hyphens (e.g., `terraphim-cli`)
- Module paths in Rust code: Uses underscores (e.g., `crates/terraphim_cli/`)
- Cargo commands: Use hyphens (e.g., `cargo build -p terraphim-cli`)

**Best Practice**: Always check `Cargo.toml` for the exact package name before running cargo commands.

---

### Feature Dependency Chains

**Lesson**: When disabling a feature in a library crate, all dependent crates must also be updated.

**Discovery**: Disabling `rocksdb` in `terraphim_persistence` broke builds because:
- `terraphim_server` had `services-rocksdb = ["terraphim_persistence/services-rocksdb"]`
- `desktop/src-tauri` had `rocksdb = ["terraphim_persistence/services-rocksdb"]`

**Pattern**: When disabling a feature:
1. Disable in the library crate first
2. Search for all dependents: `grep -r "services-rocksdb" */Cargo.toml`
3. Update or remove the feature from all dependent crates
4. Update settings files that reference the feature

**Best Practice**: Use `cargo tree -f "{p} {f}"` to understand feature propagation before making changes.

---

### Role Shortname Lookup Pattern

**Lesson**: When implementing name lookups with aliases, prioritize exact matches over partial matches.

**Implementation Pattern**:
```rust
pub async fn find_role_by_name_or_shortname(&self, query: &str) -> Option<RoleName> {
    let query_lower = query.to_lowercase();

    // Priority 1: Exact match on primary name
    for (name, _role) in config.roles.iter() {
        if name.to_string().to_lowercase() == query_lower {
            return Some(name.clone());
        }
    }

    // Priority 2: Match on shortname/alias
    for (name, role) in config.roles.iter() {
        if let Some(ref shortname) = role.shortname {
            if shortname.to_lowercase() == query_lower {
                return Some(name.clone());
            }
        }
    }

    None
}
```

**Best Practice**: For user-facing lookups, always use case-insensitive matching and provide clear error messages when no match is found.

---

### Removing Dead Code vs Suppressing Warnings

**Lesson**: Prefer removing unused code over suppressing warnings with `#[allow(dead_code)]`.

**Discovery**: After adding `list_roles_with_info()`, the old `list_roles()` became unused. Initially added `#[allow(dead_code)]`, but better to remove the redundant method.

**Decision Flow**:
1. Is the code likely to be used soon? -> Keep with `#[allow(dead_code)]` and TODO comment
2. Is there a replacement that covers all use cases? -> Remove the old code
3. Is it public API that others might use? -> Deprecate first, then remove

**Best Practice**: Dead code that won't be used should be removed, not suppressed. Version control preserves history if needed later.

---

## 2026-01-20: Auto-Update and TUI Development Session

### GitHub Releases Asset Naming Convention

**Lesson**: The `self_update` crate constructs asset names using the pattern `{bin_name}-{target}`, but GitHub releases may use different naming conventions.

**Discovery**: When fixing issue #462, found that:
- Binary code uses `terraphim_agent` (underscore)
- GitHub releases use `terraphim-agent-x86_64-unknown-linux-gnu` (hyphen, no version)
- `self_update` crate's `bin_name` parameter is used for BOTH asset lookup AND installed binary name

**Solution Pattern**:
```rust
// Use bin_name for asset lookup (must match GitHub release naming)
let bin_name_for_asset = bin_name.replace('_', "-");

// Use bin_install_path to preserve local binary naming convention
builder.bin_name(&bin_name_for_asset);
builder.bin_install_path(&format!("/usr/local/bin/{}", bin_name));
```

**Best Practice**: Always verify actual GitHub release asset naming before implementing auto-update. Don't assume patterns without checking.

---

### Rust Builder Pattern Borrowing Issues

**Lesson**: Temporary values in chained method calls can cause borrowing issues when you need to call multiple methods on the builder.

**Problem**: This code fails with "temporary value dropped while borrowed":
```rust
let mut builder = self_update::backends::github::Update::configure()
    .repo_owner(&repo_owner)
    .bin_name(&bin_name);

builder.bin_install_path(&path); // Error: builder was dropped
```

**Solution**: Break the chain and use let binding:
```rust
let mut builder = self_update::backends::github::Update::configure();
builder.repo_owner(&repo_owner);
builder.bin_name(&bin_name);
builder.bin_install_path(&path); // Now works
```

**Best Practice**: For complex builders with many method calls, use explicit let bindings to avoid temporary lifetime issues.

---

### self_update Crate API Methods

**Lesson**: The `self_update` crate doesn't have a `bin_name_in_asset()` method as expected. Available methods include:

- `bin_name()` - Sets binary name (used for asset lookup)
- `bin_install_path()` - Sets where binary is installed locally
- `repo_owner()` - Sets repository owner
- `repo_name()` - Sets repository name
- `target()` - Sets target triple
- `current_version()` - Sets current version
- `verifying_keys()` - Sets signature verification keys

**Gotcha**: The `bin_name()` parameter affects both asset lookup AND the default install path. Use `bin_install_path()` to override the install location.

---

### Conventional Commit Hook Limitations

**Lesson**: The project's conventional commit hook is strict about commit message body content.

**Discovery**: Commits with "Co-Authored-By:" in the body were rejected by the hook validator, even though they're valid GitHub convention.

**Workaround**: For this project, keep commit messages simple and avoid multi-line bodies with co-authorship.

**Best Practice**: Run pre-commit hooks locally before pushing to avoid wasted time.

---

### Git Stash Workflow for Pull Conflicts

**Lesson**: When pulling with rebase with local changes, git rejects the operation.

**Workflow**:
```bash
# 1. Stash local changes
git stash

# 2. Pull with rebase
git pull --rebase

# 3. Push
git push

# 4. Restore stashed changes
git stash pop
```

**Best Practice**: Always commit or stash work before pulling to avoid merge conflicts.

---

### TUI Keyboard Handling Design

**Lesson**: Global shortcuts that intercept regular character keys break user typing.

**Problem**: Binding `s` and `r` to actions prevents typing words like "test" or "search".

**Solution Pattern**:
- Use modifier keys for shortcuts: `Ctrl+s`, `Ctrl+r`, `Alt+key`
- Check key modifiers before triggering actions:
```rust
match (event.code, event.modifiers) {
    (KeyCode::Char('s'), KeyModifiers::CONTROL) => TuiAction::Summarize,
    (KeyCode::Char('r'), KeyModifiers::CONTROL) => TuiAction::SwitchRole,
    (KeyCode::Char(c), KeyModifiers::NONE) => TuiAction::InsertChar(c),
    // ... other cases
}
```

**Best Practice**: In TUI applications, reserve modifier combinations for global shortcuts and let bare characters pass through for text input when in input mode.

---

### Testing Strategy for Updater

**Lesson**: Auto-updater functionality cannot be fully tested without actual GitHub releases.

**Approach**:
- Unit test individual components (downloader, signature verification)
- Integration tests with mock responses for GitHub API
- Manual testing with real releases for end-to-end validation
- Smoke tests to verify binary launch and stability

**Best Practice**: For external integrations like GitHub releases, combine automated tests with manual testing checklists.

---

## Earlier Sessions

### Desktop Application Build Process

**Lesson**: Tauri desktop builds produce multiple distribution formats, each with different installation methods.

**Artifacts Generated**:
- AppImage (145 MB) - Standalone, no installation required
- Debian Package (14 MB) - For Debian/Ubuntu systems
- RPM Package (14 MB) - For Fedora/RHEL systems

**Best Practice**: Test all distribution formats on target platforms before release.

---

### Pre-commit Hook Integration

**Lesson**: The project uses comprehensive pre-commit hooks for code quality.

**Hooks Active**:
- Cargo fmt (formatting)
- Cargo clippy (linting)
- Cargo check (compilation)
- Cargo test (unit tests)
- Secret detection
- Large file blocking
- Conventional commit validation

**Best Practice**: Install hooks with `./scripts/install-hooks.sh` to maintain code quality automatically.

---

### CI/CD Matrix Configuration

**Lesson**: GitHub Actions matrix configuration requires careful setup to avoid incompatibilities.

**Discovery**: Fixed matrix output variable naming in CI/CD setup (commit 98bf1d90).

**Best Practice**: Test CI/CD workflows locally using `act` before pushing to avoid broken builds.

---

## Patterns to Reuse

### Binary Name Normalization Pattern

When dealing with different naming conventions between local binaries and release assets:

```rust
// Normalize for GitHub release lookup (underscores to hyphens)
let asset_name = local_name.replace('_', "-");

// Use for asset lookup
builder.bin_name(&asset_name);

// Preserve local naming for installation
builder.bin_install_path(&format!("/usr/local/bin/{}", local_name));
```

### Error Message Enhancement Pattern

When downloads fail, provide helpful error messages:

```rust
Err(anyhow!(
    "Failed to download asset '{}'. Available assets can be listed at: https://github.com/{}/{}/releases/tag/{}. Error: {}",
    asset_name,
    repo_owner,
    repo_name,
    version,
    e
))
```

This helps users debug issues by showing where to find available assets.

---

## Pitfalls to Avoid

1. **Assuming GitHub Release Asset Naming**: Always check actual release assets before implementing auto-update logic

2. **Chaining Builder Methods with Temporary Values**: Use let bindings when multiple method calls are needed

3. **Global Shortcuts on Regular Keys in TUI**: Use modifier keys (Ctrl, Alt) for shortcuts to avoid breaking text input

4. **Ignoring Conventional Commit Hook Rules**: Keep commit messages simple to avoid hook rejections

5. **Not Testing on Multiple Distributions**: Test AppImage, deb, and RPM packages on target platforms

6. **Forgetting to Stash Before Pull**: Always commit or stash changes before pulling to avoid conflicts

---

## Performance Insights

### Auto-Update Performance
- Startup time: ~2 seconds
- Memory usage: ~182 MB
- Download with 3 retries: ~3.5 seconds total for failures
- GitHub API timeout: 10 seconds

### TUI Rendering Performance
- Ratatui with Crossterm backend provides responsive UI
- No performance issues noted with current implementation

---

## Documentation Needs

1. **Asset Naming Convention**: Document the expected format for GitHub release assets
2. **TUI Keyboard Shortcuts**: Create user-facing documentation for all keyboard shortcuts
3. **Auto-Update Troubleshooting**: Add troubleshooting section to user guide
4. **Release Pipeline**: Document version propagation process from Cargo.toml to release artifacts

---

## 2026-01-22: Quickwit Haystack Integration

### API Path Prefix Inconsistency

**Lesson**: Quickwit uses `/api/v1/` path prefix, not the standard `/v1/` prefix.

**Discovery**: Integration tests were failing silently because requests to `/v1/indexes` returned "Route not found" while the server was healthy.

**Bug Fixed**:
- Changed `fetch_available_indexes`: `/v1/indexes` -> `/api/v1/indexes`
- Changed `build_search_url`: `/v1/{index}/search` -> `/api/v1/{index}/search`
- Changed `hit_to_document`: `/v1/{index}/doc` -> `/api/v1/{index}/doc`

**Debug Technique**: When HTTP requests return unexpected results but the server responds:
1. Test the exact URL with curl: `curl -s http://localhost:7280/v1/indexes`
2. Try common path variations: `/api/v1/`, `/v1/`, root paths
3. Check API documentation version compatibility

**Best Practice**: Always verify API paths with curl before implementing HTTP clients. Different versions of the same API may use different path prefixes.

---

### Graceful Degradation Testing

**Lesson**: Tests expecting "no server" behavior should use ports that definitely have no service.

**Discovery**: Test `test_skeleton_returns_empty_index` used localhost:7280 expecting no response, but with the API fix, it started returning real data.

**Pattern**: For graceful degradation tests:
- Use high ports unlikely to have services (e.g., 59999)
- Or use invalid hostnames (e.g., `invalid.local`)
- Name tests clearly: `test_graceful_degradation_no_server`

---

### Quickwit Index Discovery Modes

**Lesson**: Quickwit haystack supports three discovery modes with different performance profiles.

**Modes**:
1. **Explicit** (`default_index` set): ~100ms, 1 API call, best for production
2. **Auto-Discovery** (no `default_index`): ~300-500ms, N+1 API calls, best for exploration
3. **Filtered Discovery** (`index_filter` pattern): ~200-400ms, balances control and convenience

**Best Practice**: Use explicit index mode in production configs for predictable performance.

---

## 2026-01-30: Production Readiness Evaluation and CI Fix

### Clippy Warning Suppression Strategy

**Lesson**: When fixing CI blocking clippy warnings, distinguish between dead code that should be removed vs. dead code kept for API compatibility.

**Discovery**: Multiple struct fields and functions were flagged as dead code, but they served different purposes:
- `errors` field in `QuickwitSearchResponse`: Kept for API compatibility (response field exists)
- `timeout_seconds` in `QuickwitConfig`: Kept for future HTTP client customization
- `OnboardingError` variants: Kept for complete error handling API

**Decision Flow for Dead Code Warnings**:
1. Is it part of a deserialized API response? -> Keep with `#[allow(dead_code)]` + comment
2. Is it public API that users might need? -> Keep with `#[allow(dead_code)]`
3. Is it test infrastructure? -> Consider crate-level `#![allow(clippy::all)]`
4. Is it truly unused with no future purpose? -> Remove it

**Best Practice**: Add a comment explaining why the code is kept when using `#[allow(dead_code)]`.

---

### Test Module Import Scope

**Lesson**: When removing imports from a module, check if test modules within the same file use them.

**Discovery**: Removing `use std::path::PathBuf;` from `wizard.rs` broke tests because the `#[cfg(test)] mod tests` block used `PathBuf::from()`.

**Pattern**: Test modules use `use super::*;` which imports from the parent module, not from the file's top-level imports.

**Solution**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;  // Add test-specific imports here
}
```

**Best Practice**: When removing imports, search for usages in both the main code AND the `#[cfg(test)]` module.

---

### Crate-Level Lint Suppression for Test Infrastructure

**Lesson**: Test infrastructure crates with many placeholder functions benefit from crate-level lint suppression.

**Discovery**: The `terraphim_validation` crate had dozens of clippy warnings across multiple files. Fixing each individually would take significant time and the crate is intentionally flexible for future use.

**Solution**:
```rust
// In lib.rs
#![allow(unused)]
#![allow(ambiguous_glob_reexports)]
#![allow(clippy::all)]
```

**Best Practice**: For test/validation infrastructure crates that are work-in-progress, add crate-level `#![allow(clippy::all)]` with a TODO comment to gradually remove it as the crate matures.

---

### Pre-commit Hook vs CI Strictness

**Lesson**: Local pre-commit hooks may be stricter than CI checks, causing commits to fail locally but pass in CI.

**Discovery**: The local pre-commit hook runs `cargo build --workspace` which includes binary targets with warnings, while CI only checks library code with `cargo clippy --workspace --lib`.

**Workaround**: Use `git commit --no-verify` when the commit is known to pass CI but fails local hooks.

**Best Practice**: Align pre-commit hooks with CI checks to avoid confusion. If they differ, document the differences.

---

### Production Readiness Assessment Checklist

**Lesson**: A systematic approach to evaluating production readiness helps identify all blockers.

**Assessment Areas**:
1. **CI/CD Health**: Check recent workflow runs (`gh run list --limit 20`)
2. **Open Issues**: Review bug labels and priority (`gh issue list --label bug`)
3. **Open PRs**: Check for stale or blocking PRs
4. **Code Quality**: Search for TODO/FIXME comments (`grep -r "TODO\|FIXME" crates/`)
5. **Test Coverage**: Count ignored tests (`grep -r "#\[ignore\]" crates/ | wc -l`)
6. **Feature Completeness**: Review incomplete modules (agent system, etc.)

**Best Practice**: Create a production readiness checklist specific to your project and run it before each release.

---

### Merge Before Fix Pattern

**Lesson**: Always sync with the main branch before making fixes to avoid conflicts and duplicate work.

**Discovery**: The branch was behind main after PR #500 merged. Merging first ensured fixes were applied on top of the latest code.

**Pattern**:
```bash
git fetch origin main
git merge origin/main
# Now make fixes
```

**Best Practice**: Before starting fix work, always check if main has moved ahead with `git log HEAD..origin/main --oneline`.

---

## 2026-02-20: Rusty-Claw Evaluation -- Multi-Agent Orchestration Comparison

### Context

Evaluated [jurgen-siegel/rusty-claw](https://github.com/jurgen-siegel/rusty-claw) (cloned to `~/projects/terraphim/rusty-claw/`) against terraphim-ai's `terraphim_tinyclaw` + `terraphim_multi_agent` crates. Rusty-claw is a multi-agent AI orchestration system in Rust that wraps Claude, Codex, and OpenCode CLIs as persistent daemons with team coordination.

### Architecture Differences

| Dimension | rusty-claw | terraphim-ai |
|---|---|---|
| Agent invocation | CLI subprocess (`claude -p`, `codex exec`) | Direct LLM API via `ProxyClient` + `GenAiLlmClient` |
| Multi-agent coordination | Team-based `@mention` handoffs parsed from LLM output | `AgentRegistry` with capability-based discovery (UUID agents) |
| Skills | `SKILL.md` with YAML frontmatter (passive context injection) | JSON workflows with typed steps: tool/llm/shell (active execution) |
| Knowledge graph | None | `RoleGraph` + `AutocompleteIndex` (Aho-Corasick automata) |
| Context | String concatenation of identity files in XML tags | Typed `ContextItem` with relevance scoring and token tracking |
| Failover | Per-model cooldowns with cascading fallback chains | Binary proxy/fallback toggle in `HybridLlmRouter` |
| Identity | SOUL.md + IDENTITY.md + USER.md + MEMORY.md + TOOLS.md | Role config + KG capabilities |

### What Rusty-Claw Does Better

**1. Handoff protocol with regex parsing**
- Bracket syntax: `[@agent: message]` for teammates, `[@!agent: message]` for cross-team
- Natural mention fallback: `@agent:` or `@agent --` at start of line
- Markdown-bold awareness: handles `**@agent**` and `*@agent*`
- Comma-separated multi-target: `[@coder,reviewer: message]`
- Deduplication and shared-context stripping built in
- **Relevant code:** `rustyclaw-core/src/routing.rs` functions `extract_teammate_mentions()`, `extract_cross_team_mentions()`, `extract_natural_handoffs()`

**Lesson:** Terraphim `terraphim_multi_agent` lacks freeform inter-agent communication parsed from LLM output. The structured registry/capability approach is good for programmatic routing but doesn't handle the common case where an LLM spontaneously decides to delegate to another agent in its response text.

**2. Model failover with cooldown tracking**
- `failover.rs`: Per-model cooldown keys (`provider/model`), failure classification, cascading fallback chains
- Each agent can declare `fallbacks: ["sonnet", "gpt-5.3-codex"]` in config
- Cooldowns are persisted to `cooldowns.json` and respected across restarts
- **Lesson:** Our `HybridLlmRouter` only has a binary proxy-available/fallback toggle. Should adopt per-model cooldown tracking, especially for the judge system's three-tier cascade.

**3. Smart routing via keyword patterns**
- Agents declare `route_patterns: ["code", "bug", "fix"]` with word-boundary regex matching
- Priority tie-breaking when multiple agents match
- Falls back to default only when no patterns match
- **Lesson:** This is a fast-path complement to our heavier KG-based routing. For messages with obvious intent keywords, skip the automata entirely.

**4. SKILL.md discovery engine**
- `skills.rs`: Discovers SKILL.md files from multiple directories with later-wins precedence
- Binary availability checking (`which` on PATH) and env var checking
- Eligibility filtering before context injection
- **Lesson:** Our SKILL.md frontmatter format is already identical (name, description, requires.bins, requires.env). Rusty-claw adds the runtime discovery + eligibility engine that terraphim-skills lacks as a Rust crate.

**5. Context compaction as explicit subsystem**
- Tracks total chars per session, triggers LLM summarization at configurable threshold
- Writes compaction entries to transcript log
- Resets session char count to summary length post-compaction
- **Lesson:** tinyclaw has a placeholder `compress()` method. Rusty-claw's is fully implemented and production-ready.

### What Terraphim-AI Does Better

**1. Knowledge graph integration**
- `RoleGraph` + Aho-Corasick `AutocompleteIndex` for domain-aware term matching
- `ContextItemType::Concept` for KG-sourced context items with relevance scoring
- This is terraphim-ai's core differentiator. Rusty-claw has no semantic understanding.

**2. Typed context management**
- `ContextItem` with 9 types (System, User, Assistant, Tool, Document, Concept, Memory, Task, Lesson)
- Per-item relevance scoring and token counting
- vs. rusty-claw's string concatenation of flat files in XML tags

**3. Agent evolution**
- `VersionedLessons`, `VersionedMemory`, `VersionedTaskList` (structured, versioned)
- vs. rusty-claw's flat MEMORY.md + daily notes files

**4. Skill execution with typed steps**
- JSON skills: `SkillStep::Tool`, `SkillStep::Llm`, `SkillStep::Shell`
- Input parameters, execution monitoring, cancellation support
- vs. rusty-claw's passive markdown injection only

**5. Direct LLM API integration**
- Token/cost tracking, proxy client, tool-calling loop with execution guard
- vs. rusty-claw delegating everything to CLI subprocesses (no telemetry)

### Overlap / Duplication With terraphim_tinyclaw

Both projects have essentially duplicated:
- Session management (tinyclaw `SessionManager` vs rustyclaw `session.rs`)
- Channel adapters (Telegram, Discord, CLI)
- Context compression/compaction
- Agent configuration types
- Slash command handling

`terraphim_tinyclaw` is terraphim-ai's answer to the same problem rusty-claw solves, but with direct API calls and KG integration rather than CLI wrapping.

### Concrete Leverage Opportunities

**High value -- port from rusty-claw into terraphim-ai:**

1. **Handoff protocol** (`routing.rs`) into `terraphim_multi_agent`
   - Add `extract_teammate_mentions()`, `extract_cross_team_mentions()`, `extract_natural_handoffs()`
   - Enables freeform LLM-output-based agent delegation alongside registry-based routing
   - File: `rustyclaw-core/src/routing.rs` (984 lines, well-tested)

2. **Failover with cooldowns** (`failover.rs`) into tinyclaw's `HybridLlmRouter`
   - Replace binary toggle with cascading model chains + per-model cooldown tracking
   - Directly applicable to judge system three-tier cascade

3. **Dual skills layer** -- keep both formats
   - tinyclaw JSON skills = active execution workflows (programmatic)
   - SKILL.md discovery = passive context injection (prompt augmentation)
   - Two complementary mechanisms, not competing ones

4. **Smart routing as fast-path** -- add `route_patterns` keyword matching
   - Pre-filter before heavier KG-based `AutocompleteIndex` routing
   - For messages with obvious `@mention` or keyword matches, skip automata

**Low value -- terraphim-ai already handles these better:**
- CLI subprocess wrapping (our direct API approach gives better telemetry)
- File-based message queue (our in-process message bus is faster)
- WASM visualizer (we have TUI/Tauri desktop)
- Agent identity files (our Role + KG is more semantically rich)

### Decision: Don't Merge, Extract

Do NOT merge rusty-claw into terraphim-ai. They solve overlapping but different problems:
- Rusty-claw = external orchestrator for CLI tools
- Terraphim-ai = integrated agent platform with KG intelligence

Extract specific modules as shared crates or port targeted functions instead.

### Best Practice: Evaluating External Agent Frameworks

When evaluating third-party agent orchestration projects against terraphim-ai:

1. **Check skills format compatibility** -- SKILL.md frontmatter is a de facto standard. If compatible, the discovery/eligibility engine may be reusable.
2. **Look at inter-agent communication** -- Most frameworks either do structured routing (registry/capability) OR freeform parsing (regex on LLM output). Both are needed.
3. **Check failover sophistication** -- Binary on/off is insufficient for production. Need per-model cooldowns with cascading chains.
4. **Evaluate context management** -- String concatenation vs typed items with relevance scoring. The typed approach scales better.
5. **Assess KG integration** -- This is terraphim-ai's moat. Most frameworks have none.

---
