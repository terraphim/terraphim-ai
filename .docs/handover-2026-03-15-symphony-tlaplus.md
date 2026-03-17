# Handover: Symphony tlaplus-ts Orchestration

**Date**: 2026-03-15 11:53 GMT
**Branch**: `main` (terraphim-ai), `main` (tlaplus-ts on Gitea)
**Status**: Complete -- all work delivered and verified

---

## Progress Summary

### Completed This Session

1. **Symphony WORKFLOW rewrite** (`WORKFLOW-tlaplus-ts.md`)
   - Fixed three root causes preventing code accumulation across agents:
     - Branch isolation: added merge-to-main in `after_run` so downstream agents inherit work
     - Unconditional auto-close: replaced with quality gate (build + test must pass)
     - Insufficient turns: increased from 15 to 50
   - Added retry support via `{% if attempt %}` Liquid template block

2. **Gitea dependency graph setup**
   - Deleted and recreated `terraphim/tlaplus-ts` repo with 8 issues
   - Added 12 dependency edges using corrected `gitea-robot add-dep` semantics
   - Fixed documentation: `--issue X --blocks Y` means "X is blocked by Y" (not "X blocks Y")
   - Documentation fixes applied to: `gitea-robot/skill/SKILL.md`, `skill/references/claude-md-snippet.md`, `~/.claude/CLAUDE.md`

3. **Symphony execution on bigbox**
   - All 8 issues completed in ~53 minutes with 2 concurrent agents
   - Dependency ordering verified: roots first (#1, #2), then cascading (#3 -> #4+#5 -> #6 -> #7 -> #8)
   - Merge race on #2 handled automatically by retry mechanism
   - Result: 26 TypeScript source files, 8 test files, 414 tests passing

4. **Verification and validation** (disciplined V-model Phases 4 + 5)
   - Build: PASS (CJS + ESM + DTS)
   - Tests: 414/414 passing
   - Coverage: 77% statement, 92% function
   - 5 defects found, all in CLI commands (Issue #7 agent)
   - Reports: `.docs/verification-report-tlaplus-ts.md`, `.docs/validation-report-tlaplus-ts.md`

5. **Defect fixes applied** (commit `f4632e4` on tlaplus-ts)
   - D1: `check.ts` -- fixed `bridge.check()` call signature (2 args -> 3 args)
   - D2a: `check.ts` -- `duration` -> `durationSeconds`
   - D2b: `check.ts` -- `state.number` -> `state.num`
   - D3: `format.ts` -- added null guard for `result.module`
   - D4: `validate.ts` -- added null guard for `result.module`
   - D5: `.gitignore` -- removed 5 duplicate `CLAUDE.md` entries
   - Post-fix: zero TypeScript errors, 414/414 tests passing, build succeeds

### Nothing Blocked

All work is complete and pushed.

---

## Technical Context

### Repositories

| Repo | Location | State |
|------|----------|-------|
| terraphim-ai | `/Users/alex/projects/terraphim/terraphim-ai` (local) | Clean on `main`, 2 untracked report files |
| tlaplus-ts | `https://git.terraphim.cloud/terraphim/tlaplus-ts` | All 8 issues closed, `main` has all merged work + defect fixes |
| tlaplus-ts (bigbox clone) | `/tmp/tlaplus-ts-verify/` on bigbox | Used for verification and fix commits |

### Key Commits

**terraphim-ai** (`main`):
```
73e6b4c3 feat(symphony): rewrite WORKFLOW with quality gate and merge-to-main
```

**tlaplus-ts** (`main` on Gitea):
```
f4632e4 fix(cli): resolve TypeScript strict-mode errors in CLI commands
e444f35 Merge #8: docs and npm publishing
1132049 Merge #7: CLI commands
9366879 Merge #5: formatter
...8 merge commits total from Symphony agents
```

### tlaplus-ts Architecture (as built)

```
src/
  parser/       -- tree-sitter wrapper, CST-to-AST transform (Issue #3)
  ast/          -- TypeScript type definitions for TLA+ AST (Issue #2)
  eval/         -- Expression evaluator: sets, logic, functions (Issue #4)
  format/       -- Pretty-printer (Issue #5)
  bridge/       -- TLC model checking bridge via Java CLI (Issue #6)
  cli/          -- CLI: parse|format|validate|check (Issue #7)
  index.ts      -- Public API exports (Issue #1 scaffold)
test/
  8 test files, 414 tests
```

### Symphony Configuration

- **WORKFLOW file**: `crates/terraphim_symphony/examples/WORKFLOW-tlaplus-ts.md`
- **max_turns**: 50 per agent
- **max_concurrent_agents**: 2
- **Quality gate**: checks for .ts files, build pass, test pass before merging to main
- **Retry**: Symphony re-dispatches if issue stays open (quality gate failed)

### Untracked Files (terraphim-ai)

```
.docs/validation-report-tlaplus-ts.md   -- Phase 5 validation report
.docs/verification-report-tlaplus-ts.md -- Phase 4 verification report
.cachebro/                              -- cache directory (ignorable)
```

These reports are informational and can be committed if desired.

---

## Lessons Learned

1. **gitea-robot `add-dep` semantics**: `--issue X --blocks Y` means "X is blocked by Y" (Y blocks X). Documentation was wrong and has been corrected.
2. **Symphony merge-to-main pattern works**: downstream agents successfully inherited accumulated code via `git fetch origin main` + rebase in `before_run`.
3. **Quality gate prevents false closes**: issues only close when build + tests pass, preventing the zero-code-output problem from previous runs.
4. **CLI agents produce most defects**: all 5 defects came from Issue #7 (CLI). Core library (parser, AST, eval, format, bridge) was defect-free.
5. **Agent CLAUDE.md should be minimal**: stripping task-tracking commands from agent instructions leaves more turns for actual coding.

---

## Potential Follow-up Work

- **Commit the verification/validation reports** to terraphim-ai if desired
- **Publish tlaplus-ts to npm** -- the package.json is configured, just needs `npm publish`
- **Add CI/CD** to tlaplus-ts repo (GitHub Actions or Gitea runner)
- **Clean up symphony/* branches** on tlaplus-ts remote (8 branches from completed issues)
- **Run Symphony on other projects** using the proven WORKFLOW pattern
