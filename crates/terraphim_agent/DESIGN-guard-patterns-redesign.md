# Implementation Plan: Redesign guard_patterns.rs with Terraphim Thesaurus Matching

**Status**: Draft
**Research Doc**: RESEARCH-guard-patterns-redesign.md
**Author**: Claude (disciplined-design)
**Date**: 2026-02-14
**Estimated Effort**: 1-2 days

## Overview

### Summary
Replace all 12 static regex patterns in `CommandGuard` with terraphim's Aho-Corasick `find_matches` engine driven by two JSON thesaurus files (destructive patterns + safe allowlist). Zero regex retained -- all command variants are enumerated as thesaurus entries with LeftmostLongest matching.

### Approach
Pure thesaurus-driven matching using `terraphim_automata::find_matches`. Each destructive command variant (including flag orderings like `rm -rf`, `rm -fr`) is a thesaurus key mapping to a concept category via `nterm`. The `url` field carries the block reason. A second allowlist thesaurus handles safe overrides.

This directly leverages the same infrastructure that `CommandRegistry` already uses for command discovery -- the `Thesaurus` type, `find_matches`, and `load_thesaurus_from_json`.

### Scope
**In Scope:**
- Replace `CommandGuard` internals to use `find_matches` from `terraphim_automata`
- Create `guard_destructive.json` thesaurus with all destructive commands
- Create `guard_allowlist.json` thesaurus with safe overrides
- Embed both via `include_str!`, with `--guard-thesaurus` CLI override
- Add newly covered commands: `rmdir`, `chmod`, `chown`, `rm` (without -rf), `git commit --no-verify`, `shred`, `truncate`, `dd`, `mkfs`

**Out of Scope:**
- Changes to `CommandValidator` or `CommandRegistry` (separate concern)
- Remote loading of thesaurus files
- Fuzzy matching for commands
- Integration with knowledge graph API (rolegraph)

**Avoid At All Cost:**
- Any regex patterns (the whole point is eliminating them)
- Building a custom Aho-Corasick automaton outside terraphim_automata
- Adding new crate dependencies
- Changing the `GuardResult` JSON output format
- Making the guard async (it must stay synchronous for hook performance)

## Architecture

### Component Diagram
```
guard_patterns.rs (modified)
  |
  +-- CommandGuard
  |     +-- destructive_thesaurus: Thesaurus    (loaded once from JSON)
  |     +-- allowlist_thesaurus: Thesaurus       (loaded once from JSON)
  |     +-- check(&self, command: &str) -> GuardResult
  |           |
  |           +-- 1. find_matches(command, allowlist_thesaurus) -> if any match, allow
  |           +-- 2. find_matches(command, destructive_thesaurus) -> if any match, block
  |           +-- 3. else allow
  |
  +-- guard_destructive.json  (embedded via include_str!)
  +-- guard_allowlist.json    (embedded via include_str!)
```

### Data Flow
```
Command string
  -> CommandGuard::check()
    -> terraphim_automata::find_matches(command, allowlist_thesaurus, false)
      -> Aho-Corasick LeftmostLongest scan (O(n), case-insensitive)
      -> if matches found: return GuardResult::allow
    -> terraphim_automata::find_matches(command, destructive_thesaurus, false)
      -> Aho-Corasick LeftmostLongest scan (O(n), case-insensitive)
      -> if matches found:
        -> use matched.normalized_term.url as reason
        -> use matched.term as pattern
        -> return GuardResult::block
    -> return GuardResult::allow (default)
```

### Key Design Decisions
| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Zero regex, pure thesaurus | terraphim_automata LeftmostLongest already handles multi-word patterns like `rm -rf`; regex duplicates this | Hybrid thesaurus+regex (rejected: redundant) |
| Enumerate flag variants as thesaurus entries | `rm -rf`, `rm -fr`, `rm -rfi` are separate keys mapping to same concept | Single pattern with regex flag matching (rejected: that IS regex) |
| `url` field as block reason | NormalizedTerm already has `url: Option<String>` -- repurpose for guard messages | New field on NormalizedTerm (rejected: modifies shared type) |
| `nterm` as concept category | e.g. `destructive_file_removal`, `git_destructive_reset` -- natural knowledge graph concept | Flat list without categories (rejected: loses semantic grouping) |
| Allowlist as separate thesaurus | Clean separation; checked first, same as current logic | Single thesaurus with special marker (rejected: more complex) |
| `include_str!` for defaults | Binary is self-contained; no file-not-found at runtime | File-only loading (rejected: fragile deployment) |
| Synchronous API | Guard runs in pre-tool-use hook, must be fast | Async (rejected: unnecessary for embedded JSON) |

