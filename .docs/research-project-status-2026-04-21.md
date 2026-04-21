# Research Document: Project Status Assessment 2026-04-21

## 1. Problem Restatement and Scope

**Problem**: Determine what work can be progressed right now, given the current state of the codebase, open issues, PRs, and infrastructure.

**In scope**:
- Current build/test/lint status
- Open PRs on GitHub and Gitea
- High-priority unblocked issues from PageRank triage
- Infrastructure blockers (CI runners, disk, memory)
- Security vulnerabilities

**Out of scope**:
- New feature design from scratch
- Long-term roadmap planning
- Marketing/content tasks

## 2. Current System State

### 2.1 Build & Lint Status
- **cargo build --workspace**: PASSES (5 warnings in terraphim_agent -- unused functions in procedure.rs)
- **cargo clippy --workspace**: PASSES with same 5 warnings
- **cargo test --workspace**: TIMED OUT after 180s (needs investigation -- likely individual slow tests, not a fundamental breakage)
- **Branch**: main, up-to-date with origin/main

### 2.2 Untracked Files
- `session-ses_2898.md` (session artifact)
- `update_output.txt` (temporary output)

### 2.3 Warnings to Fix
- `terraphim_agent/src/learnings/procedure.rs`: `TRIVIAL_COMMANDS`, `is_trivial_command`, `from_session_commands` are unused

## 3. Open PRs

### GitHub PRs (2 open)

| PR | Title | State | Mergeable | Branch |
|----|-------|-------|-----------|--------|
| #825 | feat(repl): add --format and --robot flags to /search command | OPEN | CONFLICTING | task/696-repl-format-robot-dispatch |
| #823 | fix(security): RUSTSEC-2026-0098/0099 + rand 0.10.1 + remove dead mcp-client dep | OPEN | UNKNOWN | task/632-security_checklist-remediation-russecurity_checklist-webpki |

### Gitea PRs
- Gitea has open PRs (list-pulls output was large/truncated) -- need further investigation

### PR Analysis

**PR #823 (security fixes)**: HIGHEST PRIORITY
- Fixes RUSTSEC-2026-0098/0099/0097 (critical CVEs)
- Upgrades rustls-webpki, removes dead mcp-client dep
- Adds Ollama binding fix script
- Mergeable status UNKNOWN -- needs rebase/conflict check
- CI cannot validate (runners stopped)
- **Actionable**: Rebase onto main, resolve conflicts, merge manually if CI is down

**PR #825 (REPL format flags)**: CONFLICTING
- Adds --format and --robot flags to REPL /search command
- Has merge conflicts with main
- Gitea issue #696 already CLOSED (work was tracked there)
- Tests: 29/29 pass
- **Actionable**: Rebase onto main, resolve conflicts

### Gitea Issue #326: Merge feature/warp-drive-theme into main
- Priority 182 (highest in entire backlog)
- Assigned to merge-coordinator
- Contains ADF pipeline gap fixes, compound review improvements
- 91 comments -- long-running coordination issue
- **Actionable**: Check branch status, resolve conflicts, merge

## 4. Top Unblocked Issues by PageRank

From `gtr ready` output, these are unblocked and have the highest PageRank/priority:

### Critical / Infrastructure (Do First)

| Index | Title | Priority | Status |
|-------|-------|----------|--------|
| #725 | [Infra] All GitHub Actions runners STOPPED | 0 | open, unblocked |
| #695 | [Infra] Rust target dir 602G -- schedule cargo clean | 0 | open, unblocked |
| #874 | [ADF] CRITICAL: Memory exhaustion -- 99.5% (63.7G/64G) | 0 | open, unblocked |
| #644 | Security Audit: Critical CVEs + Port 11434 Exposure | 12 | open, unblocked |

### High-Value Feature Work (Unblocked)

