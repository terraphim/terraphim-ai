# AGENTS.md

## Purpose
Repository-level operating instructions for AI coding agents working in `terraphim-ai`.

## Key Functionality
- Defines the mandatory `/init` workflow:
  create per-file summaries in `.docs/summary-<normalized-path>.md`, then synthesize them into `.docs/summary.md`.
- Establishes core engineering commands for Rust and frontend work, including build, lint, test, and feature-gated validation flows.
- Sets coding conventions for Rust, Svelte, async execution, error handling, and documentation maintenance.
- Requires task tracking with GitHub tooling and `bd`, plus end-of-session hygiene including quality gates, sync, push, and verification.
- Documents preferred operational patterns such as using `tmux` for background work and `ubs` for changed-file bug scanning before commit.

## Important Details
- `/init` is not optional once requested; both documentation steps are mandatory.
- `.docs/agents_instructions.json` is the machine-readable companion source for project-specific patterns.
- The file is stricter than a generic contributor guide because it mixes workflow policy, release hygiene, and documentation contracts.
- Some instructions apply only when code changes land, especially issue updates, quality gates, and push/sync requirements.
