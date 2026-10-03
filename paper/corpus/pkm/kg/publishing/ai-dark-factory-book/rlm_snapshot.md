# rlm_snapshot

Concept from the AI Dark Factory book, verified against the implementation in `terraphim-ai`, `gitea-robot`, or the Zestic Gitea fork. Category: mcp-tool.

synonyms:: RLM VM snapshot, branch-and-merge

Backend-conditional. Firecracker = full VM state versioning; Docker = container restart only; LocalExecutor returns RlmError::NotSupported (Refs PR #870). Use for branch-and-merge patterns.
