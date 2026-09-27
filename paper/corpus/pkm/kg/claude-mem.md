---
title: claude-mem
synonyms:
  - claude-memory
  - thedotmack/claude-mem
tags: claude-code, memory, plugin
---

# claude-mem

Cross-session memory plugin for Claude Code. Five lifecycle hooks, SQLite storage, web viewer at `localhost:37777`.

Repo: `thedotmack/claude-mem`. Install: `/plugin marketplace add thedotmack/claude-mem; /plugin install claude-mem`.

Overlaps with terraphim-agent session search (`~/.cargo/bin/terraphim-agent sessions search`). Evaluate as complementary (per-session SQLite + UI) vs duplicative (terraphim already indexes Claude Code JSONL). See `plans/ecc-agentshield-leverage.md`.
