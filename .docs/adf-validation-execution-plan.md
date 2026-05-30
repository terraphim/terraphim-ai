# ADF End-to-End Validation Plan
# Validate Agents Produce Meaningful, Merge-Ready Contributions

**Date:** 2026-05-30
**Target Issue:** #1887 (adf-ctl pipeline-status subcommand)
**Validation Issue:** #1886 (meta-tracking)
**Flow:** `.terraphim/flows/zdp-validate-pipeline.toml`

---

## Executive Summary

This plan validates that the AI Dark Factory (ADF) on bigbox can autonomously execute the full disciplined development pipeline -- from issue pickup through to merge-ready contribution. We use issue #1887 as the validation target because it is small, self-contained, has clear acceptance criteria, and requires actual code changes with tests.

**Current ADF State:**
- Service: Active (PID 1500468, running since 10:37 CEST)
- Agents: 24 configured in terraphim.toml + 13 in other conf.d files = 37 total
- Binary: Version 1.8.0 (freshly rebuilt and deployed)
- Flows: 5 flow definitions available
- Skills: 14 disciplined skills installed
- Webhook: Listening on 172.18.0.1:9091

---

## Phase 0: Pre-Flight Checks (Manual)

Before triggering the validation, verify the environment is ready.

### Checklist

```bash
# 1. Verify ADF is running
ssh bigbox 'sudo systemctl is-active adf-orchestrator'
# Expected: active

# 2. Verify binary version
ssh bigbox '/usr/local/bin/adf --version'
# Expected: adf 1.8.0

# 3. Check available issues
ssh bigbox 'cd /opt/ai-dark-factory && gtr ready --owner terraphim --repo terraphim-ai | head -5'
# Expected: Issue list with PageRank scores

# 4. Verify opencode is available
ssh bigbox 'which opencode && opencode --version'
# Expected: path and version output

# 5. Check disk space
ssh bigbox 'df -h /opt/ai-dark-factory /tmp'
# Expected: > 10GB free on each

# 6. Verify Gitea token
ssh bigbox 'echo $GITEA_URL && test -n "$GITEA_TOKEN" && echo "Token set"'
# Expected: https://git.terraphim.cloud and "Token set"

# 7. Check no existing validation branch
ssh bigbox 'cd /data/projects/terraphim/terraphim-ai && git branch -a | grep task/1887 || echo "No existing branch"'
# Expected: "No existing branch"
```

### Artefact Directory Setup

```bash
ssh bigbox 'mkdir -p /data/projects/terraphim/terraphim-ai/.docs/adf/1887'
```

---

## Phase 1: Issue Pickup (Automated)

**Objective:** Verify the agent can identify and claim a real issue from Gitea.

### Execution

```bash
# Trigger the validation flow via adf-ctl
ssh bigbox 'cd /opt/ai-dark-factory && adf-ctl --local flow zdp-validate-pipeline --context "issue=1887"'
```

Or trigger via webhook:
```bash
# From local machine
adf-ctl trigger implementation-swarm --context "issue=1887 validate-pipeline"
```

### Success Criteria

- [ ] Flow execution starts within 30 seconds of trigger
- [ ] Issue #1887 is fetched from Gitea API
- [ ] Agent creates `.docs/adf/1887/research.md` (not empty, no placeholders)
- [ ] Agent posts comment to issue: "Research complete for #1887"

### Verification

```bash
# Check flow state
ssh bigbox 'ls -la /opt/ai-dark-factory/flow-states/zdp-validate-pipeline* 2>/dev/null | head -5'

# Check research artefact
ssh bigbox 'test -s /data/projects/terraphim/terraphim-ai/.docs/adf/1887/research.md && echo "Research exists" || echo "MISSING"'

# Check for placeholders
ssh bigbox 'grep -q "FILL_IN\|<!-- Fill in -->" /data/projects/terraphim/terraphim-ai/.docs/adf/1887/research.md 2>/dev/null && echo "HAS PLACEHOLDERS" || echo "Clean"'
```

---

## Phase 2: Disciplined Research

**Objective:** Agent produces meaningful research with actual codebase analysis.

### Expected Research Content

The research.md should contain:

1. **Issue Analysis**
   - Title: "feat(adf-ctl): add pipeline-status subcommand"
   - Body: Acceptance criteria, implementation details, constraints
   - Classification: `valid`

