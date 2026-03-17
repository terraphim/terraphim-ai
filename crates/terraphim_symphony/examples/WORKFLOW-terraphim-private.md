---
tracker:
  kind: gitea
  endpoint: https://git.terraphim.cloud
  api_key: $GITEA_TOKEN
  owner: zestic-ai
  repo: terraphim-private

polling:
  interval_ms: 30000

agent:
  runner: claude-code
  max_concurrent_agents: 2
  max_turns: 50
  claude_flags: "--dangerously-skip-permissions --allowedTools Bash,Read,Write,Edit,Glob,Grep,Skill,Task"
  settings: ~/.claude/symphony-settings.json

workspace:
  root: ~/symphony_workspaces

hooks:
  after_create: "git clone https://terraphim:${GITEA_TOKEN}@git.terraphim.cloud/zestic-ai/terraphim-private.git . && mkdir -p .docs"
  before_run: "set -e && git fetch origin && BRANCH=\"symphony/issue-${SYMPHONY_ISSUE_NUMBER}\" && git checkout main 2>/dev/null || true && git branch -D \"$BRANCH\" 2>/dev/null || true && if git rev-parse --verify \"origin/$BRANCH\" >/dev/null 2>&1; then git checkout -b \"$BRANCH\" \"origin/$BRANCH\" && git fetch origin main && git rebase origin/main 2>/dev/null || { git rebase --abort 2>/dev/null; git merge -X theirs origin/main --no-edit 2>/dev/null || true; }; else git fetch origin main && git checkout -b \"$BRANCH\" origin/main; fi"
  after_run: "set +e && BRANCH=\"symphony/issue-${SYMPHONY_ISSUE_NUMBER}\" && GITEA_API=\"https://git.terraphim.cloud/api/v1/repos/zestic-ai/terraphim-private\" && AUTH=\"Authorization: token ${GITEA_TOKEN}\" && git add -A && git diff --cached --quiet || git commit -m \"symphony: ${SYMPHONY_ISSUE_IDENTIFIER} - ${SYMPHONY_ISSUE_TITLE}\" || true && git push -u origin \"$BRANCH\" 2>/dev/null || true && GATE=true && GATE_MSG=\"\" && RS_COUNT=$(find crates -name '*.rs' 2>/dev/null | wc -l | tr -d ' ') && if [ \"$RS_COUNT\" -lt 1 ]; then GATE=false && GATE_MSG=\"No .rs files in crates/. \"; fi && cargo build 2>/tmp/sym_build_${SYMPHONY_ISSUE_NUMBER}.log || { GATE=false; GATE_MSG=\"${GATE_MSG}Build failed. \"; } && cargo test 2>/tmp/sym_test_${SYMPHONY_ISSUE_NUMBER}.log || { GATE=false; GATE_MSG=\"${GATE_MSG}Tests failed. \"; } && cargo clippy -- -D warnings 2>/tmp/sym_clippy_${SYMPHONY_ISSUE_NUMBER}.log || { GATE=false; GATE_MSG=\"${GATE_MSG}Clippy failed. \"; } && cargo fmt -- --check 2>/tmp/sym_fmt_${SYMPHONY_ISSUE_NUMBER}.log || { GATE=false; GATE_MSG=\"${GATE_MSG}Format check failed. \"; } && VMODEL_MSG=\"\" && if [ -f \".docs/verification-issue-${SYMPHONY_ISSUE_NUMBER}.md\" ]; then VMODEL_MSG=\"V-model verification present. \"; else VMODEL_MSG=\"WARNING: No verification artefact. \"; fi && if [ -f \".docs/validation-issue-${SYMPHONY_ISSUE_NUMBER}.md\" ]; then VMODEL_MSG=\"${VMODEL_MSG}V-model validation present. \"; else VMODEL_MSG=\"${VMODEL_MSG}WARNING: No validation artefact. \"; fi && if [ \"$GATE\" = \"true\" ]; then MERGE_OK=false && git fetch origin main && git checkout main && git pull origin main && git merge -X theirs \"$BRANCH\" --no-ff -m \"Merge ${SYMPHONY_ISSUE_IDENTIFIER}: ${SYMPHONY_ISSUE_TITLE}\" && git push origin main && MERGE_OK=true; if [ \"$MERGE_OK\" = \"true\" ]; then curl --retry 3 --retry-delay 5 --max-time 30 -sf -X PATCH -H \"$AUTH\" -H 'Content-Type: application/json' -d '{\"state\":\"closed\"}' \"${GITEA_API}/issues/${SYMPHONY_ISSUE_NUMBER}\" >/dev/null || true; curl --retry 3 --retry-delay 5 --max-time 30 -sf -X POST -H \"$AUTH\" -H 'Content-Type: application/json' -d \"{\\\"body\\\":\\\"Quality gate passed (build+test+clippy+fmt). ${VMODEL_MSG}Merged to main and closed.\\\"}\" \"${GITEA_API}/issues/${SYMPHONY_ISSUE_NUMBER}/comments\" >/dev/null || true; cargo clean 2>/dev/null || true; else git checkout \"$BRANCH\" 2>/dev/null; curl --retry 2 --max-time 15 -sf -X POST -H \"$AUTH\" -H 'Content-Type: application/json' -d '{\"body\":\"Merge to main failed (concurrent merge race). Will retry.\"}' \"${GITEA_API}/issues/${SYMPHONY_ISSUE_NUMBER}/comments\" >/dev/null || true; fi; else curl --retry 2 --max-time 15 -sf -X POST -H \"$AUTH\" -H 'Content-Type: application/json' -d \"{\\\"body\\\":\\\"Quality gate failed: ${GATE_MSG}${VMODEL_MSG}Issue left open for retry.\\\"}\" \"${GITEA_API}/issues/${SYMPHONY_ISSUE_NUMBER}/comments\" >/dev/null || true; fi"
  timeout_ms: 1200000

