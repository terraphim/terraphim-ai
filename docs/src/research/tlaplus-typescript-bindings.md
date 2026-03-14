# Research Document: TypeScript Bindings for TLA+

**Status**: Draft
**Author**: Terraphim AI
**Date**: 2026-03-14
**Reviewers**: Alex

## Executive Summary

This research examines the TLA+ toolchain ecosystem to determine the optimal approach for creating TypeScript bindings. The ecosystem already contains significant TypeScript/JavaScript implementations: tree-sitter-tlaplus (npm parser), Spectacle (full JS interpreter ~5000 LOC), Quint (TypeScript-native TLA alternative by Informal Systems), and vscode-tlaplus (TypeScript IDE). Rather than binding the Java-based SANY/TLC directly, the most productive approach is to build a TypeScript-native TLA+ toolkit layering on tree-sitter-tlaplus for parsing and drawing architectural patterns from Spectacle's interpreter.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | Formal verification tooling in TypeScript enables broader adoption of TLA+ for distributed systems design |
| Leverages strengths? | Yes | Symphony orchestrator can decompose this into parallel agent tasks; Terraphim hooks ensure quality |
| Meets real need? | Yes | TLA+ tooling is Java-heavy; TypeScript developers need native access for CI/CD integration, VS Code tooling, and web-based model checking |

**Proceed**: Yes (3/3 YES)

## Problem Statement

### Description

The TLA+ formal specification language toolchain is primarily Java-based (SANY parser, TLC model checker, PlusCal translator). TypeScript/JavaScript developers wanting to integrate TLA+ into their workflows must either shell out to Java processes or use incomplete community tools. We need a cohesive TypeScript library that provides parsing, AST manipulation, basic evaluation, and integration with the existing TLA+ ecosystem.

### Impact

- TypeScript developers building distributed systems (blockchain, databases, messaging) lack native TLA+ tooling
- CI/CD pipelines cannot easily integrate TLA+ verification without Java dependencies
- Web-based TLA+ tools (like Spectacle) are standalone; no reusable library exists
- The VS Code extension uses regex-based parsing instead of proper AST analysis

### Success Criteria

1. Published npm package `@terraphim/tlaplus` with TypeScript types
2. Parse TLA+ specs into typed AST using tree-sitter-tlaplus
3. Basic expression evaluator for TLA+ operators
4. PlusCal-to-TLA+ awareness (parse PlusCal blocks)
5. CLI tool for spec validation and formatting
6. All work implemented via Symphony-orchestrated Claude Code agents on bigbox

## Current State Analysis

### Existing Implementations

| Component | Language | Purpose | Maturity |
|-----------|----------|---------|----------|
| SANY | Java | Parser + semantic analysis | Production (20+ years) |
| TLC | Java | Explicit-state model checker | Production |
| tree-sitter-tlaplus | C/JS | Incremental parser grammar | Stable (v1.5.0, npm published) |
| Spectacle | JavaScript | Browser-based interpreter + model checker | Active (~5000 LOC) |
| Quint | TypeScript | Alternative TLA syntax with full toolchain | Active (v0.31.0, 1.2k stars) |
| vscode-tlaplus | TypeScript | VS Code extension | Production (97% TypeScript) |
| tlaplus-formatter | Java | Code formatter using SANY | Active |
| Apalache | Scala | Symbolic model checker (SMT-based) | Production |

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| tree-sitter-tlaplus | `github.com/tlaplus-community/tree-sitter-tlaplus` | Parser grammar, npm: `@tlaplus/tree-sitter-tlaplus` |
| Spectacle | `github.com/will62794/spectacle` | JS interpreter in `js/eval.js` |
| Quint | `github.com/informalsystems/quint` | Full TS toolchain: parser, type checker, simulator, model checker |
| vscode-tlaplus | `github.com/tlaplus/vscode-tlaplus` | VS Code extension (TypeScript) |
| SANY new API | `github.com/tlaplus/tlaplus/pull/1125` | Programmatic Java API (merged Feb 2025) |
| TLA+ tools | `github.com/tlaplus/tlaplus` | Main repo: SANY, TLC, PlusCal, tla2tex |

### Data Flow