2. **Codebase Findings**
   - `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` exists (main binary)
   - Pipeline status logic already partially exists in adf-ctl.rs (lines 1039-1113)
   - Tests exist in adf-ctl.rs test module
   - The feature is mostly implemented but needs extraction/refinement

3. **File References**
   - `crates/terraphim_orchestrator/src/bin/adf-ctl.rs`
   - `crates/terraphim_orchestrator/src/bin/adf_ctl/` (potential new directory)

### Success Criteria

- [ ] Research references actual file paths that exist
- [ ] Research includes specific line numbers or function names
- [ ] No placeholder text (FILL_IN, <!-- Fill in -->)
- [ ] Classification is one of: valid, stale, duplicate, blocked, needs-rescope
- [ ] File paths verified against actual repository

### Verification Commands

```bash
ssh bigbox '
  cd /data/projects/terraphim/terraphim-ai
  RESEARCH=.docs/adf/1887/research.md
  
  # Check file exists and has content
  test -s $RESEARCH || { echo "FAIL: Research missing"; exit 1; }
  
  # Check no placeholders
  if grep -q "FILL_IN\|<!-- Fill in -->" $RESEARCH; then
    echo "FAIL: Has placeholders"
    exit 1
  fi
  
  # Check references real files
  if grep -q "crates/terraphim_orchestrator/src/bin/adf-ctl.rs" $RESEARCH; then
    echo "PASS: References real files"
  else
    echo "WARN: No file references found"
  fi
  
  # Check classification
  grep "classification:" $RESEARCH | head -1
'
```

---

## Phase 3: Disciplined Design

**Objective:** Agent produces specific, actionable design with file changes.

### Expected Design Content

The design.md should specify:

1. **New Files** (if any)
   - None required (can inline in adf-ctl.rs)
   - Optional: `crates/terraphim_orchestrator/src/bin/adf_ctl/pipeline_status.rs`

2. **Modified Files**
   - `crates/terraphim_orchestrator/src/bin/adf-ctl.rs`
     - Add `PipelineStatus` variant to `AdfSub` enum
     - Add handler in `run()` match
     - Add test cases

3. **Implementation Steps**
   1. Add `PipelineStatus { issue: String, format: OutputFormat }` to `AdfSub`
   2. Implement `cmd_pipeline_status()` function
   3. Add tests: existing artefacts, missing directory, partial artefacts
   4. Run tests

### Success Criteria

- [ ] Design includes specific file paths
- [ ] Design includes specific function/struct names
- [ ] Implementation steps are ordered and completable
- [ ] Test strategy includes at least 1 test
- [ ] Acceptance criteria map to issue requirements
- [ ] No placeholder text

### Verification

```bash
ssh bigbox '
  cd /data/projects/terraphim/terraphim-ai
  DESIGN=.docs/adf/1887/design.md
  
  test -s $DESIGN || { echo "FAIL: Design missing"; exit 1; }
  grep -q "FILL_IN\|<!-- Fill in -->" $DESIGN && { echo "FAIL: Has placeholders"; exit 1; }
  grep -q "\.rs\|\.toml" $DESIGN && echo "PASS: References code files" || echo "WARN: No code references"
'
```

---

## Phase 4: Implementation (Critical Phase)

**Objective:** Agent writes actual code, creates branch, commits, tests pass.

### Expected Implementation

The agent should:

1. Create branch: `task/1887-validation`
2. Modify `crates/terraphim_orchestrator/src/bin/adf-ctl.rs`
3. Add tests
4. Run `cargo test -p terraphim_orchestrator`
5. Commit with conventional message

### Success Criteria

- [ ] Git branch `task/1887-validation` exists
- [ ] At least 1 file modified or created
- [ ] At least 1 test added
- [ ] `cargo test` passes
- [ ] Code committed with message following convention
- [ ] Implementation status documents all changes

### Verification Commands

```bash
ssh bigbox '
  cd /data/projects/terraphim/terraphim-ai
  
  # Check branch exists
  git branch -a | grep task/1887-validation || { echo "FAIL: No branch"; exit 1; }
  
  # Check files changed
  CHANGES=$(git diff --name-only main..task/1887-validation | wc -l)
  if [ "$CHANGES" -ge 1 ]; then
    echo "PASS: $CHANGES files changed"
    git diff --stat main..task/1887-validation
  else
    echo "FAIL: No files changed"
    exit 1
  fi
  
  # Check for test files
  git diff --name-only main..task/1887-validation | grep -E "test|spec" && echo "PASS: Tests present" || echo "WARN: No test files"
'
```

