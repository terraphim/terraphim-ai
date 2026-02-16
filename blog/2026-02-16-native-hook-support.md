# Introducing Native Hook Support: terraphim-agent Now Learns from Your Mistakes

**Date**: February 16, 2026
**Author**: Terraphim Engineering Team
**Tags**: rust, cli, ai-agents, developer-tools, machine-learning

---

## TL;DR

We're excited to announce **native hook support** for terraphim-agent v1.8.1! This feature captures failed commands from AI agents (Claude Code, Codex, OpenCode) and learns from them, creating a personal knowledge base of mistakes and corrections. No more jq dependency, no more bash wrappers—just `terraphim-agent learn hook`.

## The Problem

If you're using AI agents like Claude Code, you've probably experienced this:

1. Agent suggests: `cargo buid` (typo)
2. Command fails with error
3. You fix it: `cargo build`
4. **The mistake is forgotten**
5. Next session: Same typo, same failure

Every developer has their own "greatest hits" of mistakes:
- `npm isntall` instead of `npm install`
- `git psuh` instead of `git push`
- `pip intall` instead of `pip install`

These mistakes are personal, contextual, and **valuable**—if only we could remember them.

## The Solution: Native Hook Support

With terraphim-agent v1.8.1, we've introduced a complete learning system:

```bash
# One-command setup
terraphim-agent learn install-hook claude

# That's it. Every failed command is now captured automatically.
```

### How It Works

```
Claude Code executes Bash command
        ↓
Command fails (exit code != 0)
        ↓
Hook captures: command + error + context
        ↓
Stored as Markdown: ~/.local/share/terraphim/learnings/
        ↓
Query anytime: terraphim-agent learn query "cargo buid"
```

### Key Features

**1. Native Implementation**
- No external dependencies (no jq!)
- Written in Rust using serde
- ~50 lines of code vs 115-line bash wrapper
- 156 tests passing

**2. Universal Support**
Works with Claude Code, Codex, and OpenCode:
```bash
terraphim-agent learn install-hook claude
terraphim-agent learn install-hook codex
terraphim-agent learn install-hook opencode
```

**3. Fail-Open Design**
Never blocks your workflow. If capture fails, the command still executes.

**4. Smart Filtering**
- Ignores test commands (`cargo test`, `npm test`)
- Redacts secrets automatically (AWS keys, API tokens)
- Only captures Bash tool failures

**5. Rich Context**
Each learning includes:
- Command that failed
- Error output
- Exit code
- Timestamp
- Working directory
- Tags for categorization

## Live Demonstration

Let's prove it works with a realistic scenario:

### Step 1: Set Role
```bash
$ terraphim-agent setup --template rust-engineer-v2
Configuration set to role 'Rust Engineer v2'
```

### Step 2: Capture Mistake
```bash
# Simulate Claude Code making a typo
echo '{"tool_name":"Bash","tool_input":{"command":"cargo buid"},"tool_result":{"exit_code":101,"stderr":"error: no such subcommand"}}' | ~/.config/claude/terraphim-hook.sh
```

### Step 3: Verify Capture
```bash
$ terraphim-agent learn list
Recent learnings:
  1. [G] cargo buid (exit: 101)
```

### Step 4: Query Mistake
```bash
$ terraphim-agent learn query "cargo buid"
Learnings matching 'cargo buid':
  [G] cargo buid (exit: 101)
```

### Step 5: View Full Context
```bash
$ cat ~/.local/share/terraphim/learnings/learning-xxx.md
---
id: xxx-xxx
command: cargo buid
exit_code: 101
captured_at: 2026-02-16T15:34:39
---

## Command
`cargo buid`

## Error Output
```
error: no such subcommand
```
```

## Multi-Role Engineering

We've also added 4 new engineer role templates, each with different ranking methods:

| Role | Ranking | Use Case |
|------|---------|----------|
| **FrontEnd Engineer** | BM25Plus | JavaScript/TypeScript development |
| **Python Engineer** | BM25F | Python with field-weighted ranking |
| **Rust Engineer v2** | TitleScorer | Dual haystack (docs.rs + local) |
| **Terraphim Engineer v2** | TerraphimGraph | Graph embeddings + hybrid KG |

Each role learns differently and optimizes search for its domain.

## The Complete Learning Cycle

```
┌─────────────────────────────────────────────────────────────┐
│  1. LEARN                                                    │
│     Command fails → Hook captures → Markdown stored         │
│     ✅ Works with Claude, Codex, OpenCode                   │
├─────────────────────────────────────────────────────────────┤
│  2. QUERY                                                    │
│     Search patterns → Find similar mistakes                 │
│     ✅ Pattern matching on command + error                  │
├─────────────────────────────────────────────────────────────┤
│  3. CORRECT                                                  │
│     Add corrections: learn correct <id> --correction        │
│     ✅ Future: Auto-suggest from knowledge graph            │
├─────────────────────────────────────────────────────────────┤
│  4. REPLACE                                                  │
│     Real-time suggestions via replace --role <role>         │
│     ✅ Uses thesaurus for context-aware corrections         │
└─────────────────────────────────────────────────────────────┘
```

## Installation

```bash
# Install latest terraphim-agent
cargo install terraphim-agent

# Install hook for your AI agent
terraphim-agent learn install-hook claude

# Verify installation
terraphim-agent learn --help
```

## Verification & Validation

This release passed rigorous quality gates:

- ✅ **Static Analysis**: UBS scanner (0 critical findings)
- ✅ **Unit Tests**: 156 tests passing
- ✅ **Integration Tests**: All E2E scenarios pass
- ✅ **Acceptance Testing**: Live demonstration completed
- ✅ **Requirements Traceability**: 100% coverage

See full report: `.docs/verification-validation-report.md`

## What's Next

1. **Auto-suggest**: Query learnings in real-time during command entry
2. **Learning insights**: Analytics on most common mistakes per role
3. **Team sharing**: Optional sync of learnings across team members
4. **IDE integration**: VS Code extension for inline suggestions

## Get Started

```bash
# Install
cargo install terraphim-agent

# Set up your role
terraphim-agent setup --template rust-engineer-v2

# Install hook
terraphim-agent learn install-hook claude

# Start learning from your mistakes!
```

## Links

- **GitHub**: https://github.com/terraphim/terraphim-ai
- **Release**: v1.8.1
- **Documentation**: `.docs/learn-correct-cycle.md`
- **Issues**: Report bugs or request features

---

*Terraphim: Your AI agent's memory for mistakes.*

#rust #cli #ai #developer-tools #learning #claude #codex #opencode
