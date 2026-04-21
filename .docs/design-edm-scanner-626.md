# Design & Implementation Plan: EDM Scanner -- terraphim_negative_contribution Crate (#626)

## 1. Summary of Target Behavior

A new workspace crate `terraphim_negative_contribution` that statically detects Explicit Deferral Markers (EDMs) -- `todo!()`, `unimplemented!()`, and panic stubs -- in Rust source files. The scanner uses Aho-Corasick for O(n) multi-pattern matching, excludes non-production files, respects inline suppressions, and returns findings as `Vec<ReviewFinding>` for direct consumption by the compound review pipeline.

## 2. Key Invariants and Acceptance Criteria

### Invariants
- Aho-Corasick automaton built once at construction, never rebuilt
- `NegativeContributionScanner` is `Clone` + `Send + Sync` (Arc-safe, no interior mutability)
- Tier 1 patterns only -- no Tier 2 (`// FIXME`, `// HACK`)
- Single-pass byte scanning for all patterns simultaneously

### Acceptance Criteria (from issue #626)
- [ ] `todo!()` detected as High severity finding
- [ ] `unimplemented!()` detected as High severity finding
- [ ] `panic!("not implemented")` and `panic!("TODO")` detected as High severity
- [ ] `todo!("` and `unimplemented!("` (with message) detected
- [ ] `// terraphim: allow(stub)` suppresses finding on same line
- [ ] Files under `/tests/`, `/examples/`, `/benches/` excluded by path
- [ ] Files named `build.rs` excluded by path
- [ ] Files with suffix `_test.rs` excluded by path
- [ ] Files containing `#[test]` or `#[cfg(test)]` in full content excluded
- [ ] `// FIXME` and `// HACK` NOT flagged (Tier 2, out of scope)
- [ ] `cargo build -p terraphim_negative_contribution` succeeds
- [ ] `cargo test -p terraphim_negative_contribution` passes
- [ ] `cargo clippy -p terraphim_negative_contribution -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## 3. High-Level Design and Boundaries

```
┌─────────────────────────────────────────────────┐
│         terraphim_negative_contribution          │
│                                                   │
│  ┌──────────┐   ┌───────────┐   ┌──────────────┐│
│  │patterns  │──>│ scanner   │──>│ReviewFinding ││
│  │(Tier 1)  │   │(AC-based) │   │(from types)  ││
│  └──────────┘   └───────────┘   └──────────────┘│
│       │              │                           │
│       │         ┌────┴─────┐                     │
│       │         │exclusion │                     │
│       │         │logic     │                     │
│       │         └──────────┘                     │
└─────────────────────────────────────────────────┘
        │                │
        v                v
  aho-corasick     terraphim_types
   (1.0.2)       (ReviewFinding etc.)
```

### Module Layout

| Module | Responsibility |
|--------|---------------|
| `src/lib.rs` | Crate root, re-exports, `NegativeContributionScanner` struct |
| `src/patterns.rs` | Tier 1 pattern definitions, severity mapping |
| `src/scanner.rs` | Aho-Corasick scanning, byte-offset-to-line conversion, suppression check |
| `src/exclusion.rs` | `is_non_production()` path and content exclusion logic |

### Boundaries
- **Inside**: Pattern definitions, scanning logic, exclusion logic, finding construction
- **Outside**: File I/O (caller provides content), diff parsing (future), LSP integration (future)
- **No dependency on `terraphim_automata`**: Our patterns are compile-time constants, not user thesauruses

## 4. File/Module-Level Change Plan

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `crates/terraphim_negative_contribution/Cargo.toml` | Create | - | Crate manifest with aho-corasick, terraphim_types deps | - |
| `crates/terraphim_negative_contribution/src/lib.rs` | Create | - | Crate root, re-exports, `NegativeContributionScanner` struct | patterns, scanner, exclusion |
| `crates/terraphim_negative_contribution/src/patterns.rs` | Create | - | `NEGATIVE_PATTERNS_TIER1`, `PatternInfo` with severity | - |
| `crates/terraphim_negative_contribution/src/scanner.rs` | Create | - | `scan_content()` method, byte-offset-to-line, suppression | patterns, terraphim_types |
| `crates/terraphim_negative_contribution/src/exclusion.rs` | Create | - | `is_non_production(path, content)` | - |

No existing files modified. Pure addition.

## 5. Step-by-Step Implementation Sequence

### Step 1: Cargo.toml + empty lib.rs (~5 min)
- Create crate directory
- `Cargo.toml` with `aho-corasick = "1.0.2"`, `terraphim_types` path dep
- Empty `lib.rs` with crate-level docs
- **Deployable**: `cargo check -p terraphim_negative_contribution` passes

### Step 2: patterns.rs (~10 min)
- Define `NEGATIVE_PATTERNS_TIER1: &[&str]` with the 6 patterns
- Define `PatternInfo` struct mapping pattern index to severity and description
- Unit test: verify pattern count and content
- **Deployable**: compiles, pattern unit test passes

### Step 3: exclusion.rs (~15 min)
- Implement `is_non_production(path: &str, full_content: &str) -> bool`
- Path checks: `/tests/`, `/examples/`, `/benches/`, `build.rs`, `_test.rs` suffix
- Content checks: `#[test]` or `#[cfg(test)]` anywhere in file
- Unit tests for each exclusion rule (positive and negative)
- **Deployable**: exclusion tests pass