### Eliminated Options (Essentialism)
| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| ReplacementService wrapper | We only need detection, not replacement output | Unnecessary abstraction layer |
| CommandValidator integration | Different concern (risk levels, rate limiting, roles) | Scope creep, separate PR |
| Logseq markdown builder | JSON is simpler and more direct for guard rules | Over-engineering the configuration format |
| Knowledge graph API integration | Guard must work offline without API | Availability dependency |
| Custom case-sensitive matching mode | Aho-Corasick is case-insensitive; enumerate case-sensitive variants | Modifying terraphim_automata core |

### Simplicity Check

> "Minimum code that solves the problem. Nothing speculative."

**What if this could be easy?**

Load two JSON thesauruses. Call `find_matches` twice per command (allowlist first, destructive second). Return allow/block. That is the entire implementation.

The `find_matches` function already handles: building the Aho-Corasick automaton, LeftmostLongest matching, case-insensitive scanning, filtering short patterns, returning matched terms with their normalized concepts. We just call it.

**Nothing Speculative Checklist:**
- [x] No features the user did not request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### New Files
| File | Purpose |
|------|---------|
| `crates/terraphim_agent/data/guard_destructive.json` | Destructive command thesaurus |
| `crates/terraphim_agent/data/guard_allowlist.json` | Safe command allowlist thesaurus |

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_agent/src/guard_patterns.rs` | Replace regex with `find_matches`; load thesaurus from JSON |
| `crates/terraphim_agent/src/main.rs` | Add `--guard-thesaurus` CLI flag to guard subcommand |

### Deleted Files
None (guard_patterns.rs is modified, not deleted).

## API Design

### Public Types (unchanged)
```rust
/// GuardResult stays exactly the same -- backward compatible
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardResult {
    pub decision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}
```

### Modified Types
```rust
/// CommandGuard -- internal structure changes, public API stays same
pub struct CommandGuard {
    destructive_thesaurus: Thesaurus,
    allowlist_thesaurus: Thesaurus,
}

impl CommandGuard {
    /// Create guard with embedded default thesauruses
    pub fn new() -> Self { ... }

    /// Create guard with custom thesaurus JSON strings
    pub fn from_json(
        destructive_json: &str,
        allowlist_json: &str,
    ) -> Result<Self, terraphim_automata::TerraphimAutomataError> { ... }

