# Research Document: Teaching LLMs and Coding Agents Terraphim Capabilities

## 1. Problem Restatement and Scope

### Problem Statement
How can we systematically teach LLMs and coding agents (Claude Code, Cursor, Windsurf, Cline, etc.) to leverage Terraphim's semantic search, knowledge graph, and autocomplete capabilities through:
1. **Tool prompts** - Terraphim-specific tool definitions
2. **Hooks** - Pre-commit and pre-write message interception
3. **Capability injection** - Teaching agents new behaviors

### Use Cases to Validate
1. **npm → bun replacement**: `npm install` is always replaced by `bun install`
2. **Attribution replacement**: "Claude Code" attribution is always replaced by "Terraphim AI"

### IN Scope
- Claude Code hooks (PreToolUse, PostToolUse, user-prompt-submit)
- Pre-commit hooks for git operations
- Pre-write message interception
- Tool prompt patterns for MCP servers
- Self-documenting API patterns
- Agent capability injection via CLAUDE.md/AGENTS.md

### OUT of Scope
- Building new agent frameworks from scratch
- Non-Claude coding agents (except patterns applicable to all)
- Real-time streaming modifications (too complex for initial implementation)

## 2. User & Business Outcomes

### User-Visible Changes
1. **Automatic command replacement**: When agent writes `npm install`, it becomes `bun install` transparently
2. **Attribution correction**: Commit messages show "Terraphim AI" instead of "Claude Code"
3. **Knowledge-graph powered suggestions**: Autocomplete suggests domain-specific terms
4. **Semantic search integration**: Agents can search Terraphim's indexed knowledge

### Business Outcomes
- Consistent code standards enforcement across all AI-assisted development
- Brand attribution correction in generated content
- Knowledge graph-driven code quality improvements
- Reduced manual intervention for repetitive corrections

## 3. System Elements and Dependencies

### External Reference Systems Analyzed

#### Ultimate Bug Scanner (UBS)
| Element | Location | Role |
|---------|----------|------|
| Agent Detection | `install.sh` | Auto-detects Claude Code, Cursor, Windsurf, Cline, Codex |
| File-save Hook | `~/.claude/hooks/on-file-write.sh` | Triggers `ubs --ci` when Claude saves files |
| Rule Injection | `.cursor/rules`, agent-specific locations | Adds quality checks to agent workflows |
| Pre-commit Gate | Git hook | `ubs . --fail-on-warning` blocks buggy commits |
| Output Formats | CLI flags | JSON, JSONL, SARIF for machine-readable output |
| Easy Mode | `--easy-mode` flag | Zero-prompt agent integration |

**Key Pattern**: UBS uses **file-save hooks** and **rule injection** to teach agents to run quality checks.

#### Coding Agent Session Search (CASS)
| Element | Location | Role |
|---------|----------|------|
| Self-documenting API | `cass capabilities --json` | Feature discovery for agents |
| Introspection | `cass introspect --json` | Full schema + argument types |
| Robot Docs | `cass robot-docs commands` | LLM-optimized documentation |
| Forgiving Syntax | CLI parser | Normalizes typos (Levenshtein ≤2), teaches on correction |
| Structured Output | `--format json` | All results with `_meta` blocks |
| Token Budget | `--max-tokens N` | Controls output for LLM context limits |

**Key Pattern**: CASS uses **self-documenting APIs** and **forgiving syntax with teaching feedback**.

### Terraphim System Elements

| Element | Location | Role |
|---------|----------|------|
| MCP Server | `crates/terraphim_mcp_server/` | Exposes autocomplete, search, KG tools |
| TUI | `crates/terraphim_tui/` | CLI for replacements and REPL |
| Existing Hooks | `.claude/hooks/subagent-start.json` | Injects context on subagent start |
| Settings | `.claude/settings.local.json` | Permission allowlists |
| Integration Guide | `examples/TERRAPHIM_CLAUDE_INTEGRATION.md` | Hooks and skills documentation |
| Knowledge Graphs | `docs/src/kg/` | Markdown files defining synonyms |

