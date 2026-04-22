# Design & Implementation Plan: Technical Product Owner Agent (Epic #769)

## 1. Summary of Target Behavior

After implementation:

1. **`product-development`** is a planning-only agent that produces roadmap prioritization and feature backlog recommendations. It no longer handles implementation or frontend work.

2. **`repo-steward`** is a new Growth-layer agent that runs every 6 hours, synthesizing repository stability and usefulness signals into evidence-backed Gitea issues.

3. **Meta-coordinator** dispatches planning issues to `product-development` and repo-health issues to `repo-steward`.

4. Both agents use direct mentions (`@adf:product-development`, `@adf:repo-steward`) without persona resolution ambiguity.

5. Repo-steward issues are deduplicated via Theme-ID and capped at one per panel per 24 hours.

## 2. Key Invariants and Acceptance Criteria

### Invariants

| ID | Invariant | Rationale |
|----|-----------|-----------|
| I1 | `product-development` never creates branches, code, or PRs | Planning-only contract |
| I2 | `repo-steward` never implements code or merges PRs | Synthesis-only contract |
| I3 | `repo-steward` creates at most one issue per panel per 24h | Spam prevention |
| I4 | All repo-steward issues have Theme-ID in body | Deduplication |
| I5 | All repo-steward issues have at least 2 evidence sources | Evidence quality |
| I6 | Both agents use subscription-only models (C1) | Cost/policy compliance |
| I7 | No orchestrator core Rust changes in v1 | Configuration-first approach |

### Acceptance Criteria

| Criterion | Test |
|-----------|------|
| AC1 | `product-development` template parses and contains no implementation workflow | `test_product_development_template.py` |
| AC2 | `repo-steward` template parses with correct schedule and capabilities | `test_repo_steward_template.py` |
| AC3 | Meta-coordinator dispatch prompt includes both roles | String assertion in test or manual inspection |
| AC4 | `product-development` test agent in mention.rs uses Carthos, not Lux | `mention.rs` test update |
| AC5 | `orchestrator.example.toml` reflects normalized roles | File diff review |
| AC6 | Architecture docs show both roles in correct layers | `.docs/adf-architecture.md` review |
| AC7 | Pilot project validates role separation | Manual validation |

## 3. High-Level Design and Boundaries

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    Existing ADF Infrastructure               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │   Gitea API  │  │  Quickwit    │  │  terraphim-agent │  │
│  │   (gtr CLI)  │  │  (optional)  │  │  (KG + learn)    │  │
│  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘  │
│         │                  │                    │            │
│         └──────────────────┼────────────────────┘            │
│                            │                                 │
│                            ▼                                 │
│                   ┌─────────────────┐                        │
│                   │   repo-steward  │                        │
│                   │  (Growth layer) │                        │
│                   │  15 */6 * * *   │                        │
│                   └────────┬────────┘                        │
│                            │                                 │
│              ┌─────────────┼─────────────┐                  │
│              ▼             ▼             ▼                  │
│        ┌─────────┐  ┌──────────┐  ┌──────────┐            │
│        │Stability│  │Usefulness│  │ Learning │            │
│        │  Panel  │  │  Panel   │  │ Capture  │            │
│        └────┬────┘  └────┬─────┘  └────┬─────┘            │
│             │            │             │                   │
│             └────────────┼─────────────┘                   │
│                          ▼                                  │
│                   ┌─────────────┐                          │
│                   │ Gitea Issue │                          │
│                   │ [Repo       │                          │
│                   │ Stewardship]│                          │
│                   └─────────────┘                          │
└─────────────────────────────────────────────────────────────┘
```

### Role Boundaries

| Role | Responsibility | Does NOT Do |
|------|---------------|-------------|
| `product-development` | Roadmap prioritization, feature backlog shaping, sequencing recommendations | Implementation, code review, frontend work |
| `repo-steward` | Synthesize repo health signals, identify stability/usefulness debt, recommend backlog actions | Implement fixes, merge PRs, run tests |
| `implementation-swarm` | Build features, write code, create PRs | Planning, prioritization, repo health analysis |

### Dispatch Routing

```
Meta-coordinator dispatch decision:
├─ Issue about roadmap, feature tradeoffs, sequencing, scoping
│  └─ Dispatch: @adf:product-development
│
├─ Issue about repo health, docs confusion, recurring friction,
│   onboarding pain, repeated manual remediation
│  └─ Dispatch: @adf:repo-steward
│
├─ Issue about security vulnerabilities
│  └─ Dispatch: @adf:security-sentinel
│
├─ Issue about implementation/bugfix
│  └─ Dispatch: @adf:implementation-swarm (default)
│
└─ Issue about quality/testing
   └─ Dispatch: @adf:test-guardian
