# Implementation Plan: ADF Real Issue Processing + k=3 Project Template

**Status**: Draft
**Research Doc**: `docs/research/research-adf-real-issue-processing-1882.md`
**Date**: 2026-05-28
**Estimated Effort**: 2-3 days

## Overview

### Summary
Replace the placeholder `adf-e2e-stage` script with `adf-issue-stage` that produces structured artefacts for each ZDP phase. Add `FlowDefinition` TOML files for k=3 parallel planning (research + design). Wire flow execution into local ADF via a runner script. Add `boosting.toml` from #1882.

### Approach
1. New `adf-issue-stage` bash script with real stage logic (issue read, classification, artefact write, structured comment)
2. New flow definitions in `.terraphim/flows/` using existing `FlowDefinition` schema
3. New `adf-flow-runner` script that invokes the orchestrator's flow engine locally
4. `boosting.toml` configuration for per-phase model selection

### Scope

**In Scope:**
- Replace `adf-e2e-stage` with `adf-issue-stage`
- Update all 12 agents in `adf.toml`
- Add k=3 flow definitions for research and design phases
- Add `boosting.toml` with per-phase model rosters
- Stage artefact contracts (file-based, git-tracked)
- Corrections driven by structured review output, not manual flags

**Out of Scope:**
- Parallel matrix execution (sequential is sufficient for k=3)
- LSP-based drift_check (terraphim_lsp is placeholder)
- Proposal/Verdict typed persistence (file artefacts are sufficient)
- CI workflow integration
- terraphim_multi_agent changes
- terraphim_persistence schema changes

**Avoid At All Cost:**
- Proof mode / dispatch-only comments on real issues
- New Rust crate or significant Rust code changes (keep in bash + TOML)
- Pay-per-use model integration in planning roster
- KG orchestration wiring (MockAutomata is not production)

## Architecture

### Component Diagram

```
.adf-ctl --local trigger <agent> --direct --context "issue=N"
  |
  v
adf-issue-stage (replaces adf-e2e-stage)
  |
  +-- reads issue from Gitea API (curl + jq)
  +-- classifies issue state
  +-- produces artefact in .docs/adf/<issue>/<stage>.md
  +-- posts structured comment (classification + artefact path)
  |
  v
.docs/adf/<issue>/
  ├── research.md          (disciplined-research output)
  ├── design.md            (disciplined-design output)
  ├── implementation.diff  (or no-op rationale)
  ├── review-findings.json (structured review output)
  └── corrections.log      (loopback log)

.adf-ctl --local flow <flow-name> --context "issue=N"
  |
  v
adf-flow-runner
  |
  +-- loads .terraphim/flows/<flow-name>.toml
  +-- resolves {{issue}}, {{matrix.*}} templates
  +-- invokes FlowExecutor via orchestrator binary or direct script
  |
  v
FlowDefinition (k=3 matrix fan-out)
  |
  +-- step 1: matrix (3 agents, 3 models)
  |     -> .docs/adf/<issue>/research-proposal-{1,2,3}.md
  +-- step 2: judge agent
  |     -> .docs/adf/<issue>/research-synthesis.md
  +-- step 3: checkpoint (human review)
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Bash scripts for stage logic | Matches existing architecture; no Rust recompilation | Rust plugins (over-engineering), Python (not in stack) |
| File artefacts in `.docs/adf/` | Git-tracked, human-readable, no schema migration | Database (terraphim_persistence), JSON-only (fragile) |
| `FlowDefinition` TOML for k=3 | Matrix fan-out already exists in flow engine | New fan-out primitive in multi_agent (duplicate) |
| Sequential k=3 via matrix | Sufficient for planning; avoids parallel execution change | Parallel execution (deferred optimisation) |
| `boosting.toml` as config file | Decouples model selection from agent definitions | Hardcoded in scripts (inflexible) |

### Simplicity Check

**What if this could be easy?** The simplest approach: replace the template comment with actual issue analysis + file write. The flow engine already exists; we just need TOML files to define the k=3 pipeline. No new Rust code needed.

**Senior Engineer Test**: Would a senior engineer call this overcomplicated? No. The changes are: one new script, two new flow TOML files, one config TOML, updated agent definitions. All configuration, no new runtime code.

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `.terraphim/bin/adf-issue-stage` | Real stage logic: read issue, classify, produce artefact, structured comment |
| `.terraphim/flows/zdp-research.toml` | k=3 research flow: matrix fan-out + judge + checkpoint |
| `.terraphim/flows/zdp-design.toml` | k=3 design flow: matrix fan-out + judge + checkpoint |
| `.terraphim/bin/adf-flow-runner` | Invokes flow definitions locally (bash wrapper) |
| `.terraphim/boosting.toml` | Per-phase model rosters from #1882 |
| `.terraphim/contracts/api.toml` | Drift_check contract example from #1882 |
| `.docs/adf/.gitkeep` | Ensure artefact directory is tracked |

### Modified Files

| File | Changes |
|------|---------|
| `.terraphim/adf.toml` | Update all 12 agents to use `adf-issue-stage` instead of `adf-e2e-stage` |
| `.terraphim/adf.toml` | Add `[[flows]]` section pointing to flow definitions |

### Deleted Files

| File | Reason |
|------|--------|
| `.terraphim/bin/adf-e2e-stage` | Replaced by `adf-issue-stage` |

## API Design

### `adf-issue-stage` Script Interface

```bash
# Usage: adf-issue-stage "<task-string>"
# Task string format: "stage=<stage>; [issue=<N>;] [artefact=<path>;] [extra-args]"

