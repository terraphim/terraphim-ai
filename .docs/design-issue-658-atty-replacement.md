# Implementation Plan: Replace atty with std::io::IsTerminal

**Status**: Draft
**Research Doc**: [.docs/research-issue-658-659-atty-fxhash.md](research-issue-658-659-atty-fxhash.md)
**Author**: Terraphim AI
**Date**: 2026-03-11
**Estimated Effort**: 30 minutes

---

## Overview

### Summary
Replace all usages of the unmaintained `atty` crate with `std::io::IsTerminal` (stable since Rust 1.70), then remove the `atty` dependency from Cargo.toml.

### Approach
Simple find-and-replace migration:
1. Replace `atty::is(atty::Stream::Stdout)` with `std::io::stdout().is_terminal()`
2. Replace `atty::is(atty::Stream::Stdin)` with `std::io::stdin().is_terminal()`
3. Add `use std::io::IsTerminal;` where needed
4. Remove `atty` from Cargo.toml

### Scope

**In Scope:**
- Replace 4 atty usages across 4 files
- Remove atty dependency from terraphim_agent/Cargo.toml
- Update feature flags to remove atty reference
- Verify compilation and tests pass

**Out of Scope:**
- fxhash replacement (transitive dependency, handled separately)
- Any behavioral changes to TTY detection
- Changes to any other dependencies

**Avoid At All Cost:**
- Changing TTY detection behavior
- Adding new dependencies
- Modifying unrelated code

---

## Architecture

### Simplicity Check

**What if this could be easy?**
The simplest approach is a direct API swap:
- `atty::is(Stream::Stdout)` → `std::io::stdout().is_terminal()`
- `atty::is(Stream::Stdin)` → `std::io::stdin().is_terminal()`

This is a mechanical transformation with identical semantics.

---

