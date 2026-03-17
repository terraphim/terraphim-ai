---
tracker:
  kind: gitea
  endpoint: https://git.terraphim.cloud
  api_key: $GITEA_TOKEN
  owner: terraphim
  repo: tlaplus-ts

polling:
  interval_ms: 30000

agent:
  runner: claude-code
  max_concurrent_agents: 2
  max_turns: 50
  claude_flags: "--dangerously-skip-permissions --allowedTools Bash,Read,Write,Edit,Glob,Grep"
  settings: ~/.claude/symphony-settings.json

workspace:
  root: ~/symphony_workspaces

hooks:
  after_create: "git clone https://terraphim:${GITEA_TOKEN}@git.terraphim.cloud/terraphim/tlaplus-ts.git . && echo 'CLAUDE.md' >> .gitignore && cat > CLAUDE.md << 'CLAUDEEOF'\n# tlaplus-ts Agent Instructions\n\n## Your Task\nYou are implementing a specific issue for the tlaplus-ts TypeScript library.\nFocus ALL your effort on writing production-quality source code and tests.\nDo NOT run gitea-robot, tea, or any task-tracking commands.\nThe orchestrator handles all task tracking.\n\n## Project Setup\n- Runtime: Node.js 20, npm 10\n- Test framework: vitest\n- Build: tsup (CJS + ESM bundles)\n- TypeScript 5.x strict mode, ESM modules\n- Use British English in documentation\n\n## Key Dependencies\n- tree-sitter and @tlaplus/tree-sitter-tlaplus for parsing\n- vitest for testing\n- tsup for bundling\n- typescript 5.x\n\n## Quality Requirements\n- Every public function must have JSDoc documentation\n- Every module must have at least one test file\n- All tests must pass: npx vitest run\n- Build must succeed: npm run build\n- Never use mocks in tests\nCLAUDEEOF\nnpm install 2>/dev/null || true"
  before_run: "set -e && git fetch origin && BRANCH=\"symphony/issue-${SYMPHONY_ISSUE_NUMBER}\" && if git rev-parse --verify \"origin/$BRANCH\" >/dev/null 2>&1; then git checkout \"$BRANCH\" 2>/dev/null || git checkout -b \"$BRANCH\" \"origin/$BRANCH\" && git pull origin \"$BRANCH\" --rebase 2>/dev/null || true && git fetch origin main && git rebase origin/main 2>/dev/null || { git rebase --abort 2>/dev/null; git merge origin/main --no-edit 2>/dev/null || true; }; else git fetch origin main && git checkout -b \"$BRANCH\" origin/main; fi && if [ -f package.json ] && [ ! -d node_modules ]; then npm install 2>/dev/null || true; fi"
  after_run: "set +e && BRANCH=\"symphony/issue-${SYMPHONY_ISSUE_NUMBER}\" && GITEA_API=\"https://git.terraphim.cloud/api/v1/repos/terraphim/tlaplus-ts\" && AUTH=\"Authorization: token ${GITEA_TOKEN}\" && git add -A && git diff --cached --quiet || git commit -m \"symphony: ${SYMPHONY_ISSUE_IDENTIFIER} - ${SYMPHONY_ISSUE_TITLE}\" || true && git push -u origin \"$BRANCH\" 2>/dev/null || true && GATE=true && GATE_MSG=\"\" && TS_COUNT=$(find src -name '*.ts' 2>/dev/null | wc -l | tr -d ' ') && if [ \"$TS_COUNT\" -lt 1 ]; then GATE=false && GATE_MSG=\"No .ts files in src/. \"; fi && if [ -f package.json ] && grep -q '\"build\"' package.json; then npm run build 2>/tmp/sym_build_${SYMPHONY_ISSUE_NUMBER}.log || { GATE=false; GATE_MSG=\"${GATE_MSG}Build failed. \"; }; fi && TEST_COUNT=$(find test tests -name '*.test.ts' -o -name '*.spec.ts' 2>/dev/null | wc -l | tr -d ' ') && if [ \"$TEST_COUNT\" -gt 0 ]; then npx vitest run 2>/tmp/sym_test_${SYMPHONY_ISSUE_NUMBER}.log || { GATE=false; GATE_MSG=\"${GATE_MSG}Tests failed. \"; }; fi && if [ \"$GATE\" = \"true\" ]; then MERGE_OK=false && git fetch origin main && git checkout main && git pull origin main && git merge \"$BRANCH\" --no-ff -m \"Merge ${SYMPHONY_ISSUE_IDENTIFIER}: ${SYMPHONY_ISSUE_TITLE}\" && git push origin main && MERGE_OK=true; if [ \"$MERGE_OK\" = \"true\" ]; then curl --retry 3 --retry-delay 5 --max-time 30 -sf -X PATCH -H \"$AUTH\" -H 'Content-Type: application/json' -d '{\"state\":\"closed\"}' \"${GITEA_API}/issues/${SYMPHONY_ISSUE_NUMBER}\" >/dev/null || true; curl --retry 3 --retry-delay 5 --max-time 30 -sf -X POST -H \"$AUTH\" -H 'Content-Type: application/json' -d '{\"body\":\"Quality gate passed. Merged to main and closed.\"}' \"${GITEA_API}/issues/${SYMPHONY_ISSUE_NUMBER}/comments\" >/dev/null || true; rm -rf node_modules 2>/dev/null || true; else git checkout \"$BRANCH\" 2>/dev/null; curl --retry 2 --max-time 15 -sf -X POST -H \"$AUTH\" -H 'Content-Type: application/json' -d '{\"body\":\"Merge to main failed (concurrent merge race). Will retry.\"}' \"${GITEA_API}/issues/${SYMPHONY_ISSUE_NUMBER}/comments\" >/dev/null || true; fi; else curl --retry 2 --max-time 15 -sf -X POST -H \"$AUTH\" -H 'Content-Type: application/json' -d \"{\\\"body\\\":\\\"Quality gate failed: ${GATE_MSG}Issue left open for retry.\\\"}\" \"${GITEA_API}/issues/${SYMPHONY_ISSUE_NUMBER}/comments\" >/dev/null || true; fi"
  timeout_ms: 300000

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

