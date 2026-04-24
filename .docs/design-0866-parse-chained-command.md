# Design & Implementation Plan: Fix parse_chained_command Failing Subcommand Detection

## 1. Summary of Target Behaviour

After implementation, `parse_chained_command(command, exit_code)` will:
- Return the **correct failing subcommand** based on chain operator semantics and exit code
- For `&&` chains with non-zero exit: return the **last** subcommand (the one that actually failed)
- For `||` chains with non-zero exit: return the **last** subcommand (all failed, this was the final attempt)
- For `;` chains with non-zero exit: return the **last** subcommand (cannot disambiguate, document limitation)
- For zero exit code: return the **first** subcommand (chain succeeded, first is the meaningful one)
- Populate `CapturedLearning.failing_subcommand` with the failing subcommand
- Populate `CapturedLearning.full_chain` with the original full command string
- Handle single commands (no chain operator) correctly: return (command, None)

## 2. Key Invariants and Acceptance Criteria

### Invariants
- The function is pure (no side effects)
- The full_chain returned is always the original command string (preserved exactly)
- The failing_subcommand is trimmed of leading/trailing whitespace
- No panics on malformed input (handle edge cases gracefully)

### Acceptance Criteria (from Issue #866)

| # | Criterion | Testable? |
|---|-----------|-----------|
| 1 | `parse_chained_command(command, exit_code)` returns correct failing subcommand for `&&`, `||`, `;` | Yes |
| 2 | `CapturedLearning.failing_subcommand` populated with trimmed failing subcommand | Yes |
| 3 | `CapturedLearning.full_chain` populated with original full command string | Yes |
| 4 | Unit tests cover: `&&` chain, `\|\|` chain, `;` chain, single command | Yes |
| 5 | `learn list` and `learn query` output reflects corrected failing subcommand | Yes (via integration) |

### Test Cases

| Input Command | Exit Code | Expected Failing Subcommand | Expected Full Chain |
|---------------|-----------|----------------------------|---------------------|
| `cargo build && cargo test` | 1 | `cargo test` | `cargo build && cargo test` |
| `cmd_a || cmd_b || cmd_c` | 1 | `cmd_c` | `cmd_a || cmd_b || cmd_c` |
| `cmd_a; cmd_b; cmd_c` | 1 | `cmd_c` | `cmd_a; cmd_b; cmd_c` |
| `git status` | 0 | `git status` | `None` |
| `git status` | 1 | `git status` | `None` |

## 3. High-Level Design and Boundaries

### Solution Concept

The current implementation returns `parts[0]` (first subcommand) for any chained command. The fix applies operator-specific logic:

```
parse_chained_command(command, exit_code):
  if command contains " && ":
    parts = split by " && "
    if exit_code != 0:
      return (parts.last().trim(), Some(command))
    else:
      return (parts[0].trim(), Some(command))

  if command contains " || ":
    parts = split by " || "
    if exit_code != 0:
      return (parts.last().trim(), Some(command))
    else:
      return (parts[0].trim(), Some(command))

  if command contains "; ":
    parts = split by "; "
    if exit_code != 0:
      return (parts.last().trim(), Some(command))
    else:
      return (parts[0].trim(), Some(command))

  return (command.trim(), None)
```

### Boundaries

| Boundary | Inside | Outside |
|----------|---------|---------|
| Changes to `parse_chained_command` | Yes | - |
| Changes to `capture_failed_command` | No | Yes (uses return values) |
| Changes to `CapturedLearning` struct | No | Yes (already has fields) |
| New dependencies | No | Yes (use std only) |