# Exit codes:
#   0 = stage completed, artefact produced
#   1 = stage failed, no artefact
#   2 = missing required context (no issue number, no gtr available)
```

### Stage Artefact Contracts

Each stage produces a specific artefact file. The artefact is a markdown file with a YAML frontmatter header.

#### Research Artefact (`research.md`)

```markdown
---
stage: research
issue: <number>
classification: valid | stale | duplicate | blocked | needs-rescope
timestamp: <ISO 8601>
agent: disciplined-research-agent
---

## Issue Summary
<1-paragraph summary of what the issue asks for>

## Current State
<What exists today in the codebase>

## Classification Rationale
<Why this classification was chosen>

## Key Findings
<Finding 1>
<Finding 2>

## Recommendations
<What should happen next>
```

Allowed classifications:
- `valid`: Issue describes work that needs doing
- `stale`: Issue describes work already done
- `duplicate`: Issue overlaps with another open issue
- `blocked`: Issue cannot proceed due to external dependency
- `needs-rescope`: Issue is partially done but needs refinement

#### Design Artefact (`design.md`)

```markdown
---
stage: design
issue: <number>
research_classification: <from research.md>
timestamp: <ISO 8601>
agent: disciplined-design-agent
---

## Design Overview
<What this plan accomplishes>

## File Changes
<New files, modified files, deleted files>

## Implementation Steps
<Numbered steps with file references>

## Test Strategy
<How to verify>

## Rollback Plan
<How to undo if things go wrong>
```

#### Review Findings (`review-findings.json`)

```json
{
  "stage": "review",
  "issue": <number>,
  "timestamp": "<ISO 8601>",
  "agent": "<reviewer>",
  "findings": [
    {
      "severity": "P0|P1|P2",
      "category": "security|correctness|performance|maintainability",
      "file": "<path>",
      "line": <number>,
      "description": "<finding>",
      "suggestion": "<how to fix>"
    }
  ],
  "outstanding": true|false,
  "summary": "<overall verdict>"
}
```

#### Corrections Behaviour

The corrections stage reads `review-findings.json`:
- If `outstanding == true` AND any P0/P1 findings exist: re-trigger research + design
- If `outstanding == false`: log "no corrections needed"
- Corrections never posts template comments; it either re-dispatches or exits cleanly

### `boosting.toml` Schema

```toml
# Per-phase model selection for local ADF
# Used by adf-issue-stage and adf-flow-runner

