+++
title="Case Study: Knowledge Graph Validation Workflows for Pre/Post-LLM"
date=2026-01-19

[taxonomies]
categories = ["Engineering", "AI", "Knowledge Graphs"]
tags = ["Terraphim", "knowledge-graph", "mcp", "hooks", "cli", "workflows"]
[extra]
toc = true
comments = true
+++

A build-in-public case study of how we turned underutilized knowledge graph features into a local-first validation pipeline for AI coding workflows.

<!-- more -->

## Why This Existed

Terraphim already had powerful primitives (connectivity checks, fuzzy matching, role-aware graphs). The missing piece was an opinionated workflow and a stable interface (CLI + hooks) that made those primitives usable.

## The Approach

- Pre-LLM: validate semantic coherence before spending tokens
- Post-LLM: validate outputs against domain checklists
- Developer UX: unify everything behind the `terraphim-agent` CLI and Claude Code hooks

## The Results

- MCP connectivity tool wired to real RoleGraph logic
- New CLI commands (`validate`, `suggest`, unified `hook` handler)
- Skills + hook scripts that standardize the workflow

## References

- mdBook case study + appendices: https://docs.terraphim.ai/src/kg/case-studies/kg-validation-workflows.html
- Source materials:
  - `docs/sessions/session-20251228-201509.md`
  - `docs/sessions/research-underutilized-features.md`
  - `docs/sessions/design-underutilized-features.md`
  - `docs/sessions/implementation-summary.md`
