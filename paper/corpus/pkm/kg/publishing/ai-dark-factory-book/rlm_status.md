# rlm_status

Concept from the AI Dark Factory book, verified against the implementation in `terraphim-ai`, `gitea-robot`, or the Zestic Gitea fork. Category: mcp-tool.

synonyms:: RLM status, RLM session status

Returns active backend, token budget, time remaining, and recent tool calls. Call at session start and after every ~5 tool calls. Source of truth for runtime state (Refs terraphim-ai PR #870).