[planning]
parallel_proposals = 3
synthesis = "judge_compare"
timeout_per_proposal = "10m"

[[planning.models]]
provider = "anthropic"
model = "opus"

[[planning.models]]
provider = "kimi"
model = "kimi-for-coding/k2p6"

[[planning.models]]
provider = "openai"
model = "openai/gpt-5.4"

[planning.judge]
provider = "kimi"
model = "kimi-for-coding/k2p6"

[implementation]
parallel_proposals = 1
provider = "kimi"
model = "kimi-for-coding/k2p6"
verification = ["drift_check", "kg_validate", "tests"]

[review]
parallel_proposals = 2
synthesis = "consensus"

[[review.models]]
provider = "anthropic"
model = "sonnet"

[[review.models]]
provider = "kimi"
model = "kimi-for-coding/k2p6"
```

### `adf-flow-runner` Script Interface

```bash
# Usage: adf-flow-runner <flow-name> [--context "issue=N"]
# 
# Reads: .terraphim/flows/<flow-name>.toml
# Writes: .docs/adf/<issue>/<stage>-proposal-{1,2,3}.md
#         .docs/adf/<issue>/<stage>-synthesis.md

# The runner:
# 1. Loads the FlowDefinition TOML
# 2. Resolves {{issue}} template variables
# 3. For each step:
#    - Action: run shell command
#    - Agent: dispatch via adf-ctl --local trigger
#    - Matrix: expand and dispatch sequentially
#    - Gate: check condition
#    - Checkpoint: pause for human review
# 4. Records state in .docs/adf/<issue>/flow-state.json
```

## Flow Definitions

### `zdp-research.toml` (k=3 Research)

```toml
name = "zdp-research"
project = "terraphim-ai"
repo_path = "/home/alex/projects/terraphim/terraphim-ai"
timeout_secs = 1800

[[steps]]
name = "research-proposals"
kind = "agent"
cli_tool = "opencode"
task = "Run disciplined-research for issue {{issue}}. Write proposal to .docs/adf/{{issue}}/research-proposal-{{matrix.slot}}.md"
model = "{{matrix.model}}"
provider = "{{matrix.provider}}"
timeout_secs = 600
on_fail = "continue"

[steps.matrix]
max_parallel = 3
fail_strategy = "continue"

[[steps.matrix.params]]
slot = "1"
model = "opus"
provider = "anthropic"

[[steps.matrix.params]]
slot = "2"
model = "kimi-for-coding/k2p6"
provider = "kimi"

[[steps.matrix.params]]
slot = "3"
model = "openai/gpt-5.4"
provider = "openai"

[[steps]]
name = "check-proposals"
kind = "gate"
condition = "{{steps.research-proposals.success_count}} >= 2"
on_fail = "skip_failed"

[[steps]]
name = "synthesize-research"
kind = "agent"
cli_tool = "opencode"
model = "kimi-for-coding/k2p6"
provider = "kimi"
task = "Read .docs/adf/{{issue}}/research-proposal-{1,2,3}.md. Judge-compare them. Write synthesis to .docs/adf/{{issue}}/research-synthesis.md"
timeout_secs = 600

[[steps]]
name = "human-review"
kind = "checkpoint"
```

### `zdp-design.toml` (k=3 Design)

```toml
name = "zdp-design"
project = "terraphim-ai"
repo_path = "/home/alex/projects/terraphim/terraphim-ai"
timeout_secs = 1800

[[steps]]
name = "design-proposals"
kind = "agent"
cli_tool = "opencode"
task = "Read .docs/adf/{{issue}}/research-synthesis.md. Run disciplined-design. Write plan to .docs/adf/{{issue}}/design-proposal-{{matrix.slot}}.md"
model = "{{matrix.model}}"
provider = "{{matrix.provider}}"
timeout_secs = 600
on_fail = "continue"

[steps.matrix]
max_parallel = 3
fail_strategy = "continue"

[[steps.matrix.params]]
slot = "1"
model = "opus"
provider = "anthropic"