### Test Execution

```bash
ssh bigbox '
  cd /data/projects/terraphim/terraphim-ai
  git checkout task/1887-validation
  cargo test -p terraphim_orchestrator --bin adf-ctl 2>&1 | tail -20
'
```

---

## Phase 5: Structured PR Review

**Objective:** Review agent produces real findings on actual code changes.

### Expected Review Content

The review should identify:

1. **Potential Issues**
   - Error handling for missing directories
   - Edge cases (empty directories, permission errors)
   - Test coverage completeness

2. **Code Quality**
   - Follows existing patterns in adf-ctl.rs
   - Proper error propagation
   - Documentation comments

### Success Criteria

- [ ] Review references specific file:line locations
- [ ] Findings are actionable (not vague)
- [ ] Severity assigned (P0/P1/P2)
- [ ] Confidence score 1-5 provided
- [ ] At least one finding OR explicit "No issues found" statement

### Verification

```bash
ssh bigbox '
  cd /data/projects/terraphim/terraphim-ai
  REVIEW=.docs/adf/1887/review-findings.md
  
  test -s $REVIEW || { echo "FAIL: Review missing"; exit 1; }
  grep -q "FILL_IN\|<!-- Fill in -->" $REVIEW && { echo "FAIL: Has placeholders"; exit 1; }
  
  # Check for specific findings or explicit no-findings
  if grep -q "P0\|P1\|P2\|No findings" $REVIEW; then
    echo "PASS: Review has findings or explicit clean"
  else
    echo "WARN: Unclear review status"
  fi
'
```

---

## Phase 6: Corrections Loop

**Objective:** Agent addresses P0/P1 findings from review.

### Execution

Conditional on review findings. If no P0/P1, skip.

### Success Criteria

- [ ] Each P0 finding addressed with specific commit
- [ ] Each P1 finding addressed or marked as false positive with rationale
- [ ] Tests still pass after corrections
- [ ] Review findings updated to mark items as [FIXED]

### Verification

```bash
ssh bigbox '
  cd /data/projects/terraphim/terraphim-ai
  
  # Check if corrections were made
  git log --oneline task/1887-validation | head -5
  
  # Run tests post-corrections
  cargo test -p terraphim_orchestrator --bin adf-ctl 2>&1 | grep "test result:" | tail -1
'
```

---

## Phase 7: Verification

**Objective:** Confirm implementation matches design and tests pass.

### Verification Steps

```bash
ssh bigbox '
  cd /data/projects/terraphim/terraphim-ai
  
  # 1. Check design compliance
  echo "=== Design Compliance ==="
  grep -A 20 "Implementation Steps" .docs/adf/1887/design.md | head -15
  
  # 2. Run full test suite
  echo "=== Test Results ==="
  cargo test -p terraphim_orchestrator 2>&1 | grep "test result:" | tail -3
  
  # 3. Run clippy
  echo "=== Clippy ==="
  cargo clippy -p terraphim_orchestrator 2>&1 | grep "error\|warning" | wc -l
  
  # 4. Check formatting
  echo "=== Format ==="
  cargo fmt -- --check 2>&1 | head -5
'
```

### Success Criteria

- [ ] `cargo test` passes (all tests green)
- [ ] `cargo clippy` has no errors (warnings acceptable)
- [ ] `cargo fmt --check` passes
- [ ] Design compliance documented in verification report

---

## Phase 8: Validation

**Objective:** Confirm acceptance criteria from original issue are met.

### Issue #1887 Acceptance Criteria

1. [ ] `adf-ctl pipeline-status <issue>` prints summary
2. [ ] Reads artefacts from `.docs/adf/<issue>/`
3. [ ] Output includes stage name, status, file size, timestamp
4. [ ] Exit code 0 when artefacts exist, 1 when missing
5. [ ] Unit tests cover: existing, missing, partial

### Verification

```bash
ssh bigbox '
  cd /data/projects/terraphim/terraphim-ai
  
  # Build adf-ctl with changes
  cargo build --release -p terraphim_orchestrator --bin adf-ctl
  
  # Test with existing issue (should succeed)
  ./target/release/adf-ctl pipeline-status 1887 && echo "PASS: Exit 0" || echo "FAIL: Exit non-zero"
  
  # Test with non-existent issue (should fail)
  ./target/release/adf-ctl pipeline-status 99999 2>/dev/null && echo "FAIL: Should have exited 1" || echo "PASS: Exit 1"
'
```

