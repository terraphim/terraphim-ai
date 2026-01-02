# Session Handover - Git Safety Guard Implementation

## Date: 2026-01-02

## Progress Summary

### Tasks Completed

1. **Implemented git safety guard command** - New `terraphim-agent guard` subcommand that blocks destructive git/filesystem commands using regex pattern matching with allowlist support

2. **Created PreToolUse hook** - `.claude/hooks/git_safety_guard.sh` integrates with Claude Code to block dangerous commands before execution

3. **Updated hook infrastructure** - Modified `scripts/install-terraphim-hooks.sh` to include the new guard hook and updated `.claude/settings.local.json`

4. **Created skill documentation** - Added `git-safety-guard` skill to terraphim-claude-skills repository

5. **Architect review** - Evaluated whether guard could use existing infrastructure (CommandValidator, DangerousPatternHook, knowledge graph). Conclusion: Keep current regex implementation, future refactor to terraphim_hooks crate

6. **Design plan** - Created `docs/designs/dangerous-pattern-hook-unification.md` for future consolidation of guard patterns with DangerousPatternHook

### Current Implementation State

- Commit: `dbd0d7a5` - feat(agent): add git safety guard to block destructive commands
- Branch: main
- All tests passing
- Hook configured and working

### What's Working

- `terraphim-agent guard` command blocks: git checkout --, git reset --hard, rm -rf (non-temp), git push --force, git stash drop/clear, git branch -D
- `terraphim-agent guard` allows: git checkout -b, git restore --staged, rm -rf /tmp/, git push --force-with-lease
- PreToolUse hook integration with Claude Code
- Fail-open semantics (if agent not found, commands pass through)
- JSON output for programmatic use

### What's Blocked/Pending

- **terraphim-claude-skills commit** - Skill created but not committed to that repo
- **DangerousPatternHook unification** - Design plan created, implementation deferred (Phase 2)
- **Pattern loading from config** - Future feature, not implemented yet

## Technical Context

```bash
# Current branch
main

# Recent commits
dbd0d7a5 feat(agent): add git safety guard to block destructive commands
eebd8ee8 fix(ci): convert Uint8Array to Buffer for Bun compatibility
1cb4559c fix(ci): make tests optional and fix universal binary job
b8caecec fix(docker): use Rust 1.92 for edition2024 support
4fb03372 fix(ci): use custom Docker image for aarch64 NAPI builds

# Modified files (uncommitted)
.docs/plans/pr-381-github-runner-integration-design.md
.opencode/
.playwright-mcp/*.png
MIGRATION_PLAN_ZOLA_TO_MDBOOK.md
droid_configuration.md
```

## Key Discoveries

### 1. Rust regex crate doesn't support look-ahead
- Pattern `(?!-with-lease)` fails to compile
- Solution: Use allowlist to handle safe variants instead of negative look-ahead

### 2. TuiService initialization is expensive
- Guard command doesn't need TuiService but all commands went through it
- Solution: Handle stateless commands (guard) before TuiService initialization

### 3. Claude Code hook output format
- Must output JSON with `hookSpecificOutput.permissionDecision: "deny"` to block
- Empty output = allow command
- Always exit 0 (non-zero exits are treated as hook failures)

## Files Changed

| File | Change |
|------|--------|
| `crates/terraphim_agent/src/guard_patterns.rs` | NEW - Pattern matching with allowlist |
| `crates/terraphim_agent/src/main.rs` | MODIFIED - Added guard subcommand |
| `.claude/hooks/git_safety_guard.sh` | NEW - PreToolUse hook script |
| `.claude/settings.local.json` | MODIFIED - Added hook config |
| `scripts/install-terraphim-hooks.sh` | MODIFIED - Include guard in install |
| `docs/designs/dangerous-pattern-hook-unification.md` | NEW - Future refactor plan |

## Next Steps

### Priority 1: Commit to terraphim-claude-skills
```bash
cd /Users/alex/projects/terraphim/terraphim-claude-skills
git add skills/git-safety-guard/
git commit -m "feat: add git-safety-guard skill"
git push
```

### Priority 2: Test in production Claude Code session
- Restart Claude Code to load new hooks
- Verify blocked commands show proper error message
- Verify allowed commands pass through

### Priority 3 (Future): DangerousPatternHook unification
- Follow design plan in `docs/designs/dangerous-pattern-hook-unification.md`
- Move PatternGuard to terraphim_hooks crate
- Update both terraphim_agent and terraphim_multi_agent

## Architecture Notes

```
Current:
  terraphim_agent/guard_patterns.rs
       |
       v
  .claude/hooks/git_safety_guard.sh
       |
       v
  Claude Code PreToolUse

Future (per design plan):
  terraphim_hooks/guard.rs (shared)
       /              \
      v                v
  terraphim_agent   terraphim_multi_agent
  (CLI guard)       (DangerousPatternHook)
```

## Commands Reference

```bash
# Test guard command
echo "git checkout -- file.txt" | terraphim-agent guard --json

# Test hook directly
echo '{"tool_name":"Bash","tool_input":{"command":"git reset --hard"}}' | \
  .claude/hooks/git_safety_guard.sh

# Run unit tests
cargo test -p terraphim_agent guard_patterns

# Install all hooks
./scripts/install-terraphim-hooks.sh --easy-mode
```

## Background

On December 17, 2025, an AI agent ran `git checkout --` on files containing hours of uncommitted work from another agent (Codex). This destroyed the work instantly. The files were recovered from a dangling Git object, but this incident revealed that instructions alone (AGENTS.md) don't prevent execution of destructive commands. This guard provides mechanical enforcement.