[[steps.matrix.params]]
slot = "2"
model = "kimi-for-coding/k2p6"
provider = "kimi"

[[steps.matrix.params]]
slot = "3"
model = "openai/gpt-5.4"
provider = "openai"

[[steps]]
name = "check-design-proposals"
kind = "gate"
condition = "{{steps.design-proposals.success_count}} >= 2"
on_fail = "skip_failed"

[[steps]]
name = "synthesize-design"
kind = "agent"
cli_tool = "opencode"
model = "kimi-for-coding/k2p6"
provider = "kimi"
task = "Read .docs/adf/{{issue}}/design-proposal-{1,2,3}.md. Judge-compare them. Write synthesis to .docs/adf/{{issue}}/design-synthesis.md"
timeout_secs = 600

[[steps]]
name = "human-review"
kind = "checkpoint"
```

## `adf-issue-stage` Implementation Detail

### Stage Handlers

```bash
#!/usr/bin/env bash
set -euo pipefail

task="${1:-}"
ADF_DOCS="/home/alex/projects/terraphim/terraphim-ai/.docs/adf"
GITEA_API="https://git.terraphim.cloud/api/v1"

# Extract issue number
if [[ "$task" =~ (^|[[:space:];])issue=([0-9]+) ]]; then
  issue="${BASH_REMATCH[2]}"
else
  # Auto-pick from gtr ready
  issue="$(gtr ready --owner terraphim --repo terraphim-ai | jq -r '.ready_issues[0].index // empty')"
  [[ "$issue" =~ ^[0-9]+$ ]] || { printf 'No issue available\n' >&2; exit 2; }
fi

# Ensure artefact directory exists
issue_dir="${ADF_DOCS}/${issue}"
mkdir -p "$issue_dir"

# Read issue body from Gitea API
issue_body="$(curl -sf -H "Authorization: token ${GITEA_TOKEN}" \
  "${GITEA_API}/repos/terraphim/terraphim-ai/issues/${issue}" | jq -r '.body // ""')"

# Parse stage
case "$task" in
  stage=disciplined-research*) stage="research" ;;
  stage=disciplined-design*) stage="design" ;;
  stage=disciplined-implementation*) stage="implementation" ;;
  stage=structured-pr-review*|stage=pr-reviewer*) stage="review" ;;
  stage=disciplined-verification*) stage="verification" ;;
  stage=disciplined-validation*) stage="validation" ;;
  stage=corrections*) stage="corrections" ;;
  stage=security-sentinel*) stage="security" ;;
  stage=build-runner*) stage="build" ;;
  stage=meta-learning*) stage="meta-learning" ;;
  stage=implementation-swarm*) stage="implementation" ;;
  *) stage="unknown" ;;
esac

# Stage-specific logic
case "$stage" in
  research)
    artefact="${issue_dir}/research.md"
    # Check if artefact already exists
    if [[ -f "$artefact" ]]; then
      printf 'Research artefact already exists: %s\n' "$artefact" >&2
      exit 0
    fi
    # Write research artefact with frontmatter
    cat > "$artefact" <<ARTEFACT
---
stage: research
issue: ${issue}
classification: pending
timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)
agent: disciplined-research-agent
---

## Issue Summary
<!-- Agent fills this in -->

## Current State
<!-- Agent analyses codebase -->

## Classification Rationale
<!-- Agent classifies: valid/stale/duplicate/blocked/needs-rescope -->

## Key Findings
<!-- Agent documents findings -->

## Recommendations
<!-- Agent recommends next steps -->
ARTEFACT
    # Post structured comment
    gtr comment --owner terraphim --repo terraphim-ai --index "$issue" \
      --body "## Research started

