# Research Document: Fix parse_chained_command to Identify Failing Subcommand

## 1. Problem Restatement and Scope

### Problem in My Own Words
The `parse_chained_command` function in `crates/terraphim_agent/src/learnings/capture.rs:1074` is responsible for parsing chained commands (using `&&`, `||`, or `;`) and identifying which subcommand failed. Currently, it always returns the **first** subcommand as the failure, regardless of the exit code or chain operator semantics. This results in incorrect learning entries being stored.

### User-Visible Behaviour/Outcomes
- When a chained command fails (e.g., `cargo build && cargo test` fails during `cargo test`), the learning system incorrectly records `cargo build` as the failing command
- `learn query` returns wrong recovery candidates because the stored command is incorrect
- `CapturedLearning.failing_subcommand` and `full_chain` fields exist but are never populated correctly

### IN Scope
- Fix `parse_chained_command` to return the correct failing subcommand based on:
  - Exit code
  - Chain operator semantics (`&&`, `||`, `;`)
- Populate `CapturedLearning.failing_subcommand` correctly
- Populate `CapturedLearning.full_chain` correctly
- Add unit tests for the parser

### OUT of Scope
- Per-step exit-code instrumentation (shell-level changes required)
- Renaming or restructuring `CapturedLearning`
- Changes to learning storage or retrieval logic

## 2. User & Business Outcomes

| Outcome | Description |
|---------|-------------|
| Correct failing subcommand recorded | Learning entries accurately reflect which subcommand failed |
| `learn query` returns relevant results | Users get useful recovery suggestions for the actual failing command |
| `learn list` output is accurate | Debugging and review shows correct information |
| No regression in single commands | Non-chained commands continue to work correctly |

## 3. System Elements and Dependencies

| Element | Location | Role/Responsibility | Dependencies |
|---------|----------|---------------------|--------------|
| `parse_chained_command` | `crates/terraphim_agent/src/learnings/capture.rs:1074` | Parse chained commands, identify failing subcommand | None (pure function) |
| `CapturedLearning` | `crates/terraphim_agent/src/learnings/capture.rs:195` | Store captured learning with `failing_subcommand` and `full_chain` fields | None |
| `CapturedLearning::with_failing_subcommand` | `crates/terraphim_agent/src/learnings/capture.rs:250` | Builder method to set failing subcommand and full chain | None |
| `capture_failed_command` | `crates/terraphim_agent/src/learnings/capture.rs:920` | Main capture function that calls `parse_chained_command` | Calls parser, creates `CapturedLearning` |
| Learning storage | `crates/terraphim_agent/src/learnings/capture.rs:974-981` | Stores learning with chain info | Uses `with_failing_subcommand` |

### Call Flow
```
capture_failed_command()
  └─> parse_chained_command(command, exit_code)
  └─> CapturedLearning::new(actual_command, ...)
  └─> CapturedLearning::with_failing_subcommand(actual_command, chain)
```

## 4. Constraints and Their Implications

| Constraint | Why It Matters | How It Shapes Solution |
|------------|---------------|----------------------|
| Single PR, <200 LOC | Maintainability, reviewability | Keep changes focused to `capture.rs` only |
| No new dependencies | Avoid bloat, minimize risk | Use only standard string operations |
| No mocks in tests | Test realism | Write unit tests with concrete inputs |
| British English, no emoji | Project style guide | Documentation and comments follow convention |
| Exit code available but per-step codes not | Current limitation | Use conservative heuristics for `&&` and `||` chains |
| Unused `_exit_code` parameter exists | Function signature already accepts it | Simply use the parameter instead of ignoring it |

## 5. Risks, Unknowns, and Assumptions

### UNKNOWNS
- **How does the shell report exit codes for chained commands?** Assumption: for `cmd1 && cmd2`, if `cmd1` fails, exit code is from `cmd1`; if `cmd1` succeeds but `cmd2` fails, exit code is from `cmd2`.
- **Are there edge cases with whitespace around operators?** E.g., `cmd1&&cmd2` vs `cmd1 && cmd2`.

### ASSUMPTIONS
- The exit code passed to `parse_chained_command` is the exit code of the entire chained command
- For `&&` chains: a non-zero exit means the last executed subcommand failed
- For `||` chains: a non-zero exit means all subcommands failed; record the last one
- For `;` chains: cannot disambiguate without per-step exit codes; record the last subcommand

### RISKS

| Risk | Type | De-risking |
|------|------|------------|
| Incorrect heuristic for `&&` chains | Technical | Write tests for known cases; document limitation |
| Whitespace variations break parsing | Technical | Test with varied whitespace; consider normalising |
| Chain operators inside quotes/strings | Technical | Consider but defer (out of scope per issue) |

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity
- Three different chain operators with different semantics
- No per-step exit codes available
- Need to handle edge cases (whitespace, quoting)

### Simplification Strategies
1. **Conservative approach**: For all chain types with non-zero exit, return the **last** subcommand (simplest and reasonably accurate for `&&` and `||`)
2. **Operator-specific logic**: Implement different behaviour per operator (as per issue specification)
3. **Clear documentation**: Comment the limitations for `;` chains

**Recommended**: Option 2 (operator-specific) as specified in the issue, since the behaviour differs:
- `&&`: Last executed = failing (correct)
- `||`: Last attempted = failing (all failed)
- `;`: Last subcommand (cannot determine, document limitation)

## 7. Questions for Human Reviewer

1. Should we handle `cmd1&&cmd2` (no spaces around operator) or only `cmd1 && cmd2`?
2. For `&&` chains: if exit code is 0, should we return the first command or last executed?
3. Should we normalise the failing subcommand (trim, normalize spaces) before storing?
4. Is there a maximum number of subcommands we should support in a chain?
5. Should we log a warning when we cannot determine the failing subcommand accurately?
6. The spec (line 307) shows `parse_chained_command` returning `Vec<String>`, but implementation returns `(String, Option<String>)`. Should we align with spec or keep current signature?
7. Are there other chain operators to consider (`|`, `&>`)?
8. Should the full_chain include or exclude trailing/leading whitespace?
9. Is it worth adding a test for nested chains like `cmd1 && (cmd2 || cmd3)`?
10. The issue says `<200 LOC` but the rules say `<500 LOC` - which limit should we follow?

## Quality Checklist

- [x] Problem clearly distinguished from solutions
- [x] All affected system elements identified
- [x] Constraints have clear implications
- [x] Every assumption marked as such
- [x] Risks have de-risking suggestions
- [x] Questions are specific and actionable
