# rlm_code

Concept from the AI Dark Factory book, verified against the implementation in `terraphim-ai`, `gitea-robot`, or the Zestic Gitea fork. Category: mcp-tool.

synonyms:: RLM code execution, isolated python execution

Runs Python in the active executor (LocalExecutor / DockerExecutor / Firecracker). Honours timeout_ms and kill_on_drop (Refs PR #870).