## File Changes

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim-session-analyzer/src/main.rs` | Replace atty usage with IsTerminal |
| `crates/terraphim_agent/src/main.rs` | Replace atty usage with IsTerminal |
| `crates/terraphim_agent/src/onboarding/wizard.rs` | Replace atty usage with IsTerminal |
| `crates/terraphim_atomic_client/test_signature/src/main.rs` | Replace atty usage with IsTerminal |
| `crates/terraphim_agent/Cargo.toml` | Remove atty dependency, update feature flag |

---

## Implementation Steps

### Step 1: Update terraphim-session-analyzer
**Files:** `crates/terraphim-session-analyzer/src/main.rs`
**Description:** Replace atty usage with std::io::IsTerminal
**Estimated:** 3 minutes

```rust
// Line 201 - Before:
if cli.no_color || !atty::is(atty::Stream::Stdout) {

// After:
use std::io::IsTerminal;
if cli.no_color || !std::io::stdout().is_terminal() {
```

**Verification Gate:**
- [ ] `cargo check -p terraphim-session-analyzer` passes

---

### Step 2: Update terraphim_agent main.rs
**Files:** `crates/terraphim_agent/src/main.rs`
**Description:** Replace atty usage in interactive mode detection
**Estimated:** 3 minutes

```rust
// Lines 856-862 - Before:
use atty::Stream;
if !atty::is(Stream::Stdout) { ... }
if !atty::is(Stream::Stdin) { ... }

// After:
use std::io::IsTerminal;
if !std::io::stdout().is_terminal() { ... }
if !std::io::stdin().is_terminal() { ... }
```

**Verification Gate:**
- [ ] `cargo check -p terraphim_agent --features repl-interactive` passes

---

### Step 3: Update terraphim_agent wizard.rs
**Files:** `crates/terraphim_agent/src/onboarding/wizard.rs`
**Description:** Replace atty usage in TTY check
**Estimated:** 3 minutes

```rust
// Line 163 - Before:
if !atty::is(atty::Stream::Stdin) {

// After:
use std::io::IsTerminal;
if !std::io::stdin().is_terminal() {
```

**Verification Gate:**
- [ ] `cargo check -p terraphim_agent --features repl-interactive` passes

---

### Step 4: Update test_signature
**Files:** `crates/terraphim_atomic_client/test_signature/src/main.rs`
**Description:** Replace all 4 atty usages with IsTerminal
**Estimated:** 5 minutes

```rust
// Lines 19, 55, 165, 258 - Before:
if !atty::is(atty::Stream::Stdout) { ... }

// After:
use std::io::IsTerminal;
if !std::io::stdout().is_terminal() { ... }
```

**Verification Gate:**
- [ ] `cargo check -p test_signature` passes

---

### Step 5: Remove atty from Cargo.toml
**Files:** `crates/terraphim_agent/Cargo.toml`
**Description:** Remove atty dependency and update feature flag
**Estimated:** 2 minutes

```toml
# Remove this line (line 66):
atty = { version = "0.2", optional = true }

# Update this line (line 17):
# Before:
repl-interactive = ["repl", "dep:atty"]
# After:
repl-interactive = ["repl"]
```

**Verification Gate:**
- [ ] No atty references remain in Cargo.toml
- [ ] `cargo check -p terraphim_agent --features repl-interactive` passes

---

### Step 6: Workspace-wide Check
**Files:** All
**Description:** Verify no atty references remain and workspace compiles
**Estimated:** 10 minutes

```bash
# Verify no atty references remain
grep -r "atty" --include="*.rs" crates/ || echo "No atty references found"
grep -r "atty" --include="Cargo.toml" crates/ || echo "No atty in Cargo.toml"

# Verify workspace compiles
cargo check --workspace
```

**Verification Gate:**
- [ ] No atty references in any .rs files
- [ ] No atty references in any Cargo.toml files
- [ ] `cargo check --workspace` passes

---

### Step 7: Test Suite
**Files:** All
**Description:** Run tests to verify no regressions
**Estimated:** 10 minutes

```bash
# Run tests for affected crates
cargo test -p terraphim-session-analyzer
cargo test -p terraphim_agent --features repl-interactive

# Run workspace tests (excluding known flaky ones)
cargo test --workspace --exclude terraphim_agent 2>&1 | head -50
```

**Verification Gate:**
- [ ] All tests pass
- [ ] No new warnings introduced

---

## Rollback Plan

If issues discovered:
1. `git checkout -- crates/terraphim-session-analyzer/src/main.rs`
2. `git checkout -- crates/terraphim_agent/src/main.rs`
3. `git checkout -- crates/terraphim_agent/src/onboarding/wizard.rs`
4. `git checkout -- crates/terraphim_atomic_client/test_signature/src/main.rs`
5. `git checkout -- crates/terraphim_agent/Cargo.toml`

---

## Dependencies

### Dependency Removal
| Crate | Version | Reason |
|-------|---------|--------|
| atty | 0.2.14 | Unmaintained, replaced with std library |

---

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Execute Step 1: session-analyzer | Pending | Terraphim AI |
| Execute Step 2: agent main.rs | Pending | Terraphim AI |
| Execute Step 3: wizard.rs | Pending | Terraphim AI |
| Execute Step 4: test_signature | Pending | Terraphim AI |
| Execute Step 5: Cargo.toml | Pending | Terraphim AI |
| Execute Step 6: Workspace check | Pending | Terraphim AI |
| Execute Step 7: Tests | Pending | Terraphim AI |

---

## Approval

- [ ] Research document reviewed
- [ ] Implementation plan approved
- [ ] Ready to execute

---

## Quick Reference: Code Transformations

| Location | Before | After |
|----------|--------|-------|
| session-analyzer:201 | `atty::is(atty::Stream::Stdout)` | `std::io::stdout().is_terminal()` |
| agent/main.rs:856-862 | `atty::is(Stream::Stdout)` | `std::io::stdout().is_terminal()` |
| agent/main.rs:856-862 | `atty::is(Stream::Stdin)` | `std::io::stdin().is_terminal()` |
| agent/wizard.rs:163 | `atty::is(atty::Stream::Stdin)` | `std::io::stdin().is_terminal()` |
| test_signature:* | `atty::is(atty::Stream::Stdout)` | `std::io::stdout().is_terminal()` |

All transformations require: `use std::io::IsTerminal;`
