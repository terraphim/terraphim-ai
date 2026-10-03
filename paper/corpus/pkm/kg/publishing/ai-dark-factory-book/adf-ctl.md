# adf-ctl

Concept from the AI Dark Factory book, verified against the implementation in `terraphim-ai`, `gitea-robot`, or the Zestic Gitea fork. Category: cli.

synonyms:: adf-ctl CLI, ADF orchestrator control

CLI in crates/terraphim_orchestrator/src/bin/adf-ctl.rs. Talks to bigbox via SSH + HMAC-signed webhook. Four subcommands: trigger, status, cancel, agents. --format json on status and agents for parseable output (Refs #1495).
