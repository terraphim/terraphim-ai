# Reddit Announcements

## r/rust

**Title:** We built a procedural memory system for AI coding agents in Rust -- procedures, replay, self-healing, knowledge graph matching

**Body:**

We have been building [terraphim-agent](https://github.com/terraphim/terraphim-ai), a Rust CLI tool that acts as a local-first AI coding assistant. The latest release adds what we call the Operational Skill Store -- a procedural memory system that lets the agent record successful workflows, replay them with safety guards, and auto-disable procedures that start failing.

### What it does

**Record multi-step procedures:**

```bash
terraphim-agent procedure record "Deploy staging" \
  --description "Full staging deployment workflow"

terraphim-agent procedure add-step deploy-staging-01 \
  "cargo test --workspace" \
  --precondition "all code committed"

terraphim-agent procedure add-step deploy-staging-01 \
  "cargo build --release"
```

**Replay with destructive command guards:**

```bash
terraphim-agent procedure replay deploy-staging-01 --dry-run
terraphim-agent procedure replay deploy-staging-01
```

The replay engine skips privileged steps and blocks destructive commands (`rm -rf`, `git reset --hard`, etc.) via a guard pattern system. On any step failure, replay stops immediately.

**Self-healing health monitor:**

```bash
terraphim-agent procedure health
```

Every execution records success/failure against confidence metrics. Procedures with scores below 0.3 over 5+ executions are auto-disabled. No manual intervention needed.

**Auto-capture from AI coding sessions:**

```bash
terraphim-agent procedure from-session ses_abc123 --title "Setup workflow"
```

Extracts successful Bash commands from Claude Code / Cursor / Aider session logs and builds replayable procedures automatically.

### Rust-specific things that might interest you

- **Aho-Corasick automata** for knowledge graph entity matching. Every learning and procedure is annotated with entities from a thesaurus built from markdown files. LeftmostLongest matching, loaded once via `OnceLock`.
- **Guard pattern system** using a `CommandGuard` struct that pattern-matches destructive commands before execution. Binary decision (Block/Allow) with reason strings.
- **JSONL persistence** for procedures (append-friendly), markdown with YAML frontmatter for learnings. No database dependency.
- **Feature gating** keeps the binary lean: `repl-sessions` for session auto-capture, `shared-learning` for cross-agent knowledge sharing with BM25 dedup.
- **Multi-hook pipeline**: PreToolUse (warn on past failures), PostToolUse (capture failures with secret redaction), UserPromptSubmit (parse natural language corrections).
- **Importance scoring** with weighted factors: severity, repetition, recency, user correction. Used to rank which learnings surface first during PreToolUse checks.
- Zero `unsafe` code in the entire learning pipeline.

About 3,000 lines of new code across `terraphim_agent` and `terraphim_types` crates. Ten implementation phases, each with TDD and V-model verification.

The thesaurus/automata system is the same one that powers terraphim's semantic search -- we reuse `terraphim_automata` for both document indexing and learning annotation. Adding a new guard pattern or tool preference is writing a markdown file with synonym mappings.

Full source: [github.com/terraphim/terraphim-ai](https://github.com/terraphim/terraphim-ai)

Happy to answer questions about the Aho-Corasick integration, the guard pattern design, or the V-model verification approach.

---

## r/LocalLLaMA

**Title:** Built a "procedural memory" system for AI coding agents -- records workflows, replays them, auto-disables failing procedures

**Body:**

Every AI coding agent I have used has the same problem: zero memory between sessions. It suggests `npm` after you told it to use `bun`. It runs destructive git commands you explicitly warned against. It makes the same mistake at 4pm that you corrected at 9am.

We built a system called the Operational Skill Store for [terraphim-agent](https://github.com/terraphim/terraphim-ai) that gives AI coding agents actual procedural memory.

### The concept

Instead of just logging failed commands (which is where we started), the system now:

1. **Records successful workflows** as structured procedures with ordered steps, preconditions, and postconditions
2. **Replays procedures** step-by-step with destructive command guards (blocks `rm -rf`, `git reset --hard`, etc.)
3. **Tracks confidence** -- every execution records success/failure, computes a score
4. **Self-heals** -- procedures with critically low success rates (< 30% over 5+ runs) are auto-disabled
5. **Auto-captures from sessions** -- extracts procedures from Claude Code / Cursor / Aider session logs

### The hook pipeline

Three hooks intercept the agent lifecycle:

- **PreToolUse**: Before a command runs, checks it against past failures AND against a knowledge graph. If "npm" has a synonym mapping to "bun" in your KG, you get warned before the command executes.
- **PostToolUse**: Captures failed commands with full context. Redacts secrets automatically.
- **UserPromptSubmit**: Parses corrections like "use bun instead of npm" and stores them as typed learning entries (ToolPreference, CodePattern, Naming, etc.)

Learnings are ranked by importance: severity * repetition * recency * user-correction-bonus. The most relevant past mistakes surface first.

### CLI examples

```bash
# Record a procedure
terraphim-agent procedure record "Deploy to staging"
terraphim-agent procedure add-step deploy-01 "cargo test --workspace"
terraphim-agent procedure add-step deploy-01 "cargo build --release"

# Replay it
terraphim-agent procedure replay deploy-01 --dry-run
terraphim-agent procedure replay deploy-01

# Check health of all procedures
terraphim-agent procedure health

# Auto-capture from a past session
terraphim-agent procedure from-session ses_abc123

# Query past failures
terraphim-agent learn query "npm"

# Add a correction
terraphim-agent learn correct <id> "use bun instead of npm"
```

### Why this matters for local LLM workflows

If you are running Ollama or similar local models for coding assistance, you lose even more context between sessions than cloud-hosted agents. The Operational Skill Store works entirely locally -- JSONL files for procedures, markdown files for learnings. No cloud dependency. No data leaves your machine.

The knowledge graph matching (Aho-Corasick automata over a thesaurus built from local markdown files) runs in-process. Adding new patterns is editing a markdown file with synonym mappings. The whole pipeline adds under 5ms to command execution.

This is the kind of agent memory that should be standard. Your local AI assistant should get smarter over time, not start from zero every morning.

Source: [github.com/terraphim/terraphim-ai](https://github.com/terraphim/terraphim-ai)

Would love to hear from anyone building similar memory systems for local LLM workflows.
