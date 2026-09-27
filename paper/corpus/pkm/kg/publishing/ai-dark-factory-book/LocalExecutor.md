# LocalExecutor

Concept from the AI Dark Factory book, verified against the implementation in `terraphim-ai`, `gitea-robot`, or the Zestic Gitea fork. Category: rlm-backend.

synonyms:: Local executor backend, process isolation

Default backend on Mac. Process-level isolation. Fully supports rlm_code, rlm_bash, rlm_query, rlm_context, rlm_status. Snapshots return NotSupported. Honours timeout_ms and kill_on_drop (Refs PR #870).