### Step 4: scanner.rs (~30 min)
- `NegativeContributionScanner` struct with `AhoCorasick` field + `PatternInfo` vec
- `new()` constructor: build automaton from `NEGATIVE_PATTERNS_TIER1`
- `scan_file(path, content) -> Vec<ReviewFinding>`:
  1. Check `is_non_production()` -> return empty if excluded
  2. Build line offset map (Vec<usize> of `\n` positions) for byte-to-line conversion
  3. Run `automaton.find_iter(content)` over byte slices
  4. For each match: get line number, check suppression, construct `ReviewFinding`
- `scan_files(files: &[(String, String)]) -> Vec<ReviewFinding>`: iterate + flatten
- Unit tests for: basic detection, suppression, exclusion, multi-pattern, zero findings
- **Deployable**: all scanner tests pass

### Step 5: lib.rs integration + derive (~10 min)
- `NegativeContributionScanner` derives `Clone` (AhoCorasick is Clone)
- Re-export public API
- Wire everything together
- **Deployable**: full crate compiles and tests pass

### Step 6: Clippy + fmt + final verification (~5 min)
- `cargo clippy -p terraphim_negative_contribution -- -D warnings`
- `cargo fmt -p terraphim_negative_contribution -- --check`
- `cargo test -p terraphim_negative_contribution`
- **Deployable**: all quality gates green

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---------------------|-----------|---------------|
| `todo!()` detected | Unit | `scanner.rs` `test_detect_todo` |
| `unimplemented!()` detected | Unit | `scanner.rs` `test_detect_unimplemented` |
| `panic!("not implemented")` detected | Unit | `scanner.rs` `test_detect_panic_not_implemented` |
| `panic!("TODO")` detected | Unit | `scanner.rs` `test_detect_panic_todo` |
| `todo!("with message")` detected | Unit | `scanner.rs` `test_detect_todo_with_message` |
| `// terraphim: allow(stub)` suppresses | Unit | `scanner.rs` `test_suppression` |
| `/tests/` path excluded | Unit | `exclusion.rs` `test_tests_dir_excluded` |
| `/examples/` path excluded | Unit | `exclusion.rs` `test_examples_dir_excluded` |
| `/benches/` path excluded | Unit | `exclusion.rs` `test_benches_dir_excluded` |
| `build.rs` excluded | Unit | `exclusion.rs` `test_build_rs_excluded` |
| `_test.rs` suffix excluded | Unit | `exclusion.rs` `test_test_suffix_excluded` |
| `#[test]` in content excluded | Unit | `exclusion.rs` `test_inline_test_excluded` |
| `#[cfg(test)]` in content excluded | Unit | `exclusion.rs` `test_cfg_test_excluded` |
| `// FIXME` NOT flagged | Unit | `scanner.rs` `test_fixme_not_flagged` |
| `// HACK` NOT flagged | Unit | `scanner.rs` `test_hack_not_flagged` |
| Multiple patterns in one file | Unit | `scanner.rs` `test_multiple_patterns` |
| Clean file returns empty | Unit | `scanner.rs` `test_clean_file` |

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| Aho-Corasick version conflict | Pin `aho-corasick = "1.0.2"` matching terraphim_automata | None |
| False positive in doc comments | Patterns include `()` which don't appear in doc comments | Very low |
| `Clone` derive on AhoCorasick | AhoCorasick implements Clone natively | None |
| Line number off-by-one | Use 0-based `\n` counting, convert to 1-indexed | Test thoroughly |

## 8. Open Questions / Decisions for Human Reviewer

1. **`diffy` dependency**: I recommend NOT adding `diffy` for Step 1. We scan full file content, not diffs. Diff-based scanning can be added in Step 3 (compound review wiring) when needed.

2. **Scanner return type**: I recommend returning `Vec<ReviewFinding>` directly rather than a custom signal type. This gives immediate compatibility with the compound review pipeline.

3. **`is_non_production` visibility**: I recommend making it `pub` so the orchestrator can pre-filter file lists before calling the scanner.

4. **Confidence value**: I recommend `0.95` for exact macro matches since they are deterministic.
