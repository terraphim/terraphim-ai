# Engineering Scorecard

This scorecard tracks ADF reliability and code quality lead measures aligned with Q2 North Star goals.

## Weekly Measures

| Measure | Target | Owner |
|---------|--------|-------|
| Flaky agents fixed per week | >= 1 | ADF team |
| Agents re-decomposed after context rot (this week) | Track (no target yet) | ADF team |
| Test suite green on CI (main branch) | 100% | All |
| Clippy warnings introduced | 0 | All |

## Context Rot Signal Metric

**Definition**: An agent that runs beyond its `context_rot_wall_secs` threshold and is killed with a rot signal rather than a retry.

**How it is logged**: When `poll_wall_timeouts()` detects a rot threshold breach, it emits a `warn!` log line:
```
context rot signal: killing agent without retry — re-decompose the task
```

And posts a Gitea comment on the agent's `gitea_issue` (if configured) with structured re-decomposition guidance.

**Interpretation**: A non-zero count is not inherently bad — it means the system correctly identified oversized tasks. The goal is that each rot signal is followed by a re-scoped re-launch.

## North Star Q2 Reference

- Priority 1: Ranking pipeline regression gate (WIG-1)
- Priority 2: 5+ agents running reliably overnight by 2026-06-15 (ADF stabilisation)
- Priority 3: Tauri desktop parity with server API (WIG-3)
- Priority 4: CI/CD pipeline coverage (WIG-4)
