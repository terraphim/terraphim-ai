# Session: Fix #1851 - Shell injection in adf-ctl.rs var_name

**Started**: 2026-05-25T06:00:28+02:00
**Agent**: pi implementation session
**Issue**: #1851
**Branch**: task/1851-shell-injection-adf-ctl

## Context
- P1 security finding: shell injection via unvalidated env-var name in `resolve_secret()` 
- Location: `crates/terraphim_orchestrator/src/bin/adf-ctl.rs:194-195`
- CWE-78: OS Command Injection
- Introduced in commit d7613339

## Checkpoint
- No existing task branch for #1851
- No existing PR
- No relevant wiki learnings

## Status: IN PROGRESS
