# ADF End-to-End Validation Plan: Proving Agents Can Produce Merge-Ready Contributions

**Date:** 2026-05-29
**Status:** Draft
**Author:** ADF Validation Analysis

---

## 1. Executive Summary

This plan validates that ADF agents can autonomously progress a real issue from pickup through to merge-ready contribution using the full disciplined development pipeline. The validation uses a dedicated test issue on Gitea and a new `zdp-validate-pipeline` flow that exercises every stage: issue pickup, research, design, implementation, review, corrections loop, verification, validation, and final approval.

**Current state:** The infrastructure exists and is partially proven. The `adf-useful-work-proof` flow successfully demonstrated matrix fan-out (3/3 slots, exit 0). The `zdp-full` flow produced high-quality research proposals and a design proposal for issue #1882. However, the full pipeline has never been run end-to-end with actual code changes.

**Goal:** Prove the complete loop works by running a single well-scoped issue through all stages, producing a PR that passes all quality gates.

---

## 2. Current Implementation State Analysis

### 2.1 What Works (Proven)

| Component | Evidence | Status |
|-----------|----------|--------|
| Flow engine matrix fan-out | `adf-useful-work-proof` flow: 3/3 slots, exit 0 | **Working** |
| Flow engine gate evaluation | `success_count` template substitution in gate conditions | **Working** |
| Template variable injection | `{{issue}}`, `{{matrix.slot}}`, `{{matrix.model}}` all resolved | **Working** |
| K=3 research proposals | 3 research proposals generated for #1882 with KLS scoring | **Working** |
| Research quality evaluation | KLS 6-dimension evaluation produced, score 4.5/5 | **Working** |
| K=3 design proposals | 1 design proposal generated (others may have timed out) | **Partial** |
| Issue context fetching | `adf-issue-stage` fetches title/body from Gitea API | **Working** |
| Artefact persistence | `.docs/adf/<issue>/` structure maintained | **Working** |
| Gitea comment posting | `gtr comment` posts structured progress to issues | **Working** |
| PageRank issue ranking | `gtr ready` returns 480+ issues ranked by dependency impact | **Working** |
| Orchestrator test suite | 794/795 tests pass (1 unrelated compound review test fails) | **Working** |
| Shell injection prevention | Matrix param validation against metacharacters | **Working** |

### 2.2 What Has NOT Been Proven (Gaps)

| Gap | Risk | Impact |
|-----|------|--------|
| **Full pipeline execution** (research -> design -> implementation -> review -> corrections -> verification -> validation -> judge) has never completed | **HIGH** | Cannot claim agents produce meaningful work |
| **Agent writes actual code** - no evidence an opencode/claude agent makes file changes during a flow step | **HIGH** | The pipeline may produce only documents, not code |
| **Skill loading during flow execution** - `skill://disciplined-research` syntax in task prompts is untested | **MEDIUM** | Agents may not load the correct skills |
| **Corrections loop** - `handle_corrections` re-triggers research+design but has never run end-to-end | **MEDIUM** | Review findings may go unaddressed |
| **Git branching in flows** - no evidence flow creates `task/IDX-short-title` branch | **HIGH** | Cannot produce PRs |
| **PR creation from flow** - no agent has created a PR on Gitea after implementation | **HIGH** | Cannot complete the contribution cycle |
| **Review findings with actual severity** - `review-findings.json` is a template, never populated by a real review | **MEDIUM** | Corrections loop cannot trigger meaningfully |
| **Merge gate** - merge-coordinator has never evaluated real review verdicts | **MEDIUM** | Cannot merge automatically |
| **Model fallback** - `fallback_model`/`fallback_provider` defined but untested under rate limiting | **LOW** | May fail on rate limit |
| **Stage-to-stage data handoff** - design depends on research artefact, but the content may not be rich enough for the next stage | **MEDIUM** | Stages produce boilerplate, not meaningful artefacts |

