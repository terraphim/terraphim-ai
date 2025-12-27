# Design & Implementation Plan: Teaching LLMs and Coding Agents Terraphim Capabilities

## 1. Summary of Target Behavior

After implementation, the system will:

1. **PreToolUse Hook (npm → bun)**: Intercept Bash commands containing `npm install`, `yarn install`, or `pnpm install` and automatically replace them with `bun install` BEFORE Claude executes the command.

2. **Pre-commit Hook (Attribution)**: Intercept commit messages containing "Claude Code" or "Claude" and replace with "Terraphim AI" BEFORE the commit is finalized.

3. **MCP Tool Prompts**: Provide self-documenting tool definitions that teach agents about Terraphim's autocomplete, semantic search, and knowledge graph capabilities.

### Workflow Diagrams

**Use Case 1: npm → bun Replacement**
```
Claude Code                    PreToolUse Hook                  Bash
    │                              │                              │
    │ Bash("npm install")          │                              │
    │─────────────────────────────▶│                              │
    │                              │ terraphim-tui replace        │
    │                              │ "npm install" → "bun install"│
    │                              │                              │
    │ ◀───── modified command ─────│                              │
    │        "bun install"         │                              │
    │                              │                              │
    │ Bash("bun install")          │                              │
    │─────────────────────────────────────────────────────────────▶│
    │                              │                              │
```

**Use Case 2: Attribution Replacement**
```
Claude Code                    Pre-commit Hook                  Git
    │                              │                              │
    │ git commit -m "...Claude..." │                              │
    │─────────────────────────────────────────────────────────────▶│
    │                              │                              │
    │                              │◀── prepare-commit-msg ───────│
    │                              │ terraphim-tui replace        │
    │                              │ "Claude" → "Terraphim AI"    │
    │                              │─── modified message ─────────▶│
    │                              │                              │
    │ ◀───────────────── commit success ──────────────────────────│
```

## 2. Key Invariants and Acceptance Criteria

### Invariants

| Invariant | Guarantee |
|-----------|-----------|
| **Performance** | Hook execution < 100ms (no user-perceived delay) |
| **Fail-open** | If terraphim-tui fails, original command passes through |
| **Idempotency** | Multiple applications produce same result |
| **Transparency** | Replacements logged to stderr (optional, configurable) |
| **Non-destructive** | Original input recoverable from logs |

### Acceptance Criteria

| ID | Criterion | Testable? |
|----|-----------|-----------|
| AC1 | `npm install` in Bash command → `bun install` before execution | Yes |
| AC2 | `yarn install` in Bash command → `bun install` before execution | Yes |
| AC3 | `pnpm install` in Bash command → `bun install` before execution | Yes |
| AC4 | "Claude Code" in commit message → "Terraphim AI" after commit | Yes |
| AC5 | "Claude" alone in commit message → "Terraphim AI" after commit | Yes |
| AC6 | Hook failure does not block command execution | Yes |
| AC7 | Replacements logged when TERRAPHIM_VERBOSE=1 | Yes |
| AC8 | MCP tools discoverable via `tools/list` | Yes |
| AC9 | Hook execution completes in < 100ms | Yes |

## 3. High-Level Design and Boundaries

### Architecture Overview