codex:
  turn_timeout_ms: 3600000
  stall_timeout_ms: 1200000
---
You are working on issue {{ issue.identifier }}: {{ issue.title }}.

{% if issue.description %}
## Issue Description

{{ issue.description }}
{% endif %}

## Project Context

Terraphim AI is a Rust workspace with 29+ crates providing privacy-first AI assistant
capabilities including semantic search, knowledge graphs, and multi-agent orchestration.

This is the **private fork** (zestic-ai/terraphim-private) being upgraded to sync with
181 upstream commits from terraphim/terraphim-ai.

**Build**: `cargo build`
**Test**: `cargo test`
**Lint**: `cargo clippy -- -D warnings`
**Format**: `cargo fmt -- --check`

### Key architecture
- `crates/terraphim_service/` -- Main service layer
- `crates/terraphim_middleware/` -- Haystack indexing and search
- `crates/terraphim_rolegraph/` -- Knowledge graph
- `crates/terraphim_automata/` -- Text matching and autocomplete
- `crates/terraphim_config/` -- Configuration management
- `crates/terraphim_persistence/` -- Storage abstraction
- `crates/terraphim_types/` -- Shared type definitions
- `crates/terraphim_symphony/` -- Excluded from workspace, builds independently
- `terraphim_server/` -- Main HTTP server binary

### Private-only features
- `crates/terraphim_email_worker/` -- Email sending via Resend API
- CLI onboarding wizard for first-time configuration
- Groq provider integration in multi-agent crate
- CI/CD release automation with tar.gz archives

## MANDATORY: Disciplined V-Model Workflow

You MUST follow ALL 5 phases in order for this issue. Each phase produces an artefact
that you commit to the branch before proceeding to the next phase.

### Phase 1: Research (EXPLORE)
Invoke the disciplined-research skill using the Skill tool:
  Use Skill tool with skill="disciplined-research"

Produce: `.docs/research-issue-{{ issue.identifier | split: "#" | last }}.md`
Contents:
- Problem statement and success criteria
- Current state analysis (existing code, data flows)
- Risk assessment and unknowns
- Dependencies analysis
- Essential Questions Check (2/3 YES minimum)
- Vital Few constraints (max 3)

Commit the artefact: `git add .docs/ && git commit -m "docs: research for {{ issue.identifier }}"`

