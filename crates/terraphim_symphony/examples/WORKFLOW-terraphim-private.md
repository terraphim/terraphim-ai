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
  after_create: "git clone https://terraphim:${GITEA_TOKEN}@git.terraphim.cloud/zestic-ai/terraphim-private.git . && mkdir -p .docs && cat > CLAUDE.md << CLAUDEEOF\n# Terraphim Private Agent Instructions\n\n## Your Task\nYou are implementing issue #${SYMPHONY_ISSUE_NUMBER} for the terraphim-private workspace.\nFollow the DISCIPLINED V-MODEL workflow below for every feature.\nDo NOT run gitea-robot, tea, or any task-tracking commands.\nThe orchestrator handles all task tracking.\n\n## Project Overview\nTerraphim AI is a privacy-first AI assistant with semantic search across multiple\nknowledge repositories. This is the private fork (zestic-ai/terraphim-private) which\nadds: email-worker (Resend), CLI onboarding wizard, Groq provider, and CI/CD release\nautomation on top of the public terraphim/terraphim-ai codebase.\n\nThe fork is being upgraded to sync with 181 upstream commits from terraphim-ai main.\n\n## MANDATORY: Disciplined V-Model Workflow\n\nYou MUST follow ALL 5 phases in order. Each phase produces an artefact that you\ncommit to the branch before proceeding to the next phase.\n\n### Phase 1: Research (EXPLORE)\nRun the disciplined-research skill:\n  /disciplined-research\n\nProduce: .docs/research-issue-${SYMPHONY_ISSUE_NUMBER}.md\nContents:\n- Problem statement and success criteria\n- Current state analysis (existing code, data flows)\n- Risk assessment and unknowns\n- Dependencies analysis\n- Essential Questions Check (2/3 YES minimum)\n- Vital Few constraints (max 3)\n\nCommit the artefact before proceeding.\n\n### Phase 2: Design (ELIMINATE)\nRun the disciplined-design skill:\n  /disciplined-design\n\nProduce: .docs/design-issue-${SYMPHONY_ISSUE_NUMBER}.md\nContents:\n- Implementation plan (NOT code)\n- File changes (new, modified, deleted)\n- API design (public types, functions)\n- Test strategy\n- Implementation steps (sequenced)\n- 5/25 Rule applied (5 in scope, rest eliminated)\n- Eliminated Options documented\n- Simplicity Check answered\n\nCommit the artefact before proceeding.\n\n### Phase 3: Implementation (EXECUTE)\nRun the disciplined-implementation skill:\n  /disciplined-implementation\n\nFollow the design plan step by step:\n- Write tests FIRST at each step\n- Implement code to pass tests\n- Run cargo build, cargo test, cargo clippy, cargo fmt at each step\n- Log friction points\n- Commit at each step with message: feat(module): description (Refs ${SYMPHONY_ISSUE_IDENTIFIER})\n\n### Phase 4: Verification (RIGHT SIDE - verify code matches design)\nRun the disciplined-verification skill:\n  /disciplined-verification\n\nProduce: .docs/verification-issue-${SYMPHONY_ISSUE_NUMBER}.md\nContents:\n- Traceability matrix (requirement -> design -> code -> test)\n- Unit test results\n- Integration test results\n- Coverage metrics\n- Defect list with origins (loop back if needed)\n\nCommit the artefact before proceeding.\n\n### Phase 5: Validation (RIGHT SIDE - validate solution meets requirements)\nRun the disciplined-validation skill:\n  /disciplined-validation\n\nProduce: .docs/validation-issue-${SYMPHONY_ISSUE_NUMBER}.md\nContents:\n- System test results against acceptance criteria\n- Requirements compliance evidence\n- Final quality gate results\n- GO/NO-GO assessment\n\nCommit the artefact as final step.\n\n## Quality Requirements\n- cargo build must succeed\n- cargo test must pass (or cargo test --workspace for cross-crate issues)\n- cargo clippy -- -D warnings must pass\n- cargo fmt -- --check must pass\n- Use British English in documentation\n- Never use mocks in tests\n\n## Key Workspace Structure\n- crates/ -- 29+ library crates\n- terraphim_server/ -- Main HTTP server binary\n- desktop/ -- Svelte frontend (Tauri)\n- crates/terraphim_email_worker/ -- Private: email sending via Resend\n- crates/terraphim_symphony/ -- Excluded from workspace, builds independently\n\n## Important\n- Examine ALL existing code first. Previous agents may have done work.\n- Do NOT recreate files that already exist.\n- Build on what is there.\n- If a phase skill is not available, perform the phase manually following its principles.\nCLAUDEEOF\ngit add CLAUDE.md .docs/ && git commit -m 'chore: add agent CLAUDE.md with V-model workflow' --no-verify 2>/dev/null || true"
  before_run: "set -e && git fetch origin && BRANCH=\"symphony/issue-${SYMPHONY_ISSUE_NUMBER}\" && git checkout main 2>/dev/null || true && git branch -D \"$BRANCH\" 2>/dev/null || true && if git rev-parse --verify \"origin/$BRANCH\" >/dev/null 2>&1; then git checkout -b \"$BRANCH\" \"origin/$BRANCH\" && git fetch origin main && git rebase origin/main 2>/dev/null || { git rebase --abort 2>/dev/null; git merge -X theirs origin/main --no-edit 2>/dev/null || true; }; else git fetch origin main && git checkout -b \"$BRANCH\" origin/main; fi"
  after_run: "set +e && BRANCH=\"symphony/issue-${SYMPHONY_ISSUE_NUMBER}\" && GITEA_API=\"https://git.terraphim.cloud/api/v1/repos/zestic-ai/terraphim-private\" && AUTH=\"Authorization: token ${GITEA_TOKEN}\" && git add -A && git diff --cached --quiet || git commit -m \"symphony: ${SYMPHONY_ISSUE_IDENTIFIER} - ${SYMPHONY_ISSUE_TITLE}\" || true && git push -u origin \"$BRANCH\" 2>/dev/null || true && GATE=true && GATE_MSG=\"\" && RS_COUNT=$(find crates -name '*.rs' 2>/dev/null | wc -l | tr -d ' ') && if [ \"$RS_COUNT\" -lt 1 ]; then GATE=false && GATE_MSG=\"No .rs files in crates/. \"; fi && cargo build 2>/tmp/sym_build_${SYMPHONY_ISSUE_NUMBER}.log || { GATE=false; GATE_MSG=\"${GATE_MSG}Build failed. \"; } && cargo test 2>/tmp/sym_test_${SYMPHONY_ISSUE_NUMBER}.log || { GATE=false; GATE_MSG=\"${GATE_MSG}Tests failed. \"; } && cargo clippy -- -D warnings 2>/tmp/sym_clippy_${SYMPHONY_ISSUE_NUMBER}.log || { GATE=false; GATE_MSG=\"${GATE_MSG}Clippy failed. \"; } && cargo fmt -- --check 2>/tmp/sym_fmt_${SYMPHONY_ISSUE_NUMBER}.log || { GATE=false; GATE_MSG=\"${GATE_MSG}Format check failed. \"; } && VMODEL_MSG=\"\" && if [ -f \".docs/verification-issue-${SYMPHONY_ISSUE_NUMBER}.md\" ]; then VMODEL_MSG=\"V-model verification present. \"; else VMODEL_MSG=\"WARNING: No verification artefact. \"; fi && if [ -f \".docs/validation-issue-${SYMPHONY_ISSUE_NUMBER}.md\" ]; then VMODEL_MSG=\"${VMODEL_MSG}V-model validation present. \"; else VMODEL_MSG=\"${VMODEL_MSG}WARNING: No validation artefact. \"; fi && if [ \"$GATE\" = \"true\" ]; then MERGE_OK=false && git fetch origin main && git checkout main && git pull origin main && git merge -X theirs \"$BRANCH\" --no-ff -m \"Merge ${SYMPHONY_ISSUE_IDENTIFIER}: ${SYMPHONY_ISSUE_TITLE}\" && git push origin main && MERGE_OK=true; if [ \"$MERGE_OK\" = \"true\" ]; then curl --retry 3 --retry-delay 5 --max-time 30 -sf -X PATCH -H \"$AUTH\" -H 'Content-Type: application/json' -d '{\"state\":\"closed\"}' \"${GITEA_API}/issues/${SYMPHONY_ISSUE_NUMBER}\" >/dev/null || true; curl --retry 3 --retry-delay 5 --max-time 30 -sf -X POST -H \"$AUTH\" -H 'Content-Type: application/json' -d \"{\\\"body\\\":\\\"Quality gate passed (build+test+clippy+fmt). ${VMODEL_MSG}Merged to main and closed.\\\"}\" \"${GITEA_API}/issues/${SYMPHONY_ISSUE_NUMBER}/comments\" >/dev/null || true; else git checkout \"$BRANCH\" 2>/dev/null; curl --retry 2 --max-time 15 -sf -X POST -H \"$AUTH\" -H 'Content-Type: application/json' -d '{\"body\":\"Merge to main failed (concurrent merge race). Will retry.\"}' \"${GITEA_API}/issues/${SYMPHONY_ISSUE_NUMBER}/comments\" >/dev/null || true; fi; else curl --retry 2 --max-time 15 -sf -X POST -H \"$AUTH\" -H 'Content-Type: application/json' -d \"{\\\"body\\\":\\\"Quality gate failed: ${GATE_MSG}${VMODEL_MSG}Issue left open for retry.\\\"}\" \"${GITEA_API}/issues/${SYMPHONY_ISSUE_NUMBER}/comments\" >/dev/null || true; fi"
  timeout_ms: 600000