### 2.3 Critical Architecture Gap: adf-issue-stage is a Skeleton

The `.terraphim/bin/adf-issue-stage` script creates empty template artefacts with `<!-- Fill in -->` placeholders. It does NOT:

1. **Invoke an LLM** - It's pure bash that writes markdown templates
2. **Load skills** - No `skill://` resolution happens
3. **Make code changes** - No file editing, no git operations
4. **Run tests** - No `cargo test`, no linting
5. **Create branches** - No `git checkout -b`
6. **Create PRs** - No `gtr create-pull`

The `zdp-full` flow uses `opencode` as `cli_tool` directly (not `adf-issue-stage`), which should invoke a real LLM agent. But the flow's `task` prompt must be rich enough for the agent to know what to do.

---

## 3. Validation Strategy

### 3.1 Approach: Instrumented End-to-End Run

Rather than unit-testing each component in isolation, the validation runs a **single real issue** through the complete pipeline with instrumentation at each stage to capture:

- Artefact quality (not just existence)
- Agent behaviour (did it load the skill? did it write code?)
- Gate decisions (did it proceed correctly?)
- Error recovery (did corrections loop work?)

### 3.2 Test Issue Selection Criteria

The validation issue must be:
- **Self-contained**: Clear acceptance criteria in the issue body
- **Small scope**: < 100 lines of code change expected
- **Non-blocking**: Not depended on by other issues
- **Reversible**: Easy to revert if validation fails
- **Real**: An actual open issue, not a synthetic test

**Recommended:** Create a dedicated validation issue like:
> "feat(validation): add adf-pipeline-status command to adf-ctl"
> 
> Adds a new subcommand `adf-ctl pipeline-status <issue>` that reads `.docs/adf/<issue>/` and prints a summary of all stage artefacts and their status.

This is ideal because:
- It touches only `adf-ctl` (isolated, low risk)
- Has clear acceptance criteria (command runs, prints output)
- Is genuinely useful (gives visibility into ADF state)
- Requires ~50-100 lines of Rust code
- Has natural test cases

### 3.3 Validation Phases

#### Phase 0: Pre-flight Checks (Manual, 10 min)

Before running the flow:

1. **Verify build**: `cargo build -p terraphim_orchestrator --bin adf --bin adf-ctl`
2. **Verify Gitea access**: `gtr ready --owner terraphim --repo terraphim-ai | head -5`
3. **Verify model availability**: Confirm kimi/k2p6 and opencode are accessible
4. **Create validation issue**: Post the test issue on Gitea
5. **Clear any stale artefacts**: `rm -rf .docs/adf/<issue>/`

#### Phase 1: Infrastructure Prove-Out (Automated, 30 min)

Run the existing `adf-useful-work-proof` flow first to confirm the substrate:

```bash
cargo run -p terraphim_orchestrator --bin adf-ctl -- \
  --local flow adf-useful-work-proof --context "issue=<ISSUE>"
```

**Pass criteria:** All 3 matrix slots succeed, gate passes, assembled proof exists.

#### Phase 2: Single-Model Research (Automated, 15 min)

Run just the research step with a single model (not k=3) to verify skill loading:

```bash
cargo run -p terraphim_orchestrator --bin adf-ctl -- \
  --local trigger disciplined-research-agent --direct \
  --context "issue=<ISSUE>"
```

**Pass criteria:**
- Research artefact created in `.docs/adf/<ISSUE>/research.md`
- Artefact contains actual analysis (not `<!-- Fill in -->` placeholders)
- Gitea comment posted with summary
- Classification assigned (valid/stale/etc.)

**IMPORTANT DISTINCTION:** The `adf-issue-stage` agent creates empty templates. The `zdp-full` flow creates real proposals because it uses `opencode` as `cli_tool` with rich task prompts. We must decide which path to validate:

- **Path A**: Fix `adf-issue-stage` to invoke real LLM agents
- **Path B**: Validate only via `zdp-full` flow (which already uses opencode)
- **Path C**: Create a new hybrid script that wraps opencode with stage-specific prompts

**Recommendation:** Path B (validate via `zdp-full` flow) because it already uses real LLM invocation. Path A is a follow-up improvement.

#### Phase 3: Full k=3 Research + Quality Gate (Automated, 45 min)

Run the first 3 phases of `zdp-full`:

```bash
cargo run -p terraphim_orchestrator --bin adf-ctl -- \
  --local flow zdp-full --context "issue=<ISSUE>" \
  --step-range "matrix-research..gate-research-quality"
```

**Pass criteria:**
- 3 research proposals generated with KLS scores
- Quality evaluation produced
- Gate passes (score >= 3.0)

#### Phase 4: Full k=3 Design + Judge (Automated, 60 min)

Continue the flow through design and judge:

**Pass criteria:**
- 3 design proposals generated
- Quality evaluation produced
- Judge selects a design
- Gate passes

#### Phase 5: Implementation (Automated, 60 min)

The critical stage -- the agent must write real code:

**Pass criteria:**
- Agent creates a git branch `task/<ISSUE>-short-title`
- Agent modifies/creates files matching the design
- Agent writes tests
- Agent runs `cargo test` and all tests pass
- Implementation artefact records actual file changes
- No `<!-- Fill in -->` placeholders remain

**Risk mitigation:** If the agent fails to make code changes, the flow will still produce artefacts but the validation clearly flags this as a gap.

#### Phase 6: Structured PR Review (Automated, 30 min)

The review agent examines the implementation:

**Pass criteria:**
- Review findings JSON populated with real findings (not empty array)
- Findings include severity levels (P0/P1/P2)
- Confidence score assigned (1-5)
- Mermaid diagram produced (if structural-pr-review skill loads)

#### Phase 7: Corrections Loop (Conditional, 30-60 min)

If P0/P1 findings exist:

**Pass criteria:**
- Corrections agent reads review findings
- Corrections agent makes targeted fixes
- Re-review shows findings addressed
- Loop terminates (no infinite corrections)

#### Phase 8: Verification + Validation + Final Judge (Automated, 45 min)

**Pass criteria:**
- Verification: All tests pass, coverage adequate
- Validation: Acceptance criteria met
- Final judge: APPROVED verdict

#### Phase 9: PR Creation and Merge (Semi-automated, 15 min)

**Pass criteria:**
- PR created on Gitea via `gtr create-pull`
- PR references the issue (`Refs #<ISSUE>`)
- PR description includes artefact summaries
- All required reviews posted
- Merge gate evaluates correctly

---

## 4. Detailed Validation Flow Definition

A new flow `zdp-validate-pipeline` will be created that is a simplified version of `zdp-full` with:

1. **Single model** (not k=3) for faster execution during validation
2. **Stricter gate conditions** that check artefact content, not just exit code
3. **Instrumentation steps** that verify artefacts between stages
4. **Explicit git operations** (branch, commit, push) in task prompts

### 4.1 Flow Structure

```
Phase 1: research          (opencode + disciplined-research)
  -> gate: research-has-content
Phase 2: design            (opencode + disciplined-design)
  -> gate: design-has-file-changes
Phase 3: implementation    (opencode + disciplined-implementation)
  -> gate: implementation-has-code
Phase 4: review            (opencode + structured-pr-review)
  -> gate: review-has-findings
Phase 5: corrections       (conditional on P0/P1 findings)
  -> gate: corrections-resolved
Phase 6: verification      (opencode + disciplined-verification)
  -> gate: tests-pass
Phase 7: validation        (opencode + disciplined-validation)
  -> gate: acceptance-met
Phase 8: final-judge       (opencode + judge skill)
  -> gate: final-approved
Phase 9: pr-create         (action: gtr create-pull)
```

### 4.2 Key Differences from zdp-full

