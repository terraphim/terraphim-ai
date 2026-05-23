# Handover: ADF Self-Healing Phase 3 -- agents dispatched, awaiting PRs

**Date**: 2026-05-23 (afternoon, ~14:20 BST)
**Author**: Claude (this session)
**Successor entry point**: Re-engage via `/loop 30m <prompt>` (see §Re-entry) or fresh session + read this doc.
**Research**: `.docs/research-adf-self-healing-2026-05-23.md`
**Design**: `.docs/design-adf-self-healing-2026-05-23.md` + addendum `.docs/design-adf-self-healing-2026-05-23-probe-addendum.md`

## 1. What landed in `main` this session

| Commit | What |
|---|---|
| `a726f5998` | docs(adf): research + design + handover for self-healing rollout |
| `500ac933d` | chore(orchestrator): satisfy clippy needless_borrow lint |
| `11e5cbbed` | feat(adf): add decision_tier + gpt-5.5 + M2.7-highspeed + pi-rust routes |
| `e4b986f18` | fix(adf): quarantine zai-coding-plan routes (mistake -- reverted next commit) |
| `1813416c5` | fix(adf): swap Z.AI from opencode to pi-rust; add probe addendum |
| `ff542a4f4` | feat(orchestrator): per-(cli,provider,model) probe + truncated-stream detection |
| `02e7ecd66` | fix(adf): use canonical provider names in routes (drop pi-rust-* prefix) |

Net effect:
- **Step 0**: per-(CLI, provider, model) probe + truncated-stream detection (Z.AI step_start-only now classified Error). 11 new tests.
- **Step 1**: Z.AI investigation closed; routes use pi-rust for Z.AI (opencode 1.14.48 broken upstream).
- **Steps 3+4**: decision_tier (priority 65) + gpt-5.5 + MiniMax-M2.7-highspeed + pi-rust routes; canonical provider names so C1 allow-list accepts them.

## 2. In-flight ADF agent dispatches (as of session close)

| Issue | Agent | PID (bigbox) | Tier route | Status |
|---|---|---|---|---|
| #1804 P0 Debug redaction | compliance-watchdog | 3209756/3210509 (claude) | implementation_tier -> anthropic/sonnet | running |
| #1817 Operational guardrails | implementation-swarm | 3210115 (claude) | implementation_tier -> anthropic/sonnet | running |
| #1805 P1 merge-coordinator Rust crate | implementation-swarm | (queued) | -- | blocked by impl-swarm one-at-a-time guard; will auto-redispatch after #1817 completes |

Wall-clock cap on these agents: typically 1800-7200 s per the agent definitions in `/opt/ai-dark-factory/conf.d/terraphim.toml`. Expected completion 14:30-15:30 BST.

## 3. Bigbox state

