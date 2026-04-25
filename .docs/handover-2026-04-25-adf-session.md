# Session Handover -- 2026-04-25 (ADF Stabilisation Session)

**Session ended**: 12:56 BST (11:56 UTC)
**Branch**: main
**Both remotes**: Gitea + GitHub at `4c42c210`

---

## Session Focus

ADF (AI Dark Factory) stabilisation and product vision restoration. Three structural gaps from the original `cto-executive-system` vision were closed. Four classifier/routing bugs were fixed. The product-owner agent was redesigned with Compound-RICE scoring, Themis persona, and Gitea visibility.

---

## What Was Shipped

### Bug Fixes

**ExitClassifier false-positive** (`agent_run_record.rs`):
- `upstream-synchronizer`, `repo-steward`, `spec-validator` were classified as `resource_exhaustion`/`timeout` with `exit_code=0` because pattern matches (e.g. "OOM" in an infra health report) overrode a clean exit.
- Fix: `exit_code=0` is now authoritative. Pattern matches preserved in `matched_patterns` for observability.
- Three regression tests added. Binary rebuilt and deployed to bigbox.

**Review tier priority regression** (`review_tier.md`):
- `priority:: 60` should be `priority:: 40`. Originally fixed (commit `73849826`) to prevent implementation agents routing to the cheaper haiku model via review-tier false match. Silently regressed.
- Fixed. Rust tests updated to assert `priority == 40`.

**meta-coordinator duplicate fires / runaway sessions**:
- `--max-turns 3` added to both claude invocations in the meta-coordinator task.
- `max_cpu_seconds` changed from 7200 to 300.

**swap exhaustion false alerts**:
- `vm.swappiness=1` set on bigbox (persisted). Kernel was aggressively filling 4 GiB swap despite 110 GiB RAM available.
- `upstream-synchronizer` memory check updated to only flag CRITICAL when swap AND available RAM <20 GiB.
- Issue #888 closed.

### New Agents

**runtime-guardian** -- renamed from `upstream-synchronizer` (which was a misnomer). Same infra-health function: disk, Docker, memory, runners, target/ sizes, cargo outdated.

**upstream-synchronizer** (new, real fork-sync) -- monitors `/home/alex/projects/terraphim/gitea` vs `go-gitea/gitea.git`. Schedule: `30 1 * * *`. Creates Gitea issue only if >50 commits behind AND security-relevant commits found. Upstream remote added to gitea fork.

**meta-learning / Mneme** (new) -- fleet pattern synthesis. Schedule: `0 11 * * *` (after overnight window). Parses journalctl exit stats (160 records/day), reads infra-health reports, counts Theme-IDs, synthesises with sonnet. Writes `Fleet-Health-YYYYMMDD-Mneme` wiki page. Creates alert issue only for P0/P1 patterns. **First run: 2026-04-26 11:00 UTC.**

### Product Strategy -- Themis

**New persona**: `data/personas/themis.toml` -- Product Strategist, balance scales symbol, "Weigh, decide, ship."

**product-owner redesigned** with three-layer cycle:
1. **5/25 Rule** (Essentialism): 25 issues → vital 5 → "Avoid At All Cost" (20 named explicitly)
2. **Compound-RICE**: `(Reach × Impact × Confidence × Synergy) / (Effort × Maintenance)`. Synergy >2.0 = compound opportunity + 4DX lead measure. Bands: critical≥30, high≥15, medium≥7, low<7
3. **WIG alignment + mini-UAT**: maps to WIG from `progress.md`; Gherkin acceptance block + marketing hint in every created issue
4. **Gitea comment** (Option A): scoring summary posted as comment on top ready issue after each cycle

**product-development** persona corrected Lux → Ferrox. Fallback added (was failing open).

**Gitea wiki** `ADF-Product-Strategy-Themis-2026-04-25` created.

### Documentation

- `.docs/adf-architecture.md` -- 19 agents, updated Mermaid diagram, full Themis cycle, ExitClassifier history, corrected persona registry
- `.docs/adf-meet-your-agents.md` -- 8 personas active (Themis + Mneme added), full appearance descriptions, "How the Product Strategy Works" flow

### Issue Created

**#912** -- `feat(adf): implement compound-reviewer agent (Mneme feeds auto-compound loop)`
- **Blocked** until 3+ Fleet-Health Mneme pages exist (~2026-04-28 earliest)
- Compound-RICE: 189 (critical). Synergy=3.0 (builds on Mneme + existing compound prompts)
- Scope: reads Fleet-Health → mini-PRD → ≤5 tasks → implements → PR + wiki

---

## Deferred Validations (check tomorrow morning)

| Time UTC | Check | Command |
|----------|-------|---------|
| 2026-04-26 00:15 | runtime-guardian first fire | `journalctl \| grep "cron schedule fired agent=runtime-guardian"` |
| 2026-04-26 00:55 | Themis Compound-RICE first cycle | Open top Gitea ready issue -- Themis comment should appear |
| 2026-04-26 01:30 | upstream-synchronizer fork-sync | `journalctl \| grep "upstream-synchronizer"` + divergence count |
| 2026-04-26 11:00 | Mneme first fleet synthesis | `gtr wiki-get --name "Fleet-Health-20260426-Mneme"` |

---

## What Remains Open

| Issue | Status | Next action |
|-------|--------|-------------|
| #912 compound-reviewer | Blocked | Unblock after 3+ Mneme pages (from 2026-04-28) |
| #893 CI runners STOPPED | Pre-existing | Restart GitHub Actions runners on bigbox |
| #902 target/ 483 GB | Pre-existing | `cargo clean` on bigbox |
| Themis appearance description for image model | Missing | Add to `data/personas/themis.toml` following `adf-meet-your-agents.md` pattern |

---

## Key System State

| Component | State |
|-----------|-------|
| ADF binary | `/usr/local/bin/adf` -- built 2026-04-25, new ExitClassifier |
| vm.swappiness | 1 (persisted in `/etc/sysctl.d/99-bigbox-tuning.conf`) |
| gitea fork upstream remote | `https://github.com/go-gitea/gitea.git` (added 2026-04-25) |
| review_tier priority | 40 (below implementation_tier=50) |
| Orchestrator | active, PID varies, no ERROR lines |
| Active agents | 19 |