```
TLA+ Source (.tla)
    |
    v
tree-sitter-tlaplus -----> Concrete Syntax Tree (CST)
    |                           |
    v                           v
CST-to-AST transform       tree-sitter queries
    |                       (highlights, folds)
    v
Typed TLA+ AST
    |
    +---> Evaluator (operators, expressions)
    |
    +---> Formatter (pretty-print)
    |
    +---> Validator (type checking, static analysis)
    |
    +---> TLC bridge (shell out to Java for model checking)
```

### Integration Points

- **tree-sitter WASM**: Pre-built WASM binary in npm package enables browser usage
- **tree-sitter Node.js bindings**: Native C addon for server-side parsing
- **SANY XML export**: `tla2sany.xml.XMLExporter` outputs AST as XML (can be consumed from TypeScript)
- **SANY new programmatic API**: `ParserInterface.java` (merged Feb 2025) provides cleaner Java API
- **TLC CLI**: `java -cp tla2tools.jar tlc2.TLC spec.tla` for model checking
- **Apalache CLI**: Symbolic model checking via command-line

## Constraints

### Technical Constraints

- **tree-sitter-tlaplus is syntax-only**: No semantic analysis, no type checking. CST needs transformation to useful AST
- **SANY requires Java 11+**: Cannot run natively in Node.js; must shell out or use GraalVM
- **Spectacle is not a library**: Standalone web app, not published as npm package; code extraction needed
- **Quint is a different language**: Similar semantics but different syntax; not TLA+ compatible at source level
- **WASM tree-sitter**: Works in browser but has different API than Node.js bindings

### Business Constraints

- Implementation via Symphony on bigbox (Claude Code agents)
- Output to Gitea issues at `git.terraphim.cloud`
- Must use all hooks (PreToolUse, PostToolUse) for quality assurance
- Issues should be decomposable into independent parallel tasks where possible

### Non-Functional Requirements

| Requirement | Target | Rationale |
|-------------|--------|-----------|
| Parse time | < 50ms for typical spec | Interactive editing feedback |
| Bundle size | < 500KB (WASM parser) | Browser deployment |
| Node.js support | 18+ | Current LTS |
| Browser support | Chrome 90+, Firefox 90+ | tree-sitter WASM requirements |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| tree-sitter-tlaplus as parser | Only production-ready JS-compatible TLA+ parser; proven in Spectacle | npm: `@tlaplus/tree-sitter-tlaplus`, used by Spectacle browser interpreter |
| TypeScript-first API | Target audience is TS developers; type safety is the value proposition | Quint proves TS-native TLA tooling is viable and desired (1.2k stars) |
| Decomposable into independent issues | Symphony orchestrates parallel agents; each issue must be self-contained | Symphony's workspace isolation requires independent tasks |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Full model checker in TypeScript | TLC/Apalache exist; not in top 5 priorities; Spectacle already has basic safety checking |
| Quint compatibility layer | Different language syntax; interop would be a separate project |
| GraalVM/SANY Java bridge | Too complex for initial delivery; shell-out to `java -cp tla2tools.jar` suffices |
| TLAPS proof system integration | Niche use case; separate project |
| Eclipse Toolbox integration | Legacy IDE being replaced by VS Code extension |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| Symphony orchestrator | Required for agent dispatch | Low - already production-proven with PageRank Viewer |
| bigbox server | Hosts Claude Code agents | Low - infrastructure in place |
| Gitea instance | Issue tracking and output | Low - running at git.terraphim.cloud |
| Claude Code hooks | Quality assurance | Low - already configured |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `@tlaplus/tree-sitter-tlaplus` | 1.5.0 | Low - stable, maintained | Fork grammar; but unnecessary |
| `tree-sitter` (Node.js) | Latest | Low - widely used | `web-tree-sitter` for browser |
| `web-tree-sitter` | Latest | Low - official WASM bindings | Node.js native bindings |
| Java 11+ (for TLC bridge) | 11+ | Medium - optional runtime dep | Skip model checking bridge |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| tree-sitter CST-to-AST transform complexity | Medium | High | Study Spectacle's `eval.js` for patterns; it already does this |
| TLA+ operator semantics are complex | High | Medium | Start with subset (sets, functions, logic); expand incrementally |
| Symphony agent coordination for dependent issues | Low | Medium | Design issues with clear boundaries; use `Refs #N` for dependencies |
| tree-sitter API differences (Node vs WASM) | Low | Low | Abstract behind unified parser interface |