| Property | Value |
|---|---|
| `adf-orchestrator.service` | active (restarted at 14:13 BST with 37 agent definitions) |
| Orchestrator binary | `/usr/local/bin/adf` at SHA `94ebd7517` -- needs rebuild from current main eventually (Step 9 deploy) |
| Working dir | `/data/projects/terraphim/terraphim-ai` (CORRUPTED, see #1818) but worktrees work |
| Fresh build clone | `/data/projects/terraphim/terraphim-ai-fresh` (clean, at `02e7ecd66`) |
| Implementation-swarm | restored to conf.d at 14:13 BST. `cli_tool=opencode primary, fallback_provider=/home/alex/.local/bin/pi-rust` |
| Memory | 3.1G/90G high, 406 tasks (down from 84G/1086 before restart) |
| Anthropic | healthy today (probe verified) |
| Z.AI via opencode | broken (#1819 vendor) |
| Z.AI via pi-rust | healthy (verified) |

## 4. Re-entry procedure

### Option A: /loop polling (user-driven)

```
/loop 30m check bigbox journal since last tick for landed PRs on #1804 #1805 #1817; run disciplined-verification on any merged work; if implementation-swarm has freed up, redispatch #1805 via @adf:implementation-swarm
```

### Option B: manual

```bash
# Check bigbox journal for any agent completion
ssh bigbox 'sudo journalctl -u adf-orchestrator --since "2 hours ago" | grep -iE "exit_class|agent exit|PR created|PR opened|Fixes #18"'

# List open PRs on Gitea since session start (2026-05-23 ~11:30 BST)
gtr list-pulls --owner terraphim --repo terraphim-ai --state open | head -10

# For each landed PR, run:
# 1. disciplined-verification skill against the design at .docs/design-adf-self-healing-2026-05-23.md
# 2. If verification passes, merge via gtr merge-pull
# 3. If implementation-swarm is now idle: @adf:implementation-swarm on #1805
```

### Verification checklist per landed PR

- [ ] PR references `Fixes #N` where N is the issue (#1804, #1805, or #1817)
- [ ] Branch was task/N-... (NOT bare `#N`)
- [ ] Tests pass: `ssh bigbox "cd /data/projects/terraphim/terraphim-ai-fresh && cargo test ..."`
- [ ] Clippy clean: `cargo clippy ... -- -D warnings`
- [ ] No unrelated code changes (surgical changes only)
- [ ] Acceptance criteria from the issue body all ticked

## 5. Follow-up issues filed (4)

These document tech debt surfaced this session; they are NOT in-flight work:

- **#1818** Bigbox repo corruption + adopt cargo-worktree
- **#1819** [VENDOR] opencode 1.14.48 Z.AI Coding Plan stream truncation
- **#1820** Raw `#N` branch creation bug (~400 malformed refs)
- **#1821** orchestrator working_dir + taxonomy_path point at corrupted repo

## 6. Remaining steps in original 10-step design

| # | Step | Status |
|---|---|---|
| 0 | Probe redesign | ✅ done |
| 1 | Z.AI investigation | ✅ done (closed as upstream issue) |
| 2 | (removed -- pi-rust already installed) | n/a |
| 3 | decision_tier markdown | ✅ done |
| 4 | (folded into 3) | n/a |
| 5 | Debug redaction (#1804) | 🟡 in flight |
| 6 | Config-error circuit-breaker (part of #1817) | 🟡 in flight |
| 7 | Rust merge-coordinator (#1805) | 🟡 queued |
| 8 | Memory watchdog (part of #1817) | 🟡 in flight |
| 9 | Bigbox sync runbook (part of #1817) | 🟡 in flight |
| 10 | Deploy + live verification | ⏳ blocked on 5-9 |

## 7. Open decisions for the operator

1. **When #1805 redispatches**, do we want it via @adf:implementation-swarm again (after #1817 finishes) or a different agent? Recommend: implementation-swarm with its existing skill chain.
2. **When all PRs land**, do we ff-merge in PR order or rebase + squash? Per PR #1794 precedent (admin force-merge), recommend admin-merge if branch protection blocks.
3. **After merge, who deploys?** Step 9's `bigbox-sync.sh` once #1817 lands, then deploy. Could run manually from this terminal once the script exists.

## 8. Memory entries written this session

- `feedback_check_bigbox_first.md` -- always SSH bigbox first before designing
- `feedback_adf_provider_routing.md` -- Z.AI probe timeouts are advisory; opencode is canonical for openai models
- `feedback_leverage_bigbox_for_compilation.md` -- compile on bigbox, not Mac
- `feedback_bigbox_worktree_corruption.md` -- recovery sequence for .git corruption
- `feedback_implementation_swarm_convention.md` -- @adf:implementation-swarm is the canonical implementation agent name

All five address recurring mistakes I made this session and should prevent them in future sessions.

## 9. Quick-status one-liner

```bash
ssh bigbox 'ps -eo pid,etime,rss,comm | grep -E "claude|opencode|pi-rust" | grep -v grep; \
            sudo journalctl -u adf-orchestrator --since "5 minutes ago" \
              | grep -iE "exit_class|PR created|Fixes #18" | tail -5'
```

If that shows agents alive AND no completion events, they are still working.
If it shows agent processes gone AND completion events, check Gitea for PR URLs.
