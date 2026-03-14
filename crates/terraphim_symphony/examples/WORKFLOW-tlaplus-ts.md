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
  max_turns: 15
  claude_flags: "--dangerously-skip-permissions --allowedTools Bash,Read,Write,Edit,Glob,Grep"
  settings: ~/.claude/symphony-settings.json

workspace:
  root: ~/symphony_workspaces

hooks:
  after_create: "git clone https://terraphim:${GITEA_TOKEN}@git.terraphim.cloud/terraphim/tlaplus-ts.git ."
  before_run: "git fetch origin && git checkout main && git pull"
  after_run: "git add -A && git commit -m 'symphony: auto-commit' && git push || true"
  timeout_ms: 120000

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
**Runtime**: Node.js 18+
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

## Instructions

1. Read the issue carefully.
2. Examine existing code in this workspace.
3. Implement the required changes following TypeScript best practices.
4. Write comprehensive tests using vitest.
5. Ensure all existing tests still pass: `npx vitest run`
6. Ensure the build succeeds: `npm run build`
7. Ensure lint passes: `npm run lint`
8. Commit with a message referencing {{ issue.identifier }}.

{% if attempt %}
This is retry attempt {{ attempt }}. Review previous work and continue from where it left off.
{% endif %}
