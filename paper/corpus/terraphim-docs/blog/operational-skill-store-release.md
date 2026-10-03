# Your AI Agent Has Amnesia. We Fixed That.

Every AI coding agent you use today has the memory of a goldfish. It makes the same mistake at 4pm that you corrected at 9am. It suggests `npm install` after you told it three times that your project uses `bun`. It has no idea that `git push --force` to main got you fired from your last job.

This is not a tooling problem. It is an architecture problem. And we just solved it.

## From Failure Capture to Procedural Memory

Terraphim's learning system started where most do: recording failures. A PostToolUse hook captured every crashed command -- the command itself, exit code, stderr, working directory. Useful, but purely defensive. You could query past failures with `terraphim-agent learn query "npm"` and get back a list of things that went wrong. Better than nothing.

But recording failures is like a pilot who only studies crashes. What about the flights that landed safely? What about the multi-step workflows that actually work -- the ones worth repeating?

The Operational Skill Store turns terraphim-agent from a defensive failure log into a procedural memory system. It records successful workflows, replays them, monitors their health over time, and auto-disables procedures that start failing. The agent does not just remember what went wrong. It remembers what went right, and knows when "right" stops working.

## What It Actually Does

**Record multi-step procedures:**

```bash
terraphim-agent procedure record "Deploy staging" \
  --description "Full staging deployment workflow"

terraphim-agent procedure add-step deploy-staging-01 \
  "cargo test --workspace" \
  --precondition "all code committed" \
  --postcondition "tests pass"

terraphim-agent procedure add-step deploy-staging-01 \
  "cargo build --release" \
  --postcondition "binary exists at target/release/"
```

Each procedure is a sequence of ordered steps with optional preconditions and postconditions. Not a shell script -- a structured, queryable, annotatable artifact.

**Replay with safety guards:**

```bash
terraphim-agent procedure replay deploy-staging-01 --dry-run
terraphim-agent procedure replay deploy-staging-01
```

Replay executes steps in order. Privileged steps are skipped. Destructive commands (anything matching guard patterns like `rm -rf`, `git reset --hard`, `DROP TABLE`) are blocked automatically. On any step failure, replay stops. No "continuing past the point where the database got deleted."

**Self-healing through health monitoring:**

```bash
terraphim-agent procedure health
```

Every replay records success or failure against the procedure's confidence metrics. The health monitor computes a score from the ratio. Procedures with scores below 0.3 over 5+ executions are auto-disabled. You get a report:

```
deploy-staging-01  "Deploy staging"     Healthy    (0.85, 17/20)
migrate-db-02      "DB migration v2"    Degraded   (0.55, 11/20)
old-deploy-03      "Legacy deploy"      Critical   (0.20, 2/10)  [AUTO-DISABLED]
```

No human intervention required. The system notices when a procedure stops working and takes it out of rotation.

**Capture procedures from session history:**

```bash
terraphim-agent procedure from-session ses_abc123 \
  --title "Set up new Rust crate"
```

If you already have a recorded AI coding session (Claude Code, Cursor, Aider), the system extracts successful Bash commands and builds a procedure automatically. Your best sessions become replayable workflows.

## The Knowledge Graph Connection

Every learning and procedure is annotated with entities from Terraphim's knowledge graph via Aho-Corasick automata matching. When you capture a failed `npm install` command, the system matches "npm" against the knowledge graph, finds the synonym mapping to "bun", and surfaces that connection.

The PreToolUse hook uses this same pipeline in real-time. Before a command executes, the hook checks it against known patterns. If you are about to run `npm install` and the knowledge graph has a `bun` synonym mapping, you get a warning before the command fires -- not after it fails.

This is not keyword matching. The Aho-Corasick automaton uses LeftmostLongest matching across the entire thesaurus, built from markdown files in your knowledge graph directory. Adding a new pattern is writing a markdown file:

```markdown
# bun

The preferred JavaScript runtime and package manager.

synonyms:: npm, yarn, pnpm
```

The automaton rebuilds, and every future command gets checked against the new pattern. The thesaurus is loaded once via `OnceLock` and reused across the session -- no repeated filesystem hits.

## The Multi-Hook Pipeline

The system hooks into three points of the AI agent lifecycle:

1. **PreToolUse**: Checks proposed commands against past failures and knowledge graph patterns. "You tried this before and it failed" or "your project uses bun, not npm."
2. **PostToolUse**: Captures failures with full context -- command, exit code, stderr, working directory, timestamp. Automatically redacts secrets before storage.
3. **UserPromptSubmit**: Parses natural language corrections like "use X instead of Y" and stores them as typed corrections (ToolPreference, CodePattern, Naming, WorkflowStep, FactCorrection, StylePreference).

Learnings are ranked by an importance score that weighs severity (how bad was the failure), repetition (how many times has this happened), recency (when did it last occur), and whether a human corrected it (user corrections always rank highest).

## How We Built It

Ten implementation phases, each verified through V-model right-side verification. That means every phase had acceptance criteria defined before coding started, and verification happened against those criteria -- not against "does it compile."

The implementation is about 3,000 lines of new Rust across `terraphim_agent` and `terraphim_types`. Procedures are stored as JSONL for append-friendly persistence. Learnings are individual markdown files with YAML frontmatter for structured metadata.

Feature gating keeps the binary lean. Session auto-capture requires `repl-sessions`. Shared learning (trust-leveled cross-agent knowledge sharing with BM25 deduplication) requires `shared-learning`. The core procedure and learning capture system has zero optional dependencies.

We closed 8 issues and merged 4 PRs across GitHub and Gitea to ship this. Every feature was TDD: test first, implement to pass, verify against the phase's acceptance criteria.

## What Is Next

**Graduated guard patterns.** Right now, destructive commands are binary: blocked or allowed. The next step is confidence-based graduated responses -- warn at medium confidence, block at low confidence, allow at high confidence.

**Agent evolution.** Procedures that consistently succeed should propagate to other agents via shared learning. Procedures that fail should trigger automatic investigation of what changed. The system should not just disable bad procedures -- it should figure out why they went bad.

**Deeper session integration.** Auto-capture from sessions is the start. The goal is continuous procedure refinement: every session that touches a known procedure updates its steps, preconditions, and confidence scores.

## The Bottom Line

AI agents that forget everything between sessions are not agents. They are autocomplete with a chat interface. Real agency requires memory -- not just of what the user said, but of what worked, what failed, and what is currently degrading.

The Operational Skill Store gives terraphim-agent procedural memory. It records, replays, monitors, and self-heals. It is written in Rust, uses zero unsafe code, and the entire learning pipeline adds under 5ms to command execution.

Your AI agent should learn from experience. Now ours does.

---

*Terraphim AI is open source. The Operational Skill Store ships with terraphim-agent v1.12. Install with `cargo install terraphim-agent` or build from source at [github.com/terraphim/terraphim-ai](https://github.com/terraphim/terraphim-ai).*