### Open Questions

1. **Scope of evaluator**: Should we evaluate full TLA+ expressions or just a useful subset? -- Recommend: subset covering sets, functions, records, sequences, logic operators
2. **Formatter approach**: Implement our own or wrap tlaplus-formatter (Java)? -- Recommend: own implementation using CST manipulation
3. **Gitea repository**: Create new repo or use existing? -- Recommend: new repo `terraphim/tlaplus-ts`

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| tree-sitter-tlaplus grammar is complete enough for our needs | Used by Spectacle for full interpretation | Would need to extend grammar; manageable | Yes - Spectacle proves it |
| Spectacle's eval.js patterns are extractable | Open source, ~5000 LOC vanilla JS | Code may be tightly coupled to UI; refactoring needed | Partially - architecture looks modular |
| Symphony can handle 6-10 parallel issues | Successfully ran 6 issues for PageRank Viewer | Proven at this scale | Yes |
| bigbox has sufficient resources for parallel agents | Ran 2 concurrent agents successfully | May need to increase `max_concurrent_agents` | Yes |

## Research Findings

### Key Insights

1. **The ecosystem is richer than expected**: Three independent TypeScript/JavaScript TLA+ implementations exist (tree-sitter-tlaplus, Spectacle, Quint). This validates the approach and provides reference implementations.

2. **tree-sitter-tlaplus is the right parser foundation**: Published on npm, used by Spectacle for a full interpreter, supports both Node.js and WASM. The grammar covers TLA+ and PlusCal.

3. **Spectacle proves a JS TLA+ interpreter is feasible in ~5000 LOC**: Will Schultz's implementation parses TLA+ via tree-sitter and evaluates expressions, including basic safety checking (BFS state space exploration). This is the closest reference architecture.

4. **Quint shows TypeScript-native TLA tooling is desired**: 1.2k GitHub stars, actively developed, published on npm. However, Quint uses different syntax -- it's an alternative to TLA+, not a binding for TLA+.

5. **SANY's new programmatic API (PR #1125, merged Feb 2025)** makes Java interop cleaner, but for TypeScript bindings the tree-sitter approach avoids Java entirely.

6. **The VS Code extension is 97% TypeScript** but uses regex-based parsing. Migrating to tree-sitter-tlaplus would be a natural improvement, and our library could enable this.

### Relevant Prior Art