```
┌───────────────────────────────────────────────────────────────────┐
│                        Claude Code Agent                          │
├───────────────────────────────────────────────────────────────────┤
│                                                                   │
│   ┌─────────────────┐    ┌─────────────────┐    ┌──────────────┐ │
│   │ .claude/        │    │ .claude/hooks/  │    │ MCP Server   │ │
│   │ settings.json   │    │                 │    │ (via stdio)  │ │
│   └────────┬────────┘    └────────┬────────┘    └──────┬───────┘ │
│            │                      │                     │         │
│            ▼                      ▼                     ▼         │
│   ┌─────────────────┐    ┌─────────────────┐    ┌──────────────┐ │
│   │ Permission      │    │ PreToolUse      │    │ Tool Prompts │ │
│   │ Allowlists      │    │ npm_to_bun.py   │    │ (autocomplete│ │
│   └─────────────────┘    └─────────────────┘    │  search, kg) │ │
│                                 │                └──────────────┘ │
└─────────────────────────────────┼─────────────────────────────────┘
                                  │
                                  ▼
┌───────────────────────────────────────────────────────────────────┐
│                        Terraphim Layer                            │
├───────────────────────────────────────────────────────────────────┤
│   ┌─────────────────┐    ┌─────────────────┐    ┌──────────────┐ │
│   │ terraphim-tui   │    │ Knowledge Graph │    │ MCP Tools    │ │
│   │ replace CLI     │    │ docs/src/kg/    │    │ lib.rs       │ │
│   └────────┬────────┘    └────────┬────────┘    └──────────────┘ │
│            │                      │                               │
│            └──────────────────────┘                               │
│                      │                                            │
│                      ▼                                            │
│            ┌─────────────────┐                                    │
│            │ Aho-Corasick    │                                    │
│            │ FST Matcher     │                                    │
│            └─────────────────┘                                    │
└───────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌───────────────────────────────────────────────────────────────────┐
│                          Git Layer                                │
├───────────────────────────────────────────────────────────────────┤
│   ┌─────────────────┐    ┌─────────────────┐                      │
│   │ .git/hooks/     │    │ prepare-commit  │                      │
│   │ pre-commit      │────│ -msg            │                      │
│   └─────────────────┘    └─────────────────┘                      │
└───────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Current Responsibility | New Responsibility |
|-----------|----------------------|-------------------|
| `.claude/settings.local.json` | Permission allowlists | Add hook configuration references |
| `.claude/hooks/` | SubagentStart context | Add PreToolUse hook for npm→bun |
| `scripts/hooks/pre-commit` | Rust/JS quality checks | Add attribution replacement |
| `terraphim-tui` | REPL and search | Expose `replace` subcommand |
| `terraphim_mcp_server` | Autocomplete tools | Add self-documenting API endpoints |
| `docs/src/kg/` | Knowledge graph definitions | Already contains required mappings |

### Boundaries

**Changes INSIDE existing components:**
- `.claude/hooks/` - Add new hook file
- `scripts/hooks/pre-commit` - Extend with attribution replacement
- `.claude/settings.local.json` - Reference new hooks

**New components introduced:**
- `.claude/hooks/npm_to_bun_guard.py` - PreToolUse hook script
- `.git/hooks/prepare-commit-msg` - Git hook for attribution
- `scripts/install-terraphim-hooks.sh` - Easy-mode installer

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|--------|-------|--------------|
| `.claude/hooks/npm_to_bun_guard.py` | Create | - | PreToolUse hook intercepting Bash commands | terraphim-tui |
| `.claude/settings.local.json` | Modify | Only permissions | Add PreToolUse hook config | npm_to_bun_guard.py |
| `scripts/hooks/pre-commit` | Modify | Quality checks only | Add attribution replacement call | terraphim-tui |
| `.git/hooks/prepare-commit-msg` | Create | - | Modify commit messages via terraphim-tui | terraphim-tui |
| `scripts/install-terraphim-hooks.sh` | Create | - | Auto-detect and install all hooks | All hook files |
| `crates/terraphim_tui/src/main.rs` | Modify | REPL-focused | Add `replace` subcommand for piped input | terraphim_automata |
| `crates/terraphim_mcp_server/src/lib.rs` | Modify | Tool implementations | Add `capabilities` and `robot-docs` tools | Existing tools |

### Detailed Changes

#### 1. `.claude/hooks/npm_to_bun_guard.py` (New)

```python
#!/usr/bin/env python3
"""
PreToolUse hook that replaces npm/yarn/pnpm commands with bun.
Follows Claude Code hook protocol: reads JSON from stdin, outputs JSON to stdout.
"""
# Key elements:
# - Read tool_name and input from stdin JSON
# - Only process "Bash" tool calls
# - Call terraphim-tui replace on command
# - Return modified command or allow through
```

#### 2. `.claude/settings.local.json` (Modify)

Add hooks configuration:
```json
{
  "permissions": { /* existing */ },
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": ".claude/hooks/npm_to_bun_guard.py"
          }
        ]
      }
    ]
  }
}
```

#### 3. `scripts/hooks/pre-commit` (Modify)

Add section after existing checks:
```bash
# Attribution replacement (Terraphim AI)
if command_exists terraphim-tui; then
    # Handled by prepare-commit-msg hook
    :
fi
```

#### 4. `.git/hooks/prepare-commit-msg` (New)

```bash
#!/bin/bash
# Replace Claude attribution with Terraphim AI in commit messages
COMMIT_MSG_FILE=$1
if command -v terraphim-tui >/dev/null 2>&1; then
    ORIGINAL=$(cat "$COMMIT_MSG_FILE")
    REPLACED=$(echo "$ORIGINAL" | terraphim-tui replace 2>/dev/null)
    if [ -n "$REPLACED" ] && [ "$REPLACED" != "$ORIGINAL" ]; then
        echo "$REPLACED" > "$COMMIT_MSG_FILE"
        echo "Terraphim: Attribution updated" >&2
    fi