### Phase 2: Design (ELIMINATE)
Invoke the disciplined-design skill using the Skill tool:
  Use Skill tool with skill="disciplined-design"

Produce: `.docs/design-issue-{{ issue.identifier | split: "#" | last }}.md`
Contents:
- Implementation plan (NOT code)
- File changes (new, modified, deleted)
- API design (public types, functions)
- Test strategy
- Implementation steps (sequenced)
- 5/25 Rule applied (5 in scope, rest eliminated)
- Eliminated Options documented
- Simplicity Check answered

Commit the artefact: `git add .docs/ && git commit -m "docs: design for {{ issue.identifier }}"`

### Phase 3: Implementation (EXECUTE)
Invoke the disciplined-implementation skill using the Skill tool:
  Use Skill tool with skill="disciplined-implementation"

Follow the design plan step by step:
- Write tests FIRST at each step
- Implement code to pass tests
- Run cargo build, cargo test, cargo clippy, cargo fmt at each step
- Log friction points
- Commit at each step with message: `feat(module): description (Refs {{ issue.identifier }})`

### Phase 4: Verification (RIGHT SIDE - verify code matches design)
Invoke the disciplined-verification skill using the Skill tool:
  Use Skill tool with skill="disciplined-verification"

Produce: `.docs/verification-issue-{{ issue.identifier | split: "#" | last }}.md`
Contents:
- Traceability matrix (requirement -> design -> code -> test)
- Unit test results
- Integration test results
- Coverage metrics
- Defect list with origins (loop back if needed)

Commit the artefact: `git add .docs/ && git commit -m "docs: verification for {{ issue.identifier }}"`

### Phase 5: Validation (RIGHT SIDE - validate solution meets requirements)
Invoke the disciplined-validation skill using the Skill tool:
  Use Skill tool with skill="disciplined-validation"

Produce: `.docs/validation-issue-{{ issue.identifier | split: "#" | last }}.md`
Contents:
- System test results against acceptance criteria
- Requirements compliance evidence
- Final quality gate results
- GO/NO-GO assessment

Commit the artefact: `git add .docs/ && git commit -m "docs: validation for {{ issue.identifier }}"`

## Quality Requirements
- cargo build must succeed
- cargo test must pass (or cargo test --workspace for cross-crate issues)
- cargo clippy -- -D warnings must pass
- cargo fmt -- --check must pass
- Use British English in documentation
- Never use mocks in tests

## Phase Budgeting

For complex issues (merge/reconciliation, large refactors), agents may exhaust their
turn budget during phases 1-3. When creating issues, consider splitting into:

- **Implementation issue**: Phases 1-3 (research, design, implementation)
- **Verification issue**: Phase 4 only, depends on implementation issue
- **Validation issue**: Phase 5 only, depends on verification issue

For verification-only issues (title contains "verify" or "verification"):
- Skip phases 1-3
- Read existing `.docs/research-issue-*.md` and `.docs/design-issue-*.md` artefacts
- Proceed directly to Phase 4: Verification

For validation-only issues (title contains "validate" or "validation"):
- Skip phases 1-4
- Read existing artefacts from prior phases
- Proceed directly to Phase 5: Validation

## CRITICAL Instructions

1. Follow all applicable V-model phases in order. Do NOT skip phases unless this is a verification-only or validation-only issue.
2. Examine ALL existing code in this workspace. Build on what is there.
3. Produce artefacts for each phase and commit them to the branch.
4. Ensure all quality gates pass before finishing.
5. Do NOT run gitea-robot, tea, or any issue-tracking commands.
6. If a skill is not available, perform the phase manually following its principles.

{% if attempt %}
## RETRY ATTEMPT {{ attempt }}

This is retry attempt {{ attempt }}. Previous attempts did not pass the quality gate.
Check existing code and V-model artefacts from previous attempts in `.docs/`.
Fix any build errors, test failures, or clippy warnings.
Focus on making `cargo build && cargo test && cargo clippy -- -D warnings && cargo fmt -- --check` all succeed.
If V-model artefacts exist from a previous attempt, build on them rather than starting over.
{% endif %}
