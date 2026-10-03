# Firecracker Executor

Concept from the AI Dark Factory book, verified against the implementation in `terraphim-ai`, `gitea-robot`, or the Zestic Gitea fork. Category: rlm-backend.

synonyms:: Firecracker microVM, VM-isolated execution

Linux only. Strongest isolation. Default on bigbox. Full VM state versioning supports branch-and-merge via rlm_snapshot. Sub-2 second boot, sub-500ms allocation.