This is a TypeScript library providing bindings for TLA+ formal specifications.
The library uses tree-sitter-tlaplus for parsing and provides typed AST,
expression evaluation, formatting, and TLC model checking bridge.

**Package name**: `@terraphim/tlaplus`
**Runtime**: Node.js 20+
**Test framework**: vitest
**Build tool**: tsup (CJS + ESM bundles)

### Key dependencies
- `@tlaplus/tree-sitter-tlaplus` (npm) -- TLA+ parser grammar
- `tree-sitter` (npm) -- parser runtime
- `vitest` -- testing framework
- `tsup` -- TypeScript bundler
- `typescript` -- 5.x with strict mode

### Reference implementations to study
- Spectacle (github.com/will62794/spectacle) -- JS TLA+ interpreter, especially js/eval.js
- Quint (github.com/informalsystems/quint) -- TS TLA toolchain architecture

### Project structure
```
src/
  parser/       -- tree-sitter wrapper, CST-to-AST transform
  ast/          -- TypeScript type definitions for TLA+ AST
  eval/         -- Expression evaluator (sets, logic, functions)
  format/       -- Pretty-printer
  bridge/       -- TLC model checking bridge (Java CLI)
  cli/          -- CLI tool (parse|format|validate|check)
  index.ts      -- Public API exports
test/
  fixtures/     -- TLA+ spec fixtures
  *.test.ts     -- vitest test files
```

## CRITICAL Instructions

1. Run `npm install` if node_modules is missing.
2. Examine ALL existing code in this workspace first. Previous agents have already built parts of this project. Do NOT recreate files that already exist. Build on what is there.
3. Implement the feature described in the issue with REAL, COMPLETE source code in the `src/` directory.
4. Write comprehensive tests using vitest in the `test/` directory (never use mocks).
5. Ensure `npm run build` succeeds before finishing.
6. Ensure `npx vitest run` passes before finishing.
7. Commit with message: feat(module): description (Refs {{ issue.identifier }})
8. Do NOT run gitea-robot, tea, or any issue-tracking commands -- the orchestrator handles that.

{% if attempt %}
## RETRY ATTEMPT {{ attempt }}

This is retry attempt {{ attempt }}. Previous attempts did not pass the quality gate.
Check the existing code, fix any build errors, fix any test failures.
Focus on making `npm run build` and `npx vitest run` succeed.
{% endif %}