---

## Phase 9: Final Judge and PR Creation

**Objective:** Final approval verdict and PR creation.

### Success Criteria

- [ ] Final approval verdict: APPROVED or REJECTED with rationale
- [ ] No outstanding P0 findings
- [ ] All acceptance criteria verified
- [ ] PR created on Gitea
- [ ] PR references issue #1887

### PR Creation Command

```bash
ssh bigbox '
  cd /data/projects/terraphim/terraphim-ai
  git checkout task/1887-validation
  git push origin task/1887-validation
  
  gtr create-pull \
    --owner terraphim \
    --repo terraphim-ai \
    --title "feat(adf-ctl): add pipeline-status subcommand for #1887" \
    --base main \
    --head task/1887-validation \
    --body "Implements pipeline-status subcommand for ADF artefact visibility.

Closes #1887

## Changes
- [List from implementation status]

## Verification
- cargo test: PASS
- cargo clippy: PASS
- cargo fmt: PASS

## Artefacts
- .docs/adf/1887/research.md
- .docs/adf/1887/design.md
- .docs/adf/1887/implementation-status.md
- .docs/adf/1887/review-findings.md
- .docs/adf/1887/verification-report.md
- .docs/adf/1887/validation-report.md
- .docs/adf/1887/final-approval-verdict.md"
'
```

---

## Success Criteria Summary

### Functional (12 criteria)

| ID | Criterion | Verification Command |
|----|-----------|---------------------|
| F1 | Agent picks up issue from `gtr ready` | `gtr view-issue --owner terraphim --repo terraphim-ai --index 1887` |
| F2 | Research has no placeholders | `! grep -q "FILL_IN\|<!-- Fill in -->" .docs/adf/1887/research.md` |
| F3 | Design includes file changes | `grep -q "\.rs\|\.toml" .docs/adf/1887/design.md` |
| F4 | Implementation modifies files | `git diff --name-only main..task/1887-validation \| wc -l` >= 1 |
| F5 | Tests added and passing | `cargo test -p terraphim_orchestrator --bin adf-ctl` |
| F6 | Review has real findings | `.docs/adf/1887/review-findings.md` exists and has content |
| F7 | Corrections applied (if needed) | `git log --oneline task/1887-validation \| grep -i "fix\|correct"` |
| F8 | Verification confirms tests pass | `cargo test` exit code 0 |
| F9 | Validation confirms acceptance criteria | `.docs/adf/1887/validation-report.md` has all YES |
| F10 | Final judge APPROVED | `.docs/adf/1887/final-approval-verdict.md` has approved: true |
| F11 | PR created on Gitea | `gtr list-pulls --owner terraphim --repo terraphim-ai --state open` |
| F12 | PR can be merged | PR has no conflicts, CI passes |

### Quality (6 criteria)

| ID | Criterion | Verification |
|----|-----------|-------------|
| Q1 | No placeholder text in any artefact | `grep -r "FILL_IN\|<!-- Fill in -->" .docs/adf/1887/ \|\| true` |
| Q2 | Artefacts reference real file paths | Files exist in git repo |
| Q3 | Code passes clippy | `cargo clippy` exit code 0 |
| Q4 | Code passes fmt | `cargo fmt --check` exit code 0 |
| Q5 | Tests exercise new code paths | Test coverage > 0% for changed files |
| Q6 | Commits follow convention | `git log --oneline task/1887-validation` matches `type(scope): message` |

---

## Risk Analysis

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Agent produces only docs, not code | HIGH | Critical | Flow has explicit "you MUST write code" prompts; gate checks for git diff |
| Model rate limits or failures | MEDIUM | High | Use kimi-for-coding/k2p6 (subscription); fallback to same model |
| Corrections loop infinite | LOW | Medium | Flow caps at 1 iteration; manual intervention after 2 hours |
| Git branch conflicts | LOW | Medium | Check for existing branch in pre-flight; use unique suffix |
| Disk space exhaustion | LOW | Critical | Pre-flight checks disk; /opt has 500GB+ available |
| Test failures in unrelated code | MEDIUM | Medium | Run only terraphim_orchestrator tests, not full workspace |
| Agent spawns unexpected processes | MEDIUM | High | Monitor with `pgrep`; kill stray processes if needed |