fi
```

#### 5. `scripts/install-terraphim-hooks.sh` (New)

```bash
#!/bin/bash
# Easy-mode installer for Terraphim hooks
# Inspired by Ultimate Bug Scanner's install.sh

detect_claude_code() { ... }
install_pretooluse_hook() { ... }
install_git_hooks() { ... }
main() { ... }
```

#### 6. `terraphim-tui replace` subcommand (Modify)

Add to existing CLI:
```rust
#[derive(Subcommand)]
enum Commands {
    // Existing commands...

    /// Replace text using knowledge graph patterns (for piped input)
    Replace {
        /// Optional text to replace (reads from stdin if not provided)
        text: Option<String>,
        /// Role to use for replacement patterns
        #[arg(short, long, default_value = "Terraphim Engineer")]
        role: String,
    },
}
```

#### 7. MCP Self-documenting API (Modify)

Add to `terraphim_mcp_server`:
```rust
// New tools:
// - "capabilities": List available features as JSON
// - "robot_docs": LLM-optimized documentation
// - "introspect": Full schema with argument types
```

## 5. Step-by-Step Implementation Sequence

### Phase 1: Core Replacement Infrastructure (Steps 1-3)

| Step | Task | Purpose | Deployable? |
|------|------|---------|-------------|
| 1 | Add `replace` subcommand to terraphim-tui | Enable piped text replacement | Yes |
| 2 | Test `replace` with existing KG files | Validate bun.md, terraphim_ai.md work | Yes |
| 3 | Create prepare-commit-msg Git hook | Attribution replacement in commits | Yes |

### Phase 2: Claude Code PreToolUse Hook (Steps 4-6)

| Step | Task | Purpose | Deployable? |
|------|------|---------|-------------|
| 4 | Create npm_to_bun_guard.py hook script | Intercept Bash commands | Yes |
| 5 | Update .claude/settings.local.json | Register PreToolUse hook | Yes (requires Claude restart) |
| 6 | Test with real Claude Code session | Validate AC1-AC3 | Yes |

### Phase 3: Easy-Mode Installation (Steps 7-8)

| Step | Task | Purpose | Deployable? |
|------|------|---------|-------------|
| 7 | Create install-terraphim-hooks.sh | Zero-config setup | Yes |
| 8 | Add --easy-mode flag | UBS-inspired auto-detection | Yes |

### Phase 4: MCP Self-documenting API (Steps 9-11)

| Step | Task | Purpose | Deployable? |
|------|------|---------|-------------|
| 9 | Add `capabilities` MCP tool | Feature discovery | Yes |
| 10 | Add `robot_docs` MCP tool | LLM-optimized docs | Yes |
| 11 | Add `introspect` MCP tool | Schema + types | Yes |

### Phase 5: Documentation & Testing (Steps 12-14)

| Step | Task | Purpose | Deployable? |
|------|------|---------|-------------|
| 12 | Update TERRAPHIM_CLAUDE_INTEGRATION.md | Document new hooks | Yes |
| 13 | Add test scripts for hooks | Validate all ACs | Yes |
| 14 | Update CLAUDE.md with hook instructions | Teach future agents | Yes |

## 6. Testing & Verification Strategy

| Acceptance Criterion | Test Type | Test Location | Command |
|---------------------|-----------|---------------|---------|
| AC1: npm install → bun install | Unit | scripts/test-hooks.sh | `echo "npm install" \| terraphim-tui replace` |
| AC2: yarn install → bun install | Unit | scripts/test-hooks.sh | `echo "yarn install" \| terraphim-tui replace` |
| AC3: pnpm install → bun install | Unit | scripts/test-hooks.sh | `echo "pnpm install" \| terraphim-tui replace` |
| AC4: Claude Code → Terraphim AI | Integration | scripts/test-hooks.sh | Create test commit, verify message |
| AC5: Claude → Terraphim AI | Integration | scripts/test-hooks.sh | Create test commit, verify message |
| AC6: Hook failure → pass-through | Unit | scripts/test-hooks.sh | Simulate terraphim-tui failure |
| AC7: Verbose logging | Unit | scripts/test-hooks.sh | `TERRAPHIM_VERBOSE=1` check stderr |
| AC8: MCP tools discoverable | Integration | cargo test -p terraphim_mcp_server | Test tools/list includes capabilities |
| AC9: Performance < 100ms | Performance | scripts/test-hooks.sh | `time terraphim-tui replace` |

### Test Script Template

```bash
#!/bin/bash
# scripts/test-terraphim-hooks.sh