```

## 4. File/Module-Level Change Plan

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `scripts/adf-setup/agents/product-development.toml` | Create | Does not exist | Planning-only agent template | None |
| `scripts/adf-setup/agents/repo-steward.toml` | Create | Does not exist | Repository stewardship template | None |
| `scripts/adf-setup/tests/test_product_development_template.py` | Create | Does not exist | Validates planning-only contract | product-development.toml |
| `scripts/adf-setup/tests/test_repo_steward_template.py` | Create | Does not exist | Validates stewardship contract | repo-steward.toml |
| `scripts/adf-setup/agents/meta-coordinator.toml` | Modify | Dispatches: developer, security, compliance, reviewer | Adds: product-development, repo-steward | Both new templates |
| `scripts/adf-setup/docs/cron-cadence.md` | Modify | Meta, Developer, Reviewer, Safety tiers | Adds: product-development (Core), repo-steward (Growth) | None |
| `crates/terraphim_orchestrator/orchestrator.example.toml` | Modify | product-development as Ferrox/Rust impl | product-development as Carthos/planning, adds repo-steward | None |
| `crates/terraphim_orchestrator/src/mention.rs` | Modify | Test agent: product-development with Lux persona | Test agent: product-development with Carthos persona | None |
| `.docs/adf-architecture.md` | Modify | Shows product-development in Core layer | Updates product-development description, adds repo-steward in Growth layer | None |

## 5. Step-by-Step Implementation Sequence

### Step 1: Normalize product-development (Issue #767)
**Files:** `scripts/adf-setup/agents/product-development.toml`, `scripts/adf-setup/tests/test_product_development_template.py`, `crates/terraphim_orchestrator/orchestrator.example.toml`, `crates/terraphim_orchestrator/src/mention.rs`
**Purpose:** Make product-development consistently mean roadmap/feature prioritization
**Deployable:** Yes - additive change, does not break existing agents

1.1 Create `scripts/adf-setup/agents/product-development.toml` with:
- `layer = "Core"`
- `schedule = "0 2 * * *"` (daily at 02:00)
- `persona = "Carthos"`
- `terraphim_role = "Carthos Architecture"`
- `capabilities = ["product-development", "roadmap-prioritization", "feature-prioritization", "backlog-shaping"]`
- Task: gather ready issues, rank by reach/impact/confidence/effort, post prioritization comment

1.2 Create `scripts/adf-setup/tests/test_product_development_template.py`:
- Parse TOML and verify required fields
- Verify no implementation workflow (no branch/PR/build commands in task)
- Verify capabilities include planning-only keywords

1.3 Update `orchestrator.example.toml`:
- Change product-development persona from "ferrox" to "carthos"
- Change terraphim_role from "Ferrox Rust" to "Carthos Architecture"
- Change capabilities to planning-only set
- Update task description to planning workflow

1.4 Update `mention.rs` tests:
- Change test agent "product-development" persona from "Lux" to "Carthos"
- Change capabilities from `["typescript", "frontend", "ui"]` to planning set
- Update test assertions accordingly

### Step 2: Extend meta-coordinator dispatch (Issue #767)
**Files:** `scripts/adf-setup/agents/meta-coordinator.toml`
**Purpose:** Route planning and health issues to correct agents
**Deployable:** Yes

2.1 Update DISPATCH_PROMPT in meta-coordinator.toml:
- Add `product-development` and `repo-steward` to role list
- Add routing rules in prompt:
  - "product-development: roadmap, feature tradeoffs, sequencing, scoping"
  - "repo-steward: repo health, docs confusion, recurring friction, onboarding pain"

2.2 Update case statement to accept new roles:
```bash
case "$ROLE" in
  developer|security|compliance|reviewer|product-development|repo-steward) ;;
  *) ROLE="developer" ;;