| Index | Title | Priority | PageRank |
|-------|-------|----------|----------|
| #326 | Merge feature/warp-drive-theme into main | 182 | 0.15 |
| #625 | Epic: EDM Scanner | 40 | 0.15 |
| #626 | EDM Scanner: terraphim_negative_contribution crate | 40 | 0.005 |
| #680 | feat(codebase-eval): evaluation manifest types and TOML/YAML loader | 0 | 0.008 |
| #144 | Epic: Inter-agent orchestration via Gitea mentions | 50 | 0.017 |
| #416 | [ADF] Epic: Ship ADF agent logs to Quickwit | 100 | 0.15 |
| #330 | Phase 1: SQLite shared learning store | 37 | 0.004 |

### Quick Wins

| Index | Title | Effort |
|-------|-------|--------|
| #867/#695 | cargo clean (602GB target dir) | 1 command |
| #868 | Remove unused procedure store methods | Small |
| #869 | Format code in mention.rs | Trivial |
| #216 | Clean up dead code and deprecated functions | Small |

## 5. Constraints and Blockers

### Hard Blockers
1. **CI runners all STOPPED** (#725) -- cannot validate PRs automatically
2. **Memory at 99.5%** (#874) -- system may OOM during builds/tests
3. **Target dir 602GB** (#695/#867) -- disk pressure affecting builds

### Soft Constraints
1. Security CVEs block production merges (#644, #632)
2. PR #825 has merge conflicts
3. No tea CLI login available (use gtr via API instead)

### Assumptions
- The security PR #823 was created on a previous session and may need rebase
- The warp-drive-theme branch may be stale (created Apr 5, last updated Apr 7)
- cargo test timeout may be due to memory pressure (99.5% RAM used)

## 6. Risks, Unknowns, and Assumptions

### Risks
| Risk | Impact | Likelihood |
|------|--------|------------|
| OOM during cargo operations | High (system crash) | High (99.5% memory) |
| Stale warp-drive-theme branch conflicts | Medium (merge difficulty) | Medium |
| Security PR #823 conflicts with main | Medium (delays CVE fix) | Medium |

### Unknowns
- Whether cargo clean will free enough space for comfortable operation
- Whether the memory exhaustion is from running agents or leaked processes
- State of the warp-drive-theme branch after 2 weeks

### Assumptions
- CI runners can be restarted by running the commands in issue #725
- Security fixes in PR #823 are sound (already reviewed in previous session)
- The 5 clippy warnings are safe to fix in passing

## 7. Recommended Action Sequence

### Immediate (Do Now)
1. **cargo clean** (#695/#867) -- frees ~600GB, reduces memory pressure
2. **Kill zombie/leaked processes** (#874, #857) -- free RAM
3. **Merge security PR #823** (#632, #644) -- resolve conflicts, merge. Critical CVEs.

### Short-Term (This Session)
4. **Resolve PR #825 conflicts** -- rebase task/696 branch, fix conflicts
5. **Fix 5 clippy warnings** in procedure.rs -- remove dead code
6. **Start EDM Scanner Step 1** (#626) -- create terraphim_negative_contribution crate skeleton

### After Infrastructure Restored
7. **Restart CI runners** (#725)
8. **Assess warp-drive-theme branch** (#326) -- check if still viable or stale
9. **Continue codebase-eval manifest types** (#680)

## 8. Questions for Human Reviewer

1. **Should I proceed with cargo clean immediately?** It will require rebuilding everything (~3-5 min) but frees 600GB and may resolve the memory issue.
2. **Security PR #823**: Should I rebase and merge it now, or wait for CI to be restored?
3. **warp-drive-theme branch (#326)**: Is this still relevant after 2 weeks, or should it be closed as stale?
4. **Which feature should I prioritise after infrastructure fixes?** EDM Scanner (#626), codebase-eval (#680), or shared learning store (#330)?
5. **Should I restart the GitHub Actions runners?** This is a manual operation on bigbox.