    /// Check a command -- public API unchanged
    pub fn check(&self, command: &str) -> GuardResult { ... }
}
```

### Thesaurus JSON Schema
```json
{
  "name": "guard_destructive",
  "data": {
    "<command-to-match>": {
      "id": <concept-group-id>,
      "nterm": "<concept-category>",
      "url": "<block-reason-message>"
    }
  }
}
```

Example entries:
```json
{
  "name": "guard_destructive",
  "data": {
    "git reset --hard": {
      "id": 1, "nterm": "git_destructive_reset",
      "url": "git reset --hard destroys uncommitted changes. Use 'git stash' first."
    },
    "git checkout -- ": {
      "id": 2, "nterm": "git_discard_changes",
      "url": "git checkout -- discards uncommitted changes permanently. Use 'git stash' first."
    },
    "rm -rf": {
      "id": 3, "nterm": "destructive_file_removal",
      "url": "rm -rf is destructive. List files first, then delete individually with permission."
    },
    "rm -fr": {
      "id": 3, "nterm": "destructive_file_removal",
      "url": "rm -rf is destructive. List files first, then delete individually with permission."
    },
    "rmdir": {
      "id": 4, "nterm": "directory_removal",
      "url": "rmdir removes directories. Verify contents first."
    },
    "chmod": {
      "id": 5, "nterm": "permission_change",
      "url": "chmod changes file permissions. Verify the target and mode."
    },
    "chown": {
      "id": 5, "nterm": "permission_change",
      "url": "chown changes file ownership. Verify the target."
    }
  }
}
```

Allowlist example:
```json
{
  "name": "guard_allowlist",
  "data": {
    "git checkout -b ": {
      "id": 1, "nterm": "safe_git_branch",
      "url": "Creating a new branch is safe."
    },
    "git restore --staged": {
      "id": 2, "nterm": "safe_git_unstage",
      "url": "Unstaging files is safe."
    },
    "git clean -n": {
      "id": 3, "nterm": "safe_git_dry_run",
      "url": "Dry run is safe."
    },
    "git push --force-with-lease": {
      "id": 4, "nterm": "safe_force_push",
      "url": "Force-with-lease is safer than --force."
    },
    "rm -rf /tmp/": {
      "id": 5, "nterm": "safe_tmp_cleanup",
      "url": "Cleaning temp directories is safe."
    },
    "rm -rf /var/tmp/": {
      "id": 5, "nterm": "safe_tmp_cleanup",
      "url": "Cleaning temp directories is safe."
    }
  }
}
```

### Concept Categories (knowledge graph taxonomy)

| Concept ID | nterm | Commands Mapped |
|-----------|-------|-----------------|
| 1 | `git_destructive_reset` | `git reset --hard`, `git reset --merge` |
| 2 | `git_discard_changes` | `git checkout -- `, `git checkout <ref> -- `, `git restore`, `git restore --worktree` |
| 3 | `destructive_file_removal` | `rm -rf`, `rm -fr`, `rm -rfi`, `rm -fR`, `shred`, `unlink` |
| 4 | `directory_removal` | `rmdir` |
| 5 | `permission_change` | `chmod`, `chown` |
| 6 | `git_clean_untracked` | `git clean -f`, `git clean -fd`, `git clean -fx`, `git clean -xf` |
| 7 | `git_force_push` | `git push --force`, `git push -f` |
| 8 | `git_branch_force_delete` | `git branch -D` |
| 9 | `git_stash_destroy` | `git stash drop`, `git stash clear` |
| 10 | `git_hook_bypass` | `git commit --no-verify`, `git push --no-verify` |
| 11 | `disk_wipe` | `dd if=/dev/zero`, `dd if=/dev/urandom`, `mkfs`, `fdisk` |
| 12 | `file_truncation` | `truncate` |
| 13 | `dangerous_rm` | `rm` (bare rm without safe flags, catches `rm file.txt`) |

Note on concept 13 (`dangerous_rm`): Matching bare `rm` is aggressive. The thesaurus entry `"rm "` (with trailing space) catches `rm file.txt`. The allowlist entries `rm -rf /tmp/` and `rm -rf /var/tmp/` override for safe temp cleanup. The LeftmostLongest matching ensures `rm -rf` matches before bare `rm ` when both are in the input.

## Test Strategy

### Unit Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_git_checkout_double_dash_blocked` | `guard_patterns.rs` | Existing: verify checkout -- is blocked |
| `test_git_checkout_branch_allowed` | `guard_patterns.rs` | Existing: verify checkout -b is allowed |
| `test_git_reset_hard_blocked` | `guard_patterns.rs` | Existing: verify reset --hard blocked |
| `test_git_restore_staged_allowed` | `guard_patterns.rs` | Existing: verify restore --staged allowed |
| `test_rm_rf_blocked` | `guard_patterns.rs` | Existing: verify rm -rf blocked |
| `test_rm_rf_tmp_allowed` | `guard_patterns.rs` | Existing: verify rm -rf /tmp/ allowed |
| `test_git_push_force_blocked` | `guard_patterns.rs` | Existing: verify push --force blocked |
| `test_git_push_force_with_lease_allowed` | `guard_patterns.rs` | Existing: verify --force-with-lease allowed |
| `test_git_clean_blocked` | `guard_patterns.rs` | Existing: verify clean -fd blocked |
| `test_git_clean_dry_run_allowed` | `guard_patterns.rs` | Existing: verify clean -n allowed |
| `test_git_stash_drop_blocked` | `guard_patterns.rs` | Existing: verify stash drop blocked |
| `test_git_status_allowed` | `guard_patterns.rs` | Existing: verify safe commands pass |
| `test_normal_command_allowed` | `guard_patterns.rs` | Existing: verify cargo build passes |
| `test_rmdir_blocked` | `guard_patterns.rs` | NEW: rmdir detected |
| `test_chmod_blocked` | `guard_patterns.rs` | NEW: chmod detected |
| `test_chown_blocked` | `guard_patterns.rs` | NEW: chown detected |
| `test_bare_rm_blocked` | `guard_patterns.rs` | NEW: rm file.txt detected |
| `test_git_commit_no_verify_blocked` | `guard_patterns.rs` | NEW: --no-verify detected |
| `test_shred_blocked` | `guard_patterns.rs` | NEW: shred detected |
| `test_truncate_blocked` | `guard_patterns.rs` | NEW: truncate detected |
| `test_dd_blocked` | `guard_patterns.rs` | NEW: dd if= detected |
| `test_mkfs_blocked` | `guard_patterns.rs` | NEW: mkfs detected |
| `test_rm_fr_blocked` | `guard_patterns.rs` | NEW: rm -fr (flag reorder) detected |
| `test_custom_thesaurus` | `guard_patterns.rs` | NEW: from_json constructor works |
| `test_git_checkout_orphan_allowed` | `guard_patterns.rs` | NEW: checkout --orphan allowed |
| `test_leftmost_longest_priority` | `guard_patterns.rs` | NEW: verify `rm -rf /tmp/` matches allowlist before `rm -rf` |

### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_guard_json_output` | `guard_patterns.rs` | Verify JSON serialization unchanged |
| `test_thesaurus_load_from_embedded` | `guard_patterns.rs` | Verify include_str! loading works |

## Implementation Steps

### Step 1: Create Thesaurus JSON Files
**Files:** `crates/terraphim_agent/data/guard_destructive.json`, `crates/terraphim_agent/data/guard_allowlist.json`
**Description:** Define all destructive command patterns and safe overrides as thesaurus entries
**Tests:** JSON validity (serde_json parsing)
**Estimated:** 2 hours

Key decisions for entries:
- Each flag ordering is a separate entry (e.g., `rm -rf`, `rm -fr`)
- Multi-word entries work because Aho-Corasick handles them natively
- Trailing space on entries like `"rm "` prevents matching inside words like `rm -rf` (but LeftmostLongest handles this -- `rm -rf` is longer and wins)
- `url` field carries the human-readable block reason

### Step 2: Rewrite CommandGuard
**Files:** `crates/terraphim_agent/src/guard_patterns.rs`
**Description:** Replace regex-based internals with thesaurus-driven `find_matches`
**Tests:** All existing tests must pass, plus new tests
**Dependencies:** Step 1
**Estimated:** 3 hours

Changes:
1. Remove `use regex::Regex;`
2. Remove `DestructivePattern` and `SafePattern` structs
3. Add imports: `terraphim_automata::{find_matches, load_thesaurus_from_json}`, `terraphim_types::Thesaurus`
4. Replace `CommandGuard` fields with two `Thesaurus` instances
5. `new()`: load from `include_str!("../data/guard_destructive.json")` and `include_str!("../data/guard_allowlist.json")`
6. `from_json()`: new constructor accepting custom JSON strings
7. `check()`: call `find_matches` for allowlist first, then destructive; extract reason from `matched.normalized_term.url`

### Step 3: Add CLI Flag
**Files:** `crates/terraphim_agent/src/main.rs`
**Description:** Add `--guard-thesaurus <path>` and `--guard-allowlist <path>` flags to the guard subcommand
**Tests:** Manual CLI testing
**Dependencies:** Step 2
**Estimated:** 1 hour

Changes:
- Add optional path args to `Command::Guard` enum variant
- If provided, read file contents and pass to `CommandGuard::from_json()`
- If not provided, use default `CommandGuard::new()`

### Step 4: Update Tests
**Files:** `crates/terraphim_agent/src/guard_patterns.rs` (test module)
**Description:** Add all new test cases for newly covered commands
**Tests:** Self-referential (they ARE the tests)
**Dependencies:** Step 2
**Estimated:** 1 hour

## Rollback Plan

If issues discovered:
1. Revert to previous `guard_patterns.rs` (git revert)
2. The JSON files are new additions and can be deleted
3. CLI flag changes in main.rs are additive and can be removed

No feature flag needed -- the old code path is simply replaced. Git history provides the rollback.

## Dependencies

### New Dependencies
None. All crates already in dependency tree:
- `terraphim_automata` (already in Cargo.toml)
- `terraphim_types` (already in Cargo.toml)

### Dependencies Removed
- `regex` crate is no longer needed by guard_patterns.rs (may still be needed elsewhere in terraphim_agent)

## Performance Considerations

### Expected Performance
| Metric | Target | Current | Expected |
|--------|--------|---------|----------|
| Guard check latency | < 1ms | ~0.1ms (regex) | ~0.2ms (Aho-Corasick build + scan) |
| Memory for guard | < 10MB | ~100KB (compiled regex) | ~200KB (thesaurus + automaton) |

Note: `find_matches` builds the Aho-Corasick automaton on every call from the thesaurus. For the guard use case with ~50-100 patterns, this is negligible (< 0.1ms to build). If profiling shows this matters, a future optimization can pre-build and cache the automaton, but that requires changes to `terraphim_automata` and is out of scope.

### Why No Caching Needed Now
The guard thesaurus is small (~100 entries). Aho-Corasick builder for 100 short patterns takes microseconds. The guard is called once per Bash tool use in Claude Code hooks -- not in a hot loop. Premature optimization would add complexity for no measurable benefit.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify `rm ` with trailing space does not cause false positives | To verify in Step 4 | Implementation |
| Confirm case-insensitive matching handles `git branch -D` vs `-d` | Known: AC is case-insensitive, so `-D` matches `-d`. Block `-D` specifically by using longer pattern `git branch -d` for allowlist if needed | Implementation |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