codex:
  turn_timeout_ms: 3600000
  stall_timeout_ms: 600000
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

## MANDATORY V-Model Workflow

You MUST follow the disciplined engineering V-model for this issue.
Read your CLAUDE.md for the full 5-phase workflow with skill invocations.

**Phase sequence**: Research -> Design -> Implement -> Verify -> Validate

Each phase produces a `.docs/{phase}-issue-{{ issue.identifier | split: "#" | last }}.md` artefact.

## CRITICAL Instructions

1. Read CLAUDE.md first for the full V-model workflow.
2. Examine ALL existing code in this workspace. Build on what is there.
3. Follow all 5 V-model phases in order.
4. Produce artefacts for each phase and commit them.
5. Ensure all quality gates pass before finishing.
6. Commit with message: feat(module): description (Refs {{ issue.identifier }})
7. Do NOT run gitea-robot, tea, or any issue-tracking commands.

{% if attempt %}
## RETRY ATTEMPT {{ attempt }}

This is retry attempt {{ attempt }}. Previous attempts did not pass the quality gate.
Check existing code and V-model artefacts from previous attempts.
Fix any build errors, test failures, or clippy warnings.
Focus on making `cargo build && cargo test && cargo clippy -- -D warnings && cargo fmt -- --check` all succeed.
If V-model artefacts exist from a previous attempt, build on them rather than starting over.
{% endif %}