esac
```

### Step 3: Add repo-steward agent (Issue #768)
**Files:** `scripts/adf-setup/agents/repo-steward.toml`, `scripts/adf-setup/tests/test_repo_steward_template.py`
**Purpose:** Implement repository stewardship synthesis
**Deployable:** Yes

3.1 Create `scripts/adf-setup/agents/repo-steward.toml` with:
- `layer = "Growth"`
- `schedule = "15 */6 * * *"` (every 6 hours at :15)
- `persona = "Carthos"`
- `terraphim_role = "Carthos Architecture"`
- `capabilities = ["repo-stewardship", "stability-synthesis", "usefulness-synthesis", "backlog-prioritization"]`
- Task: two-panel evidence gathering, Theme-ID dedup, issue creation, learning capture

3.2 Create `scripts/adf-setup/tests/test_repo_steward_template.py`:
- Parse TOML and verify required fields
- Verify schedule is "15 */6 * * *"
- Verify capabilities include stewardship keywords
- Verify no dangerous flags (no auto-merge, no branch creation)

### Step 4: Update documentation (Issue #768)
**Files:** `scripts/adf-setup/docs/cron-cadence.md`, `.docs/adf-architecture.md`
**Purpose:** Document new roles and cadences
**Deployable:** Yes

4.1 Update `cron-cadence.md`:
- Add row: Core | product-development | 0 2 * * * | Daily planning
- Add row: Growth | repo-steward | 15 */6 * * * | Repo health synthesis

4.2 Update `adf-architecture.md`:
- Update product-development description from "Code review" to "Roadmap prioritization"
- Add repo-steward to Growth layer diagram
- Update ASCII and Mermaid diagrams

### Step 5: Pilot rollout (Issue #768)
**Files:** Live conf.d config on bigbox (not committed)
**Purpose:** Validate role separation and output quality
**Deployable:** N/A - validation only

5.1 Add both agents to one project config (e.g., terraphim-ai)
5.2 Trigger `@adf:product-development` on a roadmap issue
5.3 Verify it produces prioritization, not implementation
5.4 Trigger `@adf:repo-steward` on a repo-health issue
5.5 Verify it gathers evidence and creates structured issue
5.6 Observe one scheduled cycle
5.7 Review first 5 outputs for quality

## 6. Testing & Verification Strategy

### Automated Tests

| Criterion | Test Type | Test Location |
|-----------|-----------|---------------|
| AC1: product-development planning-only | Template validation | `scripts/adf-setup/tests/test_product_development_template.py` |
| AC2: repo-steward correct config | Template validation | `scripts/adf-setup/tests/test_repo_steward_template.py` |
| AC3: Meta-coordinator includes roles | String assertion | Template-level check or manual inspection |
| AC4: mention.rs tests updated | Unit test | `crates/terraphim_orchestrator/src/mention.rs` |
| AC5: example config normalized | File diff | `crates/terraphim_orchestrator/orchestrator.example.toml` |
| AC6: Architecture docs updated | Visual inspection | `.docs/adf-architecture.md` |

### Regression Tests

| Command | Purpose |
|---------|---------|
| `uv run pytest scripts/adf-setup/tests/test_migrate.py` | Migration tooling still passes |
| `cargo test -p terraphim_orchestrator mention` | Mention tests pass |
| `cargo test -p terraphim_orchestrator config` | Config parsing tests pass |
| `cargo test -p terraphim_orchestrator dispatcher` | Dispatcher tests pass |

### Manual Validation

| Step | Validation |
|------|------------|
| 5.2 | `@adf:product-development` produces prioritization output |
| 5.3 | No implementation work, no branch/PR creation |
| 5.4 | `@adf:repo-steward` gathers evidence from 2+ sources |
| 5.5 | Creates at most one issue per panel in 24h |
| 5.6 | Writes learning capture |
| 5.7 | First 5 outputs judged materially useful |

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| Prompt-only stewardship too weak | Pilot first; follow-up design if needed | Low - can escalate to core changes |
| Issue spam | Theme-ID + 24h cap + KG dedup | Very low |
| Semantic confusion persists | Clear docs, explicit mentions, test updates | Low |
| Cost overrun | Subscription models, sparse schedule | Low |
| Meta-coordinator dispatch accuracy | Keyword + LLM hybrid routing | Medium - may need tuning |
| Breaking existing product-development | Additive normalization, don't remove | Very low |

## 8. Open Questions / Decisions for Human Review

1. **Q**: Should `product-development` keep `schedule = "0 2 * * *"` or change?
   **Default**: Keep existing daily schedule.
   **Impact**: Low - can be tuned in live config.

2. **Q**: Should repo-steward use `layer = "Core"` or `layer = "Growth"`?
   **Default**: `Growth` (priority 2000) - repo stewardship is enhancement, not essential pipeline.
   **Impact**: Medium - affects dispatch priority.

3. **Q**: Should we change product-development persona from Ferrox to Carthos?
   **Default**: Yes - Carthos is architecture/planning aligned.
   **Impact**: High - affects all product-development dispatches.

4. **Q**: Should meta-coordinator use keyword or LLM routing?
   **Default**: LLM-based (following existing pattern) with keyword fallback.
   **Impact**: Medium - affects dispatch accuracy and cost.

5. **Q**: Should repo-steward issues go to same repo or central adf-fleet?
   **Default**: Same repo for visibility.
   **Impact**: Medium - affects issue clutter and discoverability.