set -e

echo "Testing Terraphim Hooks..."

# AC1-AC3: Package manager replacement
assert_replace() {
    local input="$1"
    local expected="$2"
    local actual=$(echo "$input" | ./target/release/terraphim-tui replace 2>/dev/null)
    if [ "$actual" = "$expected" ]; then
        echo "✓ '$input' → '$expected'"
    else
        echo "✗ '$input' → got '$actual', expected '$expected'"
        exit 1
    fi
}

assert_replace "npm install" "bun install"
assert_replace "yarn install" "bun install"
assert_replace "pnpm install" "bun install"
assert_replace "npm install && npm test" "bun install && bun test"

# AC4-AC5: Attribution replacement
assert_replace "Generated with Claude Code" "Generated with Terraphim AI"
assert_replace "Co-Authored-By: Claude" "Co-Authored-By: Terraphim AI"

# AC6: Fail-open
echo "npm install" | ./target/release/terraphim-tui replace --role "nonexistent" 2>/dev/null || echo "npm install"

# AC9: Performance
time_ms=$(./target/release/terraphim-tui replace "npm install" 2>&1 | grep -oP '\d+(?=ms)' || echo "0")
if [ "$time_ms" -lt 100 ]; then
    echo "✓ Performance: ${time_ms}ms < 100ms"
else
    echo "✗ Performance: ${time_ms}ms >= 100ms"
    exit 1
fi

echo "All tests passed!"
```

## 7. Risk & Complexity Review

| Risk (from Phase 1) | Mitigation | Residual Risk |
|--------------------|------------|---------------|
| **Performance overhead** | Use pre-built FST automata; cache in memory | Minimal - FST matching is O(n) |
| **False positives** | KG files use explicit synonyms; no regex guessing | Low - only exact matches |
| **Breaking changes** | Version hooks with Terraphim releases; add compatibility checks | Medium - Claude API may change |
| **Agent bypass** | Document as "safety net, not security boundary" | Accepted - by design |
| **Configuration complexity** | Provide install-terraphim-hooks.sh with --easy-mode | Low after installer exists |
| **Hook execution order** | Single hook per type; avoid conflicts | Low |
| **State persistence** | Hooks are stateless; use filesystem for any persistence | None |

### New Risks Identified

| Risk | Severity | Mitigation |
|------|----------|------------|
| **terraphim-tui not in PATH** | Medium | Installer adds to PATH; hooks use absolute paths |
| **Claude restart required** | Low | Document in installation instructions |
| **Git hooks not installed** | Low | Installer copies to .git/hooks/ |
| **Python not available** | Low | Provide bash fallback for PreToolUse hook |

## 8. Open Questions / Decisions for Human Review

1. **Hook Language**: The design uses Python for PreToolUse hook (like UBS's git_safety_guard.py). Alternative: pure Bash. Python provides better JSON handling and error messages.

2. **Verbose Mode Default**: Should `TERRAPHIM_VERBOSE=1` be the default initially (for debugging), then switched off later?

3. **MCP vs TUI for Hooks**: Design uses terraphim-tui. Alternative: call MCP server via HTTP. TUI is simpler (no running server required), but MCP would be more consistent with other integrations.

4. **prepare-commit-msg vs commit-msg**: Design uses prepare-commit-msg (modifies message before editor opens). Alternative: commit-msg (modifies after editor closes). prepare-commit-msg is less intrusive.

5. **Hook Installation Location**: Design places hooks in `.claude/hooks/` (project-local). Alternative: `~/.claude/hooks/` (global). Project-local is safer for testing, global is more convenient.

6. **Existing pre-commit Integration**: Should attribution replacement be in:
   - prepare-commit-msg (separate hook, cleaner separation)?
   - pre-commit (single location, but pre-commit doesn't modify messages)?

   Design uses prepare-commit-msg for correctness.

7. **Test Coverage**: Should we add:
   - E2E tests with actual Claude Code session (expensive)?
   - Mock-based integration tests (faster but less realistic)?

---

## Summary

This plan delivers a working implementation for teaching LLMs Terraphim capabilities through:

1. **PreToolUse Hook** (npm_to_bun_guard.py) - Intercepts Bash commands
2. **Git Hook** (prepare-commit-msg) - Modifies commit messages
3. **MCP Tools** - Self-documenting API for capability discovery
4. **Easy Installer** - Zero-config setup script

The implementation follows patterns from Ultimate Bug Scanner (agent detection, file-save hooks) and CASS (self-documenting APIs, structured output).

---

**Do you approve this plan as-is, or would you like to adjust any part?**