| Aspect | zdp-full | zdp-validate-pipeline |
|--------|----------|----------------------|
| Research slots | k=3 parallel | k=1 (faster) |
| Design slots | k=3 parallel | k=1 (faster) |
| Quality gates | KLS evaluation | Content check (non-empty, no placeholders) |
| Implementation prompt | Generic | Explicit: "create branch, write code, run tests, commit" |
| Review | structured-pr-review | Same, but writes JSON findings |
| Corrections | Separate agent | Inline: if P0/P1, re-implement |
| Git operations | Not specified | Explicit in task prompts |
| PR creation | Not in flow | Final action step |

---

## 5. Success Criteria (Must All Pass)

### 5.1 Functional Criteria

- [ ] **F1:** Agent picks up a real issue from `gtr ready`
- [ ] **F2:** Research produces a meaningful analysis (not template placeholders)
- [ ] **F3:** Design includes specific file changes with paths and rationale
- [ ] **F4:** Implementation creates/modifies at least 1 file with real code
- [ ] **F5:** Implementation creates at least 1 test that passes
- [ ] **F6:** Review identifies at least 1 finding (even if P2/minor)
- [ ] **F7:** Corrections loop addresses P0/P1 findings (if any)
- [ ] **F8:** Verification confirms `cargo test` passes
- [ ] **F9:** Validation confirms acceptance criteria are met
- [ ] **F10:** Final judge issues APPROVED verdict
- [ ] **F11:** PR is created on Gitea with correct references
- [ ] **F12:** PR can be merged (all gates green)

### 5.2 Quality Criteria

- [ ] **Q1:** No `<!-- Fill in -->` placeholders in final artefacts
- [ ] **Q2:** Artefacts contain file paths that exist in the repository
- [ ] **Q3:** Code changes pass `cargo clippy` with no warnings
- [ ] **Q4:** Code changes pass `cargo fmt --check`
- [ ] **Q5:** At least 1 test exercises the new code path
- [ ] **Q6:** Commit messages follow `feat(module): description Refs #IDX` pattern

### 5.3 Performance Criteria

- [ ] **P1:** Total flow completes within 4 hours (14400s timeout)
- [ ] **P2:** No stage exceeds its individual timeout
- [ ] **P3:** Corrections loop completes in at most 2 iterations
- [ ] **P4:** Flow does not consume more than $0.50 in LLM API costs

---

## 6. Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Agent produces only documentation, not code | **HIGH** | **HIGH** | Explicit task prompt with "you MUST write code and create a git branch" |
| Skill loading fails (`skill://` not resolved) | **MEDIUM** | **HIGH** | Include full skill instructions inline in task prompt as fallback |
| Model rate limits block execution | **MEDIUM** | **MEDIUM** | Use kimi (subscription, not pay-per-use) as primary model |
| Agent modifies wrong files | **LOW** | **MEDIUM** | Validate issue is small scope; review before merge |
| Corrections loop becomes infinite | **LOW** | **MEDIUM** | Cap at 2 iterations in corrections stage |
| Flow engine crashes mid-execution | **LOW** | **HIGH** | Flow state is persisted; can resume from checkpoint |
| Gitea API rate limiting | **LOW** | **LOW** | Minimal API calls per stage |

---

## 7. Implementation Tasks

### Task 1: Create Validation Issue on Gitea
- Create a well-scoped test issue with clear acceptance criteria
- Label it `status/ready` and `type/feature`

### Task 2: Create `zdp-validate-pipeline` Flow
- File: `.terraphim/flows/zdp-validate-pipeline.toml`
- Single-model, 9-phase flow with content gates
- Explicit git operation prompts

### Task 3: Create `adf-e2e-validate` Script
- File: `.terraphim/bin/adf-e2e-validate`
- Orchestrates the full validation run
- Captures metrics and timing
- Produces validation report

