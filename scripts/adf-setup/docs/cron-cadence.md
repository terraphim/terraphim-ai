# ADF Cron Cadence Policy

ROC v1 Step K — Refs terraphim/adf-fleet#39

## Summary

| Tier | Agent(s) | Cadence | Rationale |
|------|----------|---------|-----------|
| Meta | `fleet-meta`, `project-meta` (meta-coordinator) | `*/30 * * * *` | Coordinate; low volume; template default |
| Developer | `implementation-swarm` | `*/20 * * * *` | Higher throughput; picks up ready issues faster |
| Reviewer | `pr-reviewer` | event-driven (no cron) | Mention-dispatched via `DispatchTask::ReviewPr` |
| Planning | `product-development` | `0 2 * * *` | Daily roadmap and feature prioritization |
| Stewardship | `repo-steward` | `15 */6 * * *` | Repository health synthesis (stability + usefulness) |
| Safety | `security-sentinel`, `compliance-watchdog`, `drift-detector`, `test-guardian`, `spec-validator`, `documentation-generator` | unchanged (own schedules) | Retain their existing cadences; not in scope |

## Template status

As of ROC v1 Step K the `scripts/adf-setup/agents/` directory contains:

- `meta-coordinator.toml` — `schedule = "*/30 * * * *"` (project-meta). **Unchanged** (meta tier).
- `pr-reviewer.toml` — no `schedule` field. **Unchanged** (event-driven).
- `product-development.toml` — `schedule = "0 2 * * *"` (daily planning). **New** (planning tier).
- `repo-steward.toml` — `schedule = "15 */6 * * *"` (stewardship synthesis). **New** (growth tier).

`implementation-swarm` is **not yet templated** in `scripts/adf-setup/agents/`. Its cron definition lives in the live per-project `conf.d/*.toml` configs on bigbox.

## Step L rollout action

During Step L (live-config rollout), update every `implementation-swarm` agent block in
`/opt/ai-dark-factory/conf.d/*.toml` on bigbox:

```toml
# Before (hourly):
schedule = "0 * * * *"

# After (every 20 minutes):
schedule = "*/20 * * * *"
```

Validate with `adf --check /opt/ai-dark-factory/conf.d/<project>.toml` after each edit.

## Design reference

`cto-executive-system/plans/adf-rate-of-change-design.md` §Implementation Step 11.