### Complected Areas to Avoid
- Do NOT change how `with_failing_subcommand` works (it's fine)
- Do NOT add per-step exit code tracking (out of scope)
- Do NOT handle nested parentheses or complex shell syntax

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|--------|-------|--------------|
| `crates/terraphim_agent/src/learnings/capture.rs:1074` | Modify | Returns `parts[0]` always | Returns correct subcommand based on operator + exit_code | None |
| `crates/terraphim_agent/src/learnings/capture.rs:1886` | Modify | 2 test cases | 5+ test cases covering all operators | None |

### Specific Code Changes

#### Change 1: `parse_chained_command` function (line 1074)

**Before:**
```rust
fn parse_chained_command(command: &str, _exit_code: i32) -> (String, Option<String>) {
    let chain_operators = [" && ", " || ", "; "];

    for op in &chain_operators {
        if command.contains(op) {
            let parts: Vec<&str> = command.split(op).collect();
            if parts.len() > 1 {
                return (parts[0].trim().to_string(), Some(command.to_string()));
            }
        }
    }
    (command.trim().to_string(), None)
}
```

**After:**
```rust
fn parse_chained_command(command: &str, exit_code: i32) -> (String, Option<String>) {
    // Check for chain operators in order: &&, ||, ;
    // For each operator, apply semantic rules based on exit code

    // Helper to get failing subcommand
    let get_failing = |parts: Vec<&str>| -> String {
        if exit_code != 0 {
            // Non-zero exit: the last executed/submitted subcommand failed
            parts.last().unwrap().trim().to_string()
        } else {
            // Zero exit: chain succeeded, first subcommand is the meaningful one
            parts[0].trim().to_string()
        }
    };

    let chain_operators = [" && ", " || ", "; "];

    for op in &chain_operators {
        if command.contains(op) {
            let parts: Vec<&str> = command.split(op).collect();
            if parts.len() > 1 {
                let failing = get_failing(parts);
                return (failing, Some(command.to_string()));
            }
        }
    }

    // No chain detected
    (command.trim().to_string(), None)
}
```

#### Change 2: Update tests (line 1886)

**Before:**
```rust
#[test]
fn test_parse_chained_command() {
    let (cmd, chain) = parse_chained_command("docker build . && docker run", 1);
    assert_eq!(cmd, "docker build .");
    assert_eq!(chain, Some("docker build . && docker run".to_string()));

    let (cmd2, chain2) = parse_chained_command("git status", 0);
    assert_eq!(cmd2, "git status");
    assert_eq!(chain2, None);
}
```

**After:**
```rust
#[test]
fn test_parse_chained_command() {
    // Test && chain with non-zero exit (cargo test failed)
    let (cmd, chain) = parse_chained_command("cargo build && cargo test", 1);
    assert_eq!(cmd, "cargo test");
    assert_eq!(chain, Some("cargo build && cargo test".to_string()));

    // Test && chain with zero exit (success)
    let (cmd2, chain2) = parse_chained_command("cargo build && cargo test", 0);
    assert_eq!(cmd2, "cargo build");
    assert_eq!(chain2, Some("cargo build && cargo test".to_string()));

    // Test || chain with non-zero exit (all failed, last attempted)
    let (cmd3, chain3) = parse_chained_command("cmd_a || cmd_b || cmd_c", 1);
    assert_eq!(cmd3, "cmd_c");
    assert_eq!(chain3, Some("cmd_a || cmd_b || cmd_c".to_string()));

    // Test ; chain with non-zero exit (cannot disambiguate, return last)
    let (cmd4, chain4) = parse_chained_command("cmd_a; cmd_b;cmd_c", 1);
    assert_eq!(cmd4, "cmd_c");
    assert_eq!(chain4, Some("cmd_a; cmd_b;cmd_c".to_string()));

    // Test single command without chain
    let (cmd5, chain5) = parse_chained_command("git status", 0);
    assert_eq!(cmd5, "git status");
    assert_eq!(chain5, None);

    // Test single command with failure
    let (cmd6, chain6) = parse_chained_command("git status", 1);
    assert_eq!(cmd6, "git status");
    assert_eq!(chain6, None);
}
```

## 5. Step-by-Step Implementation Sequence

1. **Modify `parse_chained_command` function** - Change the logic to use `exit_code` parameter and return correct failing subcommand based on operator semantics. (Deployable: function compiles, old tests may fail)

2. **Update existing unit test** - Fix `test_parse_chained_command` to expect new behaviour and add more test cases. (Deployable: all tests pass)

3. **Run existing test suite** - Verify no regressions in `capture_failed_command` and related functions. (Deployable: full test suite passes)

4. **Verify with `learn list`** - Manual verification that output reflects corrected failing subcommand. (Deployable: feature complete)

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---------------------|-----------|---------------|
| `&&` chain returns last subcommand on failure | Unit | `capture.rs:1886` (`test_parse_chained_command`) |
| `\|\|` chain returns last subcommand on failure | Unit | `capture.rs:1886` (`test_parse_chained_command`) |
| `;` chain returns last subcommand on failure | Unit | `capture.rs:1886` (`test_parse_chained_command`) |
| Single command without chain | Unit | `capture.rs:1886` (`test_parse_chained_command`) |
| `failing_subcommand` populated correctly | Unit | `capture.rs` (new test) |
| `full_chain` populated correctly | Unit | `capture.rs` (new test) |
| `learn list` shows correct command | Integration | Manual verification |

### Test Command
```bash
cargo test -p terraphim_agent parse_chained_command
cargo test -p terraphim_agent capture_failed_command
```

## 7. Risk & Complexity Review

| Risk (from Phase1) | Mitigation in Design | Residual Risk |
|--------------------|---------------------|---------------|
| Incorrect heuristic for `&&` chains | Use `exit_code` to determine: non-zero = last executed subcommand | Assumes exit_code reflects last executed command |
| Whitespace variations break parsing | Use `trim()` on results; document that operators must have spaces | `cmd1&&cmd2` (no spaces) not handled |
| Chain operators inside quotes/strings | Defer to future spec (out of scope) | Nested quotes may parse incorrectly |
| Spec mismatch (Vec vs tuple) | Keep current signature `(String, Option<String>)` as it matches usage | Slight confusion with spec doc |

## 8. Open Questions / Decisions for Human Review

1. **Whitespace handling**: Should we handle `cmd1&&cmd2` (no spaces around `&&`)? Current design assumes spaces.
2. **Exit code 0 behaviour**: For successful chains, should we return first or last subcommand? (Design returns first as most "meaningful")
3. **Multiple same operators**: `cmd1 && cmd2 && cmd3` - will split correctly into 3 parts, returns last on failure. Is this sufficient?
4. **Mixed operators**: `cmd1 && cmd2 || cmd3` - current design will match `&&` first. Acceptable?
5. **Performance**: The function iterates through operators sequentially. With only 3 operators, this is negligible. Agree?

## 9. Design Decision: Why Not Use terraphim-automata for Parsing

### Question Raised
Should we leverage `terraphim-automata` (Aho-Corasick) for parsing chained commands?

### Analysis

| Aspect | terraphim-automata | Simple `split()` |
|--------|---------------------|-----------------|
| Purpose | Match MANY dynamic patterns (KG terms, thesaurus) | Parse 3 static operators (`&&`, `||`, `;`) |
| Setup Required | Build thesaurus, create matcher, handle errors | None (standard library) |
| Performance | Overhead: automaton construction + matching | Single `split()` call (optimal) |
| Readability | Obscures simple intent with abstraction | Clear, obvious intent |
| Maintenance | Dependency on KG/thesaurus availability | Self-contained, no external deps |

### How terraphim-automata IS Used Correctly

The codebase already uses `terraphim-automata` for its intended purpose - entity matching AFTER parsing:

1. `parse_chained_command()` identifies the failing subcommand (simple split)
2. `annotate_with_entities()` at line 982 calls `terraphim_automata::find_matches()` to match the parsed command against the KG thesaurus
3. This is the correct separation of concerns

### Decision

**Do NOT use terraphim-automata for parsing chained commands.** Reasons:
- Only 3 static operators to match (not a pattern-matching problem)
- `split()` is optimal for this use case
- `terraphim-automata` is correctly used downstream for entity annotation
- Keeps the function simple, testable, and fast (< microsecond execution)

### Architectural Principle

> Use `terraphim-automata` for: dynamic pattern matching against large term sets (KG, thesaurus, exit classes)
> Use standard string operations for: fixed, known-delimiter parsing (3 chain operators)

This aligns with existing architecture in:
- `terraphim_orchestrator/src/agent_run_record.rs`: Uses automata for exit class matching (many patterns)
- `capture.rs:833`: Uses automata for entity annotation (many KG terms)
- `parse_chained_command`: Uses `split()` for 3 operators (correct choice)

---

## Quality Checklist

- [x] Design clearly distinguished from research
- [x] All file changes identified with before/after
- [x] Acceptance criteria mapped to tests
- [x] Risks have mitigations
- [x] Questions are specific and actionable
- [x] No code implementation (design only)