### Task 4: Fix Content Gate Detection
- Current gates only check `exit_code == 0`
- Need gates that check artefact content (no placeholders, has file paths)
- Option: Add a `checkpoint` step that runs `grep -c "Fill in"` on artefacts

### Task 5: Enhance Task Prompts for Code Generation
- Current prompts are research-oriented
- Implementation prompt must be explicit about:
  - Creating a git branch
  - Writing Rust code
  - Writing tests
  - Running `cargo test`
  - Committing with correct message format

### Task 6: Run the Validation
- Execute `adf-e2e-validate`
- Capture all artefacts
- Produce validation report

### Task 7: Analyse Results and Document Gaps
- Review each stage's output
- Identify what worked, what didn't
- Create follow-up issues for gaps

---

## 8. Post-Validation Improvements (If Gaps Found)

Based on the gap analysis in Section 2.3, the following improvements are anticipated:

### 8.1 Upgrade adf-issue-stage to Invoke Real Agents

Replace the template-writing bash script with a wrapper that:
1. Parses the stage from the task string
2. Constructs a rich prompt with skill loading instructions
3. Invokes `opencode` (or `claude`) with the prompt
4. Captures the agent's output as the artefact
5. Validates the artefact has meaningful content

### 8.2 Add Artefact Content Validation

Create a validation step after each stage that:
- Checks for placeholder text
- Verifies referenced file paths exist
- Ensures required sections are populated
- Returns non-zero exit code if content is insufficient

### 8.3 Implement Git Operations in Flow

Add explicit git steps:
- Pre-implementation: `git checkout -b task/IDX-short-title`
- Post-implementation: `git add . && git commit -m "feat: ... Refs #IDX"`
- Pre-PR: `git push origin task/IDX-short-title`

### 8.4 Corrections Loop with Iteration Cap

Implement a bounded corrections loop:
- Max 2 iterations
- Each iteration addresses specific P0/P1 findings by number
- After max iterations, escalate to human

### 8.5 Merge Gate Integration

Wire the flow's final step to the merge-coordinator:
- Final judge approves -> merge-coordinator evaluates all reviews
- All green -> auto-merge
- Any red -> create remediation issue

---

## 9. Timeline

| Phase | Duration | Owner |
|-------|----------|-------|
| Task 1: Create validation issue | 10 min | Human |
| Task 2: Create validation flow | 30 min | Agent |
| Task 3: Create e2e validate script | 30 min | Agent |
| Task 4: Fix content gates | 20 min | Agent |
| Task 5: Enhance task prompts | 20 min | Agent |
| Task 6: Run validation | 4 hours max | Automated |
| Task 7: Analyse results | 60 min | Human + Agent |
| **Total** | **~7 hours** | |

---

## 10. Validation Report Template

After the run, produce `.docs/adf/<ISSUE>/validation-report.md`:

```markdown
# ADF Pipeline Validation Report

**Issue:** #<ISSUE>
**Flow:** zdp-validate-pipeline
**Date:** YYYY-MM-DD
**Duration:** Xh Ym

## Stage Results

| Stage | Status | Duration | Artefact Quality |
|-------|--------|----------|------------------|
| Research | PASS/FAIL | Xm | X/5 |
| Design | PASS/FAIL | Xm | X/5 |
| Implementation | PASS/FAIL | Xm | X/5 |
| Review | PASS/FAIL | Xm | X findings |
| Corrections | N/A/PASS/FAIL | Xm | N iterations |
| Verification | PASS/FAIL | Xm | X tests passing |
| Validation | PASS/FAIL | Xm | X criteria met |
| Final Judge | APPROVED/REJECTED | Xm | - |
| PR Created | YES/NO | - | - |

## Functional Criteria: X/12 passed
## Quality Criteria: X/6 passed
## Performance Criteria: X/4 passed

## Key Findings
- Finding 1
- Finding 2

## Gaps Identified
- Gap 1: description
- Gap 2: description

## Recommendations
- Recommendation 1
- Recommendation 2
```
