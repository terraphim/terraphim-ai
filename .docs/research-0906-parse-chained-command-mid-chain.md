# Research Document: Fix parse_chained_command mid-chain failure detection (#906)

**Status**: Draft
**Author**: opencode
**Date**: 2026-04-25
**Supersedes**: `.docs/research-0866-parse-chained-command.md` (previous fix was incomplete)

## Executive Summary

Commit `54e6d4b4` (Refs #866) fixed `parse_chained_command` to use the exit code parameter, but introduced a new inaccuracy: for `&&` chains with non-zero exit, it returns `parts.last()` (the **last** subcommand). In reality, when a `&&` chain fails at an earlier command, subsequent commands never execute — so the last subcommand is never the failing one unless it happens to be the second-to-last. Issue #906 was filed after a structural review caught this. The fundamental problem is that **from a single exit code alone, we cannot determine which subcommand in a `&&` chain failed** — only that some prefix of the chain succeeded and one command returned non-zero.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Incorrect learnings poison the knowledge base; users get wrong suggestions |
| Leverages strengths? | Yes | Pure function, well-tested, no external deps |
| Meets real need? | Yes | Structural review finding (#906) confirms production impact |

**Proceed**: Yes (3/3)

## Problem Statement

### Description

`parse_chained_command` in `capture.rs:1080` returns the **last** subcommand for `&&` chains with non-zero exit. This is incorrect for mid-chain failures because shell `&&` short-circuits — if `cmd1 && cmd2 && cmd3` fails at `cmd1`, then `cmd2` and `cmd3` never execute.

### Shell Semantics (verified)

```
cmd1 && cmd2 && cmd3   exit=1   => cmd1 failed, cmd2/cmd3 never ran
cmd1 && cmd2 && cmd3   exit=0   => all succeeded
cmd1 || cmd2 || cmd3   exit=1   => all failed, cmd3 was last attempted
cmd1 || cmd2 || cmd3   exit=0   => at least one succeeded (short-circuits on success)
cmd1; cmd2; cmd3       exit=1   => all ran, cannot tell which failed
```

### Impact

- Learning capture records the **wrong** command as the failure
- `learn query` returns irrelevant recovery suggestions
- Users lose trust in the learning system

### Current Behaviour (from commit 54e6d4b4)

```rust
let get_failing = |parts: &[&str]| -> String {
    if exit_code != 0 {
        parts.last().unwrap().trim().to_string()  // WRONG for && mid-chain
    } else {
        parts[0].trim().to_string()
    }
};
```

For `"cargo build && cargo test && cargo lint"` with exit=1:
- Returns `"cargo lint"` (parts.last())
- If `cargo build` failed, `cargo test` and `cargo lint` never ran
- The correct answer is **unknown** from exit code alone

### Success Criteria

1. The function does not return a provably incorrect answer
2. The function documents its limitations clearly
3. All existing tests continue to pass
4. New tests cover the mid-chain failure case

## Current State Analysis

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| `parse_chained_command` | `capture.rs:1080` | Parse chain, identify failing subcommand |
| `capture_failed_command` | `capture.rs:933` | Main entry point, calls parser at line 950 |
| `with_failing_subcommand` | `capture.rs:250` | Sets failing_subcommand and full_chain on CapturedLearning |
| Hook caller | `learnings/hook.rs:75` | `capture_from_hook` passes command + exit_code |
| CLI caller | `main.rs:2528` | `learn capture` subcommand passes command + exit_code |

### Call Flow

```
Shell command fails (exit != 0)
  -> hook.rs::capture_from_hook(input) or main.rs CLI
    -> capture_failed_command(command, error, exit_code, config)
      -> parse_chained_command(command, exit_code)
        -> returns (actual_command, Option<full_chain>)
      -> CapturedLearning::new(actual_command, ...)
      -> if full_chain: learning.with_failing_subcommand(actual_command, full_chain)
      -> stored to disk
```

### Data Flow Constraint

The **only** information available to `parse_chained_command` is:
1. The full command string (e.g., `"cmd1 && cmd2 && cmd3"`)
2. The exit code of the entire chain (e.g., `1`)

There is **no per-step exit code** information. This is a fundamental constraint.

## Constraints

| Constraint | Source | Implication |
|------------|--------|-------------|
| No per-step exit codes | Shell execution model | Cannot determine exact failing subcommand for `&&` chains |
| Pure function (no I/O) | Design | Cannot re-execute commands to find which failed |
| No new dependencies | Project rule | Standard library only |
| `CapturedLearning` struct stable | Existing storage format | Cannot add new fields without migration |
| British English, no emoji | Project style | Comments and docs |

## Multiple Interpretations Considered

### Interpretation A: Return all subcommands for ambiguous cases

For `&&` chains with non-zero exit, return **all** subcommands up to and including the unknown failing one. The caller would need to handle `Vec<String>`.

| Aspect | Assessment |
|--------|------------|
| Pro | Honest about uncertainty |
| Con | Changes function signature; callers need updating; storage format unclear |
| Verdict | Rejected — too much churn for P2 |

### Interpretation B: Return first subcommand for `&&` chains (conservative)

For `&&` chains with non-zero exit, return `parts[0]` (the first subcommand). This is the minimum guaranteed to have executed.

| Aspect | Assessment |
|--------|------------|
| Pro | Always correct (first command definitely ran) |
| Con | May be wrong if first succeeded but second failed |
| Verdict | Considered but wrong direction — swaps one wrong answer for another |

### Interpretation C: Document limitation, return best-effort + flag uncertainty

Keep returning a single best-effort subcommand but add a comment/field indicating uncertainty. For `&&` chains, return first subcommand as "candidate" with documented limitation.

| Aspect | Assessment |
|--------|------------|
| Pro | Honest, minimal code change, no signature change |
| Con | Still imperfect for `&&` chains |
| Verdict | **Recommended** — see analysis below |

### Interpretation D: Add a `ChainAnalysis` enum to the return type

Return structured metadata: `enum ChainAnalysis { Single(String), Chain { candidate: String, full: String, confidence: Confidence } }`. Let callers decide.

| Aspect | Assessment |
|--------|------------|
| Pro | Most informative, extensible |
| Con | Signature change, caller updates, storage format change |
| Verdict | Over-engineered for P2 issue |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why Vital | Evidence |
|------------|-----------|----------|
| Must not return provably wrong answer | Core correctness | #906 structural review finding |
| No signature change on `parse_chained_command` | Minimise blast radius | Called from 2 locations + tests |
| Must document limitation | Future developers | Previous fix (#866) missed this |

### Eliminated from Scope

| Eliminated | Why |
|------------|-----|
| Per-step exit code tracking | Requires shell instrumentation, out of scope |
| `terraphim-automata` for parsing | Only 3 static operators, split() is optimal |
| `ChainAnalysis` enum return type | Over-engineering for P2 |
| Handling mixed operators (`cmd1 && cmd2 || cmd3`) | Left-to-right matching is good enough |
| Quoted strings containing `&&` | Edge case, defer |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Any single-command return is sometimes wrong | High | Medium | Document clearly; accept as heuristic |
| Tests from #866 hardcode `parts.last()` behaviour | High | Low | Update tests |
| Callers downstream rely on current behaviour | Low | Medium | Check usages; return type unchanged |

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Shell `&&` short-circuits on first failure | POSIX spec + verified with bash | N/A (fact) | Yes |
| `||` short-circuits on first success | POSIX spec + verified with bash | N/A (fact) | Yes |
| Exit code from chain = exit code of last executed command | POSIX spec | If wrong, our heuristic changes | Yes |
| No nested chains in practice | Typical CLI usage | Parenthesised commands would mis-parse | No (assumption) |

### Fundamental Insight

**For `&&` chains with non-zero exit:**
- We know: some prefix of commands succeeded, then one failed
- We do NOT know: which one failed (could be any from 1st to last)
- The exit code is from the failing command, not from the last listed one
- `parts.last()` is guaranteed WRONG unless the last command failed (which we can't know)
- `parts[0]` is guaranteed to have RUN but may have succeeded

**For `||` chains with non-zero exit:**
- ALL commands executed and ALL failed
- `parts.last()` is correct (it was the last attempted)

**For `;` chains with non-zero exit:**
- ALL commands executed, one or more failed
- We cannot determine which one(s)
- `parts.last()` is as good as any guess

## Recommendations

### Proceed: Yes

This is a real correctness bug (P2) that should be fixed before it poisons more learning entries.

### Recommended Approach

For `&&` chains specifically, the fix should:
1. Return `parts[0]` for non-zero exit (the command that definitely executed)
2. Add a prominent code comment explaining the limitation
3. Consider whether `full_chain` alone is sufficient for the learning entry (it preserves all context)

**Rationale**: `parts[0]` is always at least _plausible_ — it definitely ran. `parts.last()` is often _impossible_ — it may never have executed. Returning the first is the conservative, honest choice.

### Alternative: Return full chain as the "failing subcommand"

Instead of picking one subcommand, return the full chain as the failing command and set `full_chain` to `Some(chain)`. This avoids picking a wrong subcommand entirely. The caller at line 977-980 already guards against `chain == actual_command`, so we'd need to adjust the logic.

## Next Steps

1. Human approves research findings and chooses approach
2. Phase 2: Design the specific code changes
3. Phase 3: Implement with tests
4. Phase 4: Verify
5. Phase 5: Validate

## Appendix: Shell Behaviour Verification

```bash
$ bash -c 'echo "start" && false && echo "end"'
start
exit: 1
# cmd1 ran, cmd2 failed, cmd3 never ran

$ bash -c 'true && false'
exit: 1
# cmd1 ran and succeeded, cmd2 ran and failed

$ bash -c 'false || false'
exit: 1
# Both ran, both failed

$ bash -c 'false || true'
exit: 0
# cmd1 failed, cmd2 succeeded, short-circuit
```
