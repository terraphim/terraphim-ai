# Terraphim AI Summary

## Scope
This consolidated summary synthesizes the current `.docs/summary-*.md` files, including agent-platform crates, LLM service integration, task decomposition, repository operating instructions, and the active `voice_transcribe` tool in `terraphim_tinyclaw`.

## Architecture
- `terraphim_multi_agent` is the main orchestration layer. It coordinates command handling, memory, tasks, lessons, LLM calls, and optional VM-backed execution.
- `terraphim_agent_evolution` provides versioned memory, task, and lesson storage. It is the persistence-oriented state layer behind agent adaptation and context retention.
- `terraphim_agent_messaging` and `terraphim_agent_supervisor` supply the concurrency model: mailbox-based communication plus OTP-style restart and lifecycle control.
- `terraphim_task_decomposition` and `terraphim_goal_alignment` extend orchestration with decomposition, dependency planning, semantic goal scoring, and conflict analysis via the knowledge graph.
- `terraphim_service` abstracts model providers and routing. It is the LLM boundary that higher-level agents depend on for summaries and chat completions.
- `terraphim_tinyclaw` is a more chat-oriented runtime with a tool registry and tool-calling loop. The `voice_transcribe` tool extends that loop so audio attachments can be converted into text before response generation.

## Security And Reliability
- The architecture emphasizes isolation boundaries: mailbox-driven delivery, supervision trees, durable memory snapshots, and optional VM execution reduce blast radius from individual failures.
- The `voice_transcribe` tool constrains inputs to HTTP(S) URLs, validates downloads, isolates CPU-bound media work in blocking tasks, and cleans up temp files after execution.
- Whisper model lookup is explicit and local-first, which avoids hidden network fetches during runtime but does require operators to provision model files correctly.
- Some higher-level summaries note capability gaps rather than defects, especially around richer tool-calling loops and feature maturity in goal alignment.

## Testing
- Existing summaries indicate strong automated coverage in several core crates:
  `terraphim_multi_agent`, `terraphim_agent_messaging`, `terraphim_agent_supervisor`, and `terraphim_task_decomposition` are described as production-ready with comprehensive passing tests.
- `terraphim_goal_alignment` is functional but not fully complete, with ignored tests noted in the summary.
- The `voice_transcribe` tool currently has targeted unit tests for schema and fallback behavior, but full transcription validation still depends on the optional `voice` feature and model availability.
- Repository instructions in `AGENTS.md` require build, lint, tests, and `ubs` scans on changed files before commit.

## Business Value
- Terraphim’s differentiator is not a single model wrapper; it is the combination of role-aware orchestration, structured memory, resilient multi-agent execution, and knowledge-graph-assisted planning.
- The TinyClaw tool loop shows a path from generic orchestration into user-facing assistants that can act on files, shell commands, web inputs, sessions, and now audio.
- Voice transcription is strategically useful because it closes a common messaging-channel gap: users can send speech while downstream reasoning still operates on normalized text.
- The repository operating model captured in `AGENTS.md` reinforces maintainability by making documentation refresh, quality gates, and handoff discipline part of normal development.