| Project | Relevance |
|---------|-----------|
| [Spectacle](https://github.com/will62794/spectacle) | Full JS TLA+ interpreter; reference for CST-to-AST and evaluation |
| [Quint](https://github.com/informalsystems/quint) | TypeScript TLA toolchain; reference for architecture (parser, type checker, simulator, REPL, LSP) |
| [tree-sitter-tlaplus](https://github.com/tlaplus-community/tree-sitter-tlaplus) | Parser foundation; npm package `@tlaplus/tree-sitter-tlaplus` |
| [vscode-tlaplus](https://github.com/tlaplus/vscode-tlaplus) | TypeScript IDE integration; potential consumer of our library |
| [tlaplus-formatter](https://github.com/tlaplus/tlaplus-formatter) | Java formatter; reference for formatting rules |

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| tree-sitter-tlaplus Node.js hello world | Verify parsing API, understand CST structure | 1 issue |
| Spectacle eval.js extraction | Understand evaluation patterns, identify extractable code | 1 issue |
| TLC CLI bridge prototype | Verify `java -cp tla2tools.jar tlc2.TLC` works from Node.js child_process | 1 issue |

## Recommendations

### Proceed/No-Proceed

**Proceed** -- The ecosystem validation is strong. Three independent implementations prove feasibility. tree-sitter-tlaplus provides a solid parser foundation. Symphony can orchestrate parallel implementation.

### Scope Recommendations

**Phase 1 (MVP -- 6-8 Gitea issues for Symphony):**

1. **Project scaffold**: TypeScript project with tsconfig, eslint, vitest, package.json
2. **Parser wrapper**: Typed wrapper around tree-sitter-tlaplus with CST-to-AST transform
3. **AST types**: TypeScript type definitions for TLA+ AST nodes (modules, operators, expressions, PlusCal)
4. **Basic evaluator**: Evaluate constant expressions (sets, functions, records, sequences, logic)
5. **Formatter**: Pretty-print TLA+ from AST using CST manipulation
6. **CLI tool**: `tlaplus-ts parse|format|validate <file.tla>`
7. **TLC bridge**: Shell out to Java TLC for model checking, parse results
8. **Documentation and examples**: README, API docs, example specs

**Phase 2 (Future):**
- LSP server for VS Code integration
- WASM build for browser usage
- Advanced evaluator (temporal operators, state exploration)
- Apalache integration

### Risk Mitigation Recommendations

1. **Start with parsing**: The parser (tree-sitter) is proven; get AST types right first
2. **Study Spectacle before evaluator**: Extract patterns from `js/eval.js` before writing evaluator
3. **Independent issues**: Each issue should produce a testable, committable artefact
4. **Use conformance tests**: Compare output against TLC/SANY for correctness validation

## Proposed Gitea Issue Decomposition

| # | Title | Dependencies | Parallel Group |
|---|-------|-------------|----------------|
| 1 | Scaffold TypeScript project with tsconfig, vitest, eslint | None | A |
| 2 | Create TLA+ AST type definitions | None | A |
| 3 | Implement tree-sitter-tlaplus parser wrapper with CST-to-AST | #1, #2 | B |
| 4 | Implement basic expression evaluator (sets, logic, functions) | #2, #3 | C |
| 5 | Implement TLA+ formatter (pretty-printer) | #3 | C |
| 6 | Implement TLC CLI bridge (model checking via Java) | #1 | B |
| 7 | Create CLI tool (parse, format, validate subcommands) | #3, #4, #5, #6 | D |
| 8 | Documentation, examples, and npm publish preparation | #7 | D |

**Symphony dispatch order**: A (parallel) -> B (parallel) -> C (parallel) -> D (parallel)

### WORKFLOW.md Configuration

```yaml
---
tracker:
  kind: gitea
  endpoint: https://git.terraphim.cloud
  api_key: $GITEA_TOKEN
  owner: terraphim
  repo: tlaplus-ts

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

Key dependencies:
- @tlaplus/tree-sitter-tlaplus (npm) -- parser grammar
- tree-sitter (npm) -- parser runtime
- vitest -- testing framework

Reference implementations to study:
- Spectacle (github.com/will62794/spectacle) -- JS TLA+ interpreter
- Quint (github.com/informalsystems/quint) -- TS TLA toolchain

## Instructions

1. Read the issue carefully.
2. Examine existing code in this workspace.
3. Implement the required changes following TypeScript best practices.
4. Write comprehensive tests using vitest.
5. Ensure all existing tests still pass.
6. Commit with a message referencing {{ issue.identifier }}.

{% if attempt %}
This is retry attempt {{ attempt }}. Review previous work and continue.
{% endif %}
```

## Next Steps

If approved:
1. Create Gitea repository `terraphim/tlaplus-ts` at `git.terraphim.cloud`
2. Create 8 Gitea issues with descriptions, dependencies, and labels
3. Set up dependency graph using `gitea-robot add-dep`
4. Deploy WORKFLOW.md to bigbox
5. Run Symphony to dispatch agents

## Appendix

### Sources

- [TLA+ main repository](https://github.com/tlaplus/tlaplus)
- [TLA+ codebase architecture](https://docs.tlapl.us/codebase:architecture)
- [Current state of TLA+ development (2025)](https://ahelwer.ca/post/2025-05-15-tla-dev-status/)
- [tree-sitter-tlaplus](https://github.com/tlaplus-community/tree-sitter-tlaplus)
- [tree-sitter-tlaplus npm](https://www.npmjs.com/package/@tlaplus/tree-sitter-tlaplus)
- [Spectacle -- TLA+ web explorer](https://github.com/will62794/spectacle)
- [Quint -- TypeScript TLA specification language](https://github.com/informalsystems/quint)
- [vscode-tlaplus](https://github.com/tlaplus/vscode-tlaplus)
- [TLA+ formatter](https://github.com/tlaplus/tlaplus-formatter)
- [SANY programmatic API PR #1125](https://github.com/tlaplus/tlaplus/pull/1125)
- [Apalache symbolic model checker](https://github.com/apalache-mc/apalache)
- [TLA+ Foundation](https://foundation.tlapl.us/)