Stage: disciplined-research
Issue: #${issue}
Artefact: \`${artefact}\`

Research artefact created. Agent should populate classification and findings."
    printf 'Research artefact created: %s\n' "$artefact"
    ;;

  design)
    # Requires research artefact to exist
    research_artefact="${issue_dir}/research.md"
    if [[ ! -f "$research_artefact" ]]; then
      printf 'Research artefact not found. Run research first.\n' >&2
      exit 1
    fi
    artefact="${issue_dir}/design.md"
    if [[ -f "$artefact" ]]; then
      printf 'Design artefact already exists: %s\n' "$artefact" >&2
      exit 0
    fi
    cat > "$artefact" <<ARTEFACT
---
stage: design
issue: ${issue}
timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)
agent: disciplined-design-agent
---

## Design Overview
<!-- Agent fills this in -->

## File Changes
<!-- Agent specifies file changes -->

## Implementation Steps
<!-- Agent provides numbered steps -->

## Test Strategy
<!-- Agent defines verification approach -->
ARTEFACT
    gtr comment --owner terraphim --repo terraphim-ai --index "$issue" \
      --body "## Design started

Stage: disciplined-design
Issue: #${issue}
Artefact: \`${artefact}\`

Design artefact created. Agent should populate implementation plan."
    printf 'Design artefact created: %s\n' "$artefact"
    ;;

  review)
    artefact="${issue_dir}/review-findings.json"
    cat > "$artefact" <<ARTEFACT
{
  "stage": "review",
  "issue": ${issue},
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "agent": "structured-pr-review",
  "findings": [],
  "outstanding": false,
  "summary": "pending"
}
ARTEFACT
    printf 'Review artefact created: %s\n' "$artefact"
    ;;

  corrections)
    review_artefact="${issue_dir}/review-findings.json"
    if [[ ! -f "$review_artefact" ]]; then
      printf 'No review findings. Nothing to correct.\n'
      exit 0
    fi
    outstanding="$(jq -r '.outstanding // false' "$review_artefact")"
    p0_count="$(jq '[.findings[] | select(.severity == "P0")] | length' "$review_artefact")"
    p1_count="$(jq '[.findings[] | select(.severity == "P1")] | length' "$review_artefact")"
    if [[ "$outstanding" == "true" ]] && [[ $((p0_count + p1_count)) -gt 0 ]]; then
      adf_ctl="/home/alex/projects/terraphim/terraphim-ai/target/debug/adf-ctl"
      "$adf_ctl" --local trigger disciplined-research-agent --direct --context "issue=${issue} reason=corrections-p0-p1"
      "$adf_ctl" --local trigger disciplined-design-agent --direct --context "issue=${issue} reason=corrections-p0-p1"
      printf 'Corrections: re-triggered research + design for P0/P1 findings\n'
    else
      printf 'Corrections: no outstanding P0/P1 findings\n'
    fi
    ;;

  *)
    # Generic stages: build, security, meta-learning, verification, validation
    artefact="${issue_dir}/${stage}.md"
    cat > "$artefact" <<ARTEFACT
---
stage: ${stage}
issue: ${issue}
timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)
agent: ${stage}
---

## ${stage} output
<!-- Agent populates stage-specific output -->
ARTEFACT
    printf '%s artefact created: %s\n' "$stage" "$artefact"
    ;;
esac
```

### Corrections Contract

The corrections stage is driven by `review-findings.json`, not manual flags:

1. Read `review-findings.json`
2. Check `outstanding` field
3. Count P0 and P1 findings
4. If `outstanding == true` AND P0+P1 > 0: re-trigger research + design
5. Otherwise: exit 0 with log message
6. Never post a comment unless re-triggering (and even then, only the re-triggered stages comment)

## Updated `adf.toml`

All agents use `adf-issue-stage`. The `task` field carries the stage identifier.

