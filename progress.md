# Terraphim AI - Progress Tracking

## Current WIGs (Wildly Important Goals) - Q2 2026

1. **WIG-1: Stabilise core search and ranking pipeline**
   - Fast, accurate local-first search across knowledge graphs
   - Ranking algorithm tuned for role-based contexts
   - Resolve test hangs and race conditions in ranking

2. **WIG-2: Unify Tauri desktop and server parity**
   - Desktop app matches server API surface
   - Shared configuration and state management
   - End-to-end testing covers both paths

3. **WIG-3: Complete agent orchestration and MCP integration**
   - Multi-agent workflow execution
   - MCP server interoperability
   - Hook system for custom integrations

4. **WIG-4: Production-ready build and release pipeline**
   - CI/CD passes on all platforms
   - Automated releases with artifacts
   - Quality gates (Sentrux) enforced

## Current Quarter Focus
- MSRV stabilisation: `.clippy.toml` set to 1.91.0; polyfill removal (#2811 PR #2812) and workspace rust-version propagation (#2770 PR #2774) in review queue (WIG-4)
- Grep correctness: #2722 — kg_hits/concepts hardcoded to 0/empty in Insufficient path; fix target is terraphim-clients polyrepo (WIG-1)
- ADF live-validation epic #2707 — skill-level validation coverage; files issues on generator not leaves (WIG-3)
- Quality gate: remove global `#![allow(clippy::all)]` from terraphim_validation (#2758, PR #2773 in queue) (WIG-4)
- Merge channel active as of 2026-06-20; priority PRs queued: #2772 #2773 #2774 #2778 #2780 #2785 #2804 #2812