---

## Monitoring During Execution

### Real-Time Monitoring

```bash
# Terminal 1: Watch service logs
ssh bigbox 'sudo journalctl -u adf-orchestrator -f'

# Terminal 2: Watch agent processes
ssh bigbox 'watch -n 5 "pgrep -a -f opencode \|\| echo No agents"'

# Terminal 3: Watch flow state
ssh bigbox 'watch -n 30 "ls -la /opt/ai-dark-factory/flow-states/ 2>/dev/null \| tail -5"'

# Terminal 4: Watch artefact directory
ssh bigbox 'watch -n 30 "ls -la /data/projects/terraphim/terraphim-ai/.docs/adf/1887/ 2>/dev/null"'
```

### Key Metrics to Track

- **Flow step duration:** Each step should complete within its timeout
- **Agent spawn time:** opencode should start within 60 seconds
- **Model API latency:** kimi responses within 30-60 seconds
- **Disk usage:** Should not grow beyond 1GB during validation
- **Memory usage:** adf-orchestrator should stay below 2GB

---

## Rollback Plan

If validation fails catastrophically:

```bash
# 1. Stop the flow
ssh bigbox 'sudo systemctl restart adf-orchestrator'

# 2. Clean up branch
ssh bigbox '
  cd /data/projects/terraphim/terraphim-ai
  git checkout main
  git branch -D task/1887-validation 2>/dev/null || true
  git push origin --delete task/1887-validation 2>/dev/null || true
'

# 3. Clean up artefacts (preserve for analysis)
ssh bigbox '
  mv /data/projects/terraphim/terraphim-ai/.docs/adf/1887 \
     /data/projects/terraphim/terraphim-ai/.docs/adf/1887-failed-$(date +%Y%m%d-%H%M%S)
'

# 4. Close PR if created
# Manual: gtr list-pulls --owner terraphim --repo terraphim-ai --state open
```

---

## Post-Validation Actions

### If Validation Succeeds

1. Merge PR #1887 (or keep open for human review)
2. Update issue #1886 with success report
3. Document lessons learned in wiki
4. Close issue #1887
5. Schedule next validation with more complex issue

### If Validation Fails

1. Document failure mode in `.docs/adf/1887/failure-analysis.md`
2. Create follow-up issues for each gap:
   - Agent not writing code -> #1886-followup-1
   - Skill loading failing -> #1886-followup-2
   - etc.
3. Update issue #1886 with failure report and next steps
4. Do NOT close #1887 (keep for re-validation)

---

## Appendices

### A. Available Agent Dispatch Methods

```bash
# Method 1: Direct flow execution (local)
cd /home/alex/projects/terraphim/terraphim-ai
adf-ctl --local flow zdp-validate-pipeline --context "issue=1887"

# Method 2: Webhook trigger (remote)
adf-ctl trigger implementation-swarm --context "issue=1887 validate-pipeline"

# Method 3: Direct agent dispatch
adf --local --agent disciplined-research-agent

# Method 4: Gitea mention (creates webhook event)
# Comment on issue: "@adf:implementation-swarm issue=1887"
```

### B. Relevant Files

| File | Purpose |
|------|---------|
| `.terraphim/flows/zdp-validate-pipeline.toml` | Validation flow definition |
| `.terraphim/flows/zdp-full.toml` | Full k=3 flow with quality gates |
| `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` | CLI binary (target for changes) |
| `crates/terraphim_orchestrator/src/flow/executor.rs` | Flow execution engine |
| `.docs/adf-validation-plan.md` | Original validation plan |

### C. Skill References

| Skill | Used In | Purpose |
|-------|---------|---------|
| disciplined-research | Phase 1 | Issue analysis and classification |
| disciplined-design | Phase 2 | Implementation planning |
| disciplined-implementation | Phase 3 | Code writing and testing |
| structural-pr-review | Phase 4 | Code review with severity |
| disciplined-verification | Phase 6 | Test and compliance check |
| disciplined-validation | Phase 7 | Acceptance criteria verification |

---

## Sign-Off

**Prepared by:** ADF Orchestrator
**Date:** 2026-05-30
**Version:** 1.0
**Status:** Ready for Execution