```toml
# (header unchanged)

[[agents]]
name = "disciplined-research-agent"
layer = "Core"
cli_tool = "/home/alex/projects/terraphim/terraphim-ai/.terraphim/bin/adf-issue-stage"
task = "stage=disciplined-research"
project = "terraphim-ai"

[[agents]]
name = "disciplined-design-agent"
layer = "Core"
cli_tool = "/home/alex/projects/terraphim/terraphim-ai/.terraphim/bin/adf-issue-stage"
task = "stage=disciplined-design"
project = "terraphim-ai"

# ... (all 12 agents follow same pattern)

[[agents]]
name = "corrections"
layer = "Core"
capabilities = ["remediation", "review-fix", "quality-gate"]
cli_tool = "/home/alex/projects/terraphim/terraphim-ai/.terraphim/bin/adf-issue-stage"
task = "stage=corrections"
project = "terraphim-ai"

# Flow definitions for k=3 planning
[[flows]]
name = "zdp-research"
config = ".terraphim/flows/zdp-research.toml"

[[flows]]
name = "zdp-design"
config = ".terraphim/flows/zdp-design.toml"
```

## Test Strategy

### Unit Tests
| Test | Location | Purpose |
|------|----------|---------|
| `adf-issue-stage` parses task string | Manual bash -n + run | Script is syntactically valid |
| Research artefact has frontmatter | Run on test issue | YAML frontmatter present |
| Corrections reads review-findings.json | Mock JSON file | Correctly identifies P0/P1 |
| Design requires research artefact | Run without research | Exits 1 with message |

### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| Full pipeline on test issue | Manual dispatch via adf-ctl | All stages produce artefacts |
| k=3 flow produces 3 proposals | Manual flow run | 3 proposal files created |
| Corrections loopback works | Trigger corrections after review | Re-dispatches research + design |

### Validation
- `bash -n .terraphim/bin/adf-issue-stage` (syntax check)
- `adf --local --check .terraphim/adf.toml` (config validation)
- `ubs .terraphim/bin/adf-issue-stage` (bug scan)
- Manual test on #1336 (re-scope) or #1882 (new work)

## Implementation Steps

### Step 1: Create `adf-issue-stage` script
**Files:** `.terraphim/bin/adf-issue-stage`
**Description:** New script with real stage logic
**Tests:** `bash -n`, manual dispatch
**Estimated:** 2 hours

### Step 2: Create `boosting.toml`
**Files:** `.terraphim/boosting.toml`
**Description:** Per-phase model rosters from #1882
**Tests:** `toml::from_str` validation
**Estimated:** 30 minutes

### Step 3: Create flow definitions
**Files:** `.terraphim/flows/zdp-research.toml`, `.terraphim/flows/zdp-design.toml`
**Description:** k=3 matrix fan-out for research and design
**Tests:** Flow parses correctly, matrix params valid
**Estimated:** 1 hour

### Step 4: Create `adf-flow-runner` script
**Files:** `.terraphim/bin/adf-flow-runner`
**Description:** Bash wrapper that resolves templates and dispatches matrix steps
**Tests:** Manual flow run
**Estimated:** 2 hours

### Step 5: Update `adf.toml`
**Files:** `.terraphim/adf.toml`
**Description:** Point all agents at new script, add flows section
**Tests:** `adf --local --check`
**Estimated:** 30 minutes

### Step 6: Delete `adf-e2e-stage`
**Files:** `.terraphim/bin/adf-e2e-stage`
**Description:** Remove old placeholder
**Tests:** Confirm no references remain
**Estimated:** 5 minutes

### Step 7: Validate on real issue
**Files:** None
**Description:** Run full pipeline on #1336 or #1882
**Tests:** Artefacts produced, meaningful comments only
**Estimated:** 1 hour

### Step 8: Create drift_check contract example
**Files:** `.terraphim/contracts/api.toml`
**Description:** Example contract from #1882
**Tests:** toml validation
**Estimated:** 15 minutes

## Rollback Plan

If issues discovered:
1. Revert `.terraphim/adf.toml` to point at old script
2. Restore `adf-e2e-stage` from git history
3. All artefacts are in `.docs/adf/` which is git-tracked and can be removed

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Should `adf-flow-runner` invoke the Rust `FlowExecutor` directly or reimplement in bash? | Pending | Alex |
| How does `adf-ctl flow` command integrate with existing `adf-ctl` binary? | Pending | Alex |
| Should flow definitions be validated by the Rust flow config parser at load time? | Pending | Alex |
