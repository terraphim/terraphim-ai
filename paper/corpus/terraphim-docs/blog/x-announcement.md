# X/Twitter Thread

## Tweet 1

Your AI coding agent has the memory of a goldfish.

It suggests npm after you said bun. It runs git reset --hard after you warned against it. It repeats the same mistake you corrected three hours ago.

We fixed this. Thread on building procedural memory for AI agents in Rust.

## Tweet 2

The Operational Skill Store turns terraphim-agent from "record failures" into full procedural memory:

- Record multi-step workflows as structured procedures
- Replay them with destructive command guards
- Track confidence over time
- Auto-disable procedures that start failing

3,000 lines of Rust doing what should be standard.

## Tweet 3

The self-healing part is what matters most.

Every replay records success or failure. Score drops below 0.3 over 5+ runs? Procedure auto-disables. No human intervention.

Your deploy script broke because a dependency changed? The system notices before you do.

## Tweet 4

Knowledge graph integration via Aho-Corasick automata:

Before a command runs, it gets checked against your project's knowledge graph. "npm install" triggers a warning because your KG maps npm -> bun.

Adding new patterns = writing a markdown file with synonym mappings. The automaton rebuilds. Done.

## Tweet 5

Three hooks into the agent lifecycle:

PreToolUse: warn on past failures + KG pattern checks
PostToolUse: capture failures with secret redaction
UserPromptSubmit: parse "use X instead of Y" as typed corrections

Importance scoring ranks which learnings surface first. User corrections always win.

## Tweet 6

The part nobody talks about: AI agents that run locally need memory MORE than cloud agents.

No conversation history across sessions. No server-side context.

The Operational Skill Store is entirely local. JSONL + markdown. Zero cloud dependency. Under 5ms overhead per command.

## Tweet 7

Open source, written in Rust, ships with terraphim-agent.

Record. Replay. Monitor. Self-heal.

Your AI agent should learn from experience. Now ours does.

github.com/terraphim/terraphim-ai