### Dependencies

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Claude Code    │────▶│  Claude Hooks    │────▶│  Terraphim      │
│  (Agent)        │     │  (PreToolUse,    │     │  (MCP Server,   │
│                 │     │   user-prompt)   │     │   TUI)          │
└─────────────────┘     └──────────────────┘     └─────────────────┘
         │                       │                       │
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  CLAUDE.md      │     │  Pre-commit      │     │  Knowledge      │
│  (Instructions) │     │  Hooks           │     │  Graph Files    │
└─────────────────┘     └──────────────────┘     └─────────────────┘
```

## 4. Constraints and Their Implications

### Technical Constraints

| Constraint | Implication |
|------------|-------------|
| **Hook execution timeout** | 60 seconds max; must be fast (<100ms for good UX) |
| **JSON response format** | Hooks must output valid JSON with `permissionDecision` |
| **Restart required** | Claude Code snapshots hook config at startup |
| **Regex pattern matching** | Not a security boundary; determined agents can bypass |
| **Token budget** | Prompts must stay within context limits |

### Business Constraints

| Constraint | Implication |
|------------|-------------|
| **Transparency** | Users should know when replacements happen (optional logging) |
| **Reversibility** | Changes should be reviewable before commit |
| **Cross-platform** | Skills work everywhere; hooks are CLI-only |

### UX Constraints

| Constraint | Implication |
|------------|-------------|
| **Non-blocking** | Hooks should not slow down agent workflows |
| **Informative** | Blocked operations should explain alternatives |
| **Configurable** | Different modes (replace, suggest, passive) |

## 5. Risks, Unknowns, and Assumptions

### Unknowns
1. **Hook execution order**: If multiple hooks exist, which runs first?
2. **Hook composition**: Can hooks chain (one hook calls another)?
3. **Error propagation**: How do hook failures affect agent workflow?
4. **State persistence**: Can hooks maintain state across invocations?

### Assumptions
1. **ASSUMPTION**: Claude Code hooks API is stable and documented
2. **ASSUMPTION**: PreToolUse hook can intercept Bash commands containing npm/yarn
3. **ASSUMPTION**: Pre-commit hooks run before Claude sees commit results
4. **ASSUMPTION**: Terraphim MCP server can be queried from hook scripts

### Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| **Performance overhead** | Medium | Cache knowledge graph in memory; use fast FST matching |
| **False positives** | High | Whitelist patterns (e.g., "npm" in comments) |
| **Breaking changes** | Medium | Version hooks alongside Terraphim releases |
| **Agent bypass** | Low | Hooks are safety net, not security boundary |
| **Configuration complexity** | Medium | Provide `--easy-mode` for zero-config setup |

### De-risking Experiments
1. **Benchmark hook latency**: Measure terraphim-tui replace performance
2. **Test hook composition**: Try chaining multiple PreToolUse hooks
3. **Validate regex patterns**: Test against real npm/yarn command variations

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity
1. **Multiple hook types**: PreToolUse, PostToolUse, user-prompt-submit, file-write
2. **Multiple agents**: Claude Code, Cursor, Windsurf, Cline, Codex
3. **Multiple integration points**: Hooks, skills, MCP tools, CLAUDE.md
4. **Existing infrastructure**: Already have partial hook setup in `.claude/`

### Simplification Strategies

#### Strategy 1: Start with PreToolUse for Bash
Focus on single hook type that intercepts all package manager commands:
```
Bash("npm install") → Hook → Bash("bun install")
```

#### Strategy 2: Use Terraphim MCP as Single Source
All replacements go through MCP server; hooks are thin wrappers:
```bash
#!/bin/bash
INPUT=$(cat)
terraphim-mcp-client replace "$INPUT" || echo "$INPUT"
```

#### Strategy 3: Progressive Enhancement
1. **Phase 1**: PreToolUse hook for npm → bun (single use case)
2. **Phase 2**: Extend to commit message attribution
3. **Phase 3**: Add self-documenting API for discoverability
4. **Phase 4**: Agent rule injection for Cursor, Windsurf, etc.

### Recommended Simplification
**Start with Strategy 3** - Progressive enhancement from a working minimal implementation.

## 7. Questions for Human Reviewer

1. **Hook Priority**: Should npm→bun replacement happen at PreToolUse (before execution) or user-prompt-submit (before Claude sees it)?

2. **Attribution Scope**: Should "Claude Code" → "Terraphim AI" apply to:
   - Only commit messages?
   - All generated text?
   - Only specific file patterns?

3. **Failure Mode**: If terraphim-tui fails, should we:
   - Block the operation (fail-safe)?
   - Pass through unchanged (fail-open)?

4. **Cross-Agent Support**: Is supporting Cursor/Windsurf/Cline in scope for initial implementation?

5. **MCP vs TUI**: Should hooks call:
   - `terraphim-tui replace` (simple, file-based)?
   - MCP server via HTTP (richer, requires running server)?

6. **State Management**: Should hooks track:
   - Replacement statistics?
   - Blocked command history?
   - Learning/adaptation data?

7. **User Notification**: When a replacement happens, should we:
   - Log silently?
   - Show stderr notification?
   - Add comment to output?

8. **Testing Strategy**: How should we validate hook behavior:
   - Unit tests for replacement logic?
   - Integration tests with mock Claude?
   - E2E tests with real Claude Code?

9. **Distribution**: How should hooks be distributed:
   - Part of Terraphim codebase?
   - Separate claude-hooks package?
   - install.sh auto-detection (like UBS)?

10. **Version Compatibility**: How do we handle:
    - Claude Code API changes?
    - Terraphim version mismatches?
    - Breaking changes in hook format?

---

## Appendix: Key Patterns from Reference Systems

### Pattern 1: Self-Documenting APIs (from CASS)
```bash
terraphim-agent capabilities --json    # Feature discovery
terraphim-agent introspect --json      # Schema + types
terraphim-agent robot-docs             # LLM-optimized docs
```

### Pattern 2: Agent Detection (from UBS)
```bash
# Detect and configure agents
detect_claude_code() { ... }
detect_cursor() { ... }
detect_windsurf() { ... }
install_hooks_for_detected_agents()
```

### Pattern 3: Forgiving Syntax with Teaching (from CASS)
```
User types: "terraphim repalce"
System: "Did you mean 'replace'? [Auto-corrected]"
```

### Pattern 4: Quality Gate Integration (from UBS)
```bash
# Pre-commit hook
terraphim-agent validate . --fail-on-warning || exit 1
```

### Pattern 5: Structured Output for Agents (from CASS)
```json
{
  "result": "bun install",
  "_meta": {
    "original": "npm install",
    "replacements": 1,
    "time_ms": 12
  }
}
```
