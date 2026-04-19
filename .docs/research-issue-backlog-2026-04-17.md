# Research: Terraphim AI Issue Backlog Analysis

**Date**: 2026-04-17
**Sources**: GitHub Issues (terraphim/terraphim-ai, 212 open) + Gitea Issues (terraphim/terraphim-ai, 212 open, PageRank-scored)
**Context**: FFF Epic #222 recently completed (PR #820). Rust 1.95 clippy fixes merged. CI partially functional. 207 open Gitea issues.

---

## Executive Summary

The Terraphim AI backlog comprises 212 open issues spanning twelve thematic clusters. The highest strategic imperative is to **stabilise the AI Dark Factory (ADF) orchestration layer**, which is currently experiencing a 52% agent timeout rate, 87% disk utilisation, dead CI runners, and cascading security remediation failures. Until ADF is reliable, downstream workstreams -- including the highest-PageRank epic for inter-agent orchestration (#225) and the core learning/KG evolution thesis -- cannot safely advance. The recommended sequence is: (1) Stop the bleeding (security + infrastructure), (2) Stabilise the factory (ADF reliability), (3) Wire the nervous system (inter-agent orchestration), (4) Build the brain (learning + KG), (5) Deliver user value (PA/SO roles), and (6) Harden and scale.

---

## 1. Thematic Clustering of Open Issues

### Cluster A: ADF Reliability & Deployment (26+ issues)
**PageRank Impact**: Very High (dependency hub for most other clusters)
**Status**: Critically unstable

The core orchestration infrastructure is failing. This cluster includes the epic for self-improving agent loops, operational reliability fixes, and all deployment blockers.

| Issue | Title | PageRank | Blocked |
|-------|-------|----------|---------|
| #326 | ADF: Re-enable test-guardian and full deployment | 0.0079 | No |
| #365 | Epic: Auto-Harness Self-Improving Agent Loop | 0.1500* | No |
| #320 | ADF: Config-only agent reliability improvements (A2-A5) | 0.0029 | No |
| #318 | Epic: ADF orchestration remediation | 0.0014 | Yes (5 blockers) |
| #367 | Nightly failure clustering via Flow DAG | 0.0047 | Yes (1 blocker) |
| #366 | Structured AgentRunRecord with ExitClass taxonomy | 0.0054 | No |
| #120 | Implement execution tiers with risk classification | 0.0056 | No |
| #704 | Wire terraphim_agent_evolution into ADF orchestrator | 0.1500* | No |
| #256 | 52% agent timeout rate tuning | 0.1500* | No |
| #255 | TOML config duplicate key crashes | 0.1500* | No |
| #200 | Disk usage at 87% | 0.1500* | No |
| #197 | Cost optimisation via self-improvement loop | 0.1500* | No |
| #321-325 | Drift cooldown, failure cooldown, mention retry, compilation fixes | 0.0014-0.0035 | Mixed |

*Note: Issues with 0.1500 PageRank appear to hit the scoring cap -- they are treated as maximum-impact by the dependency graph.*

**Key insight**: #326 is the gateway to full ADF deployment. #320 (config-only reliability) is unblocked and can be started immediately. #318 is blocked by five sub-issues, making it a bottleneck epic.

---

### Cluster B: Inter-Agent Orchestration & Coordination (12+ issues)
**PageRank Impact**: Highest overall (#225 is 0.0167, 2.2x the next highest)
**Status**: Strategic differentiator, partially blocked by Cluster A

This cluster represents the project's core thesis: agents that coordinate via Gitea mentions, message queues, and shared context.

| Issue | Title | PageRank | Blocked |
|-------|-------|----------|---------|
| #225 | Epic: Inter-agent orchestration via Gitea mentions | **0.0167** | No |
| #160 | Phase 4: Suggestion approval workflow | 0.0094 | No |
| #230 | Phase 5: Webhook-driven mention detection | 0.0075 | No |
| #728 | AgentEvolution haystack ServiceType | -- | No |
| #727 | Wire agent_evolution into orchestrator | -- | No |
| #687 | Steering/follow-up message queues | -- | -- |
| #686 | Typed tool hooks | -- | -- |
| #685 | Cross-provider context serialisation | -- | -- |
| #688 | JSONL RPC envelope | -- | -- |

**Key insight**: #225 has the highest PageRank in the entire backlog because it unblocks the most downstream work. However, it depends on ADF being stable enough to run multi-agent workflows. Starting this before Cluster A stabilises risks compounding failures.

---

### Cluster C: Security, Compliance & Quality Gates (20+ issues)
**PageRank Impact**: High (blocks merges, blocks CI)
**Status**: Existential risk -- must be resolved before any production deployment

A critical vulnerability (RUSTSEC-2026-0049) in rustls-webpki has propagated across multiple PRs, causing security-sentinel, compliance-watchdog, spec-validator, compound-review, and test-guardian failures.

| Issue | Title | Priority | Status |
|-------|-------|----------|--------|
| #448 | Critical vulnerability RUSTSEC-2026-0049 | 26 | Open |
| #451 | security-sentinel FAIL on #360 | **167** | Open |
| #506 | security-sentinel FAIL on #400 | **52** | Open |
| #505 | security-sentinel FAIL on #108 | 6 | Open |
| #518 | security-sentinel FAIL on #428 | 8 | Open |
| #499 | security-sentinel FAIL on #406 | 4 | Open |
| #494 | compliance-watchdog FAIL on #400 | **30** | Open |
| #529 | compliance-watchdog FAIL on #438 | 2 | Open |
| #519 | compliance-watchdog FAIL on #428 | 2 | Open |
| #504 | test-guardian FAIL on #410 | 2 | Open |
| #491 | comprehensive_cli_tests fail (Default role) | 8 | Open |
| #501 | compound-review FAIL on #108 | **10** | Open |
| #498 | compound-review FAIL on #406 | **46** | Open |
| #539 | spec-validator FAIL on #442 | 4 | Open |
| #533 | spec-validator missing business-scenario-design skill | 0 | Open |
| #520 | spec-validator FAIL on #428 | 2 | Open |
| #503 | spec-validator FAIL on #410 | 2 | Open |
| #514 | All 5 GitHub Actions runners STOPPED | 4 | Open |

**Key insight**: RUSTSEC-2026-0049 is not a single fix -- it is a cascading failure across rustls-webpki, cargo audit/deny, and port exposure. Priority 167 on #451 indicates this is the most urgent individual issue. The dead CI runners (#514) amplify the problem by preventing automated verification of fixes.

---

### Cluster D: Learning & Knowledge Graph Evolution (18+ issues)
**PageRank Impact**: Medium-High (long-term value, some blocked)
**Status**: Core product thesis, partially foundational

The self-improving knowledge graph is Terraphim's long-term differentiator. This cluster spans learning capture, cross-agent injection, EIDOS confidence scoring, and shared workspace infrastructure.

| Issue | Title | PageRank | Blocked |
|-------|-------|----------|---------|
| #810 | Learning-driven command correction Phases 2 & 3 | -- | -- |
| #729 | Cross-run compounding via lesson injection | -- | -- |
| #602 | Confidence-driven KG promotion: EIDOS | -- | -- |
| #601 | EIDOS episodic reasoning | -- | -- |
| #600 | Dimensional verdict scoring | -- | -- |
| #265 | Epic: ADF shared learnings | 0.1500* | No |
| #266 | SharedLearning store with trust-gated promotion | 0.1500* | No |
| #330 | Phase 1: SQLite shared learning store | 0.0038 | No |
| #331 | Phase 2: Gitea wiki sync | 0.0014 | Yes (1) |
| #332 | Phase 3: Quality loop | 0.0014 | Yes (1) |
| #328 | Epic: ADF Shared Workspace | 0.1500* | No |
| #513 | KG concept extraction for imported sessions | 0.0026 | Yes (1) |
| #267 | Cross-agent learning injection | 0.0036 | Yes (2) |
| #268 | Shared learning verification loop | 0.0026 | Yes (1) |
| #269 | Learning evolution DAG | 0.0014 | Yes (1) |

**Key insight**: #330 (SQLite shared learning store) is unblocked and foundational. Phases 2 and 3 (#331, #332) are blocked by Phase 1, creating a natural waterfall. The EIDOS work (#600-#602) is strategically important but should follow the shared learning infrastructure.

---

### Cluster E: TinyClaw / Agent SDK / CLI Foundation (14+ issues)
**PageRank Impact**: Medium (user-facing surface)
**Status**: Mixed -- some ready, some blocked

TinyClaw is the markdown-defined command layer. The Agent SDK migration and standalone LLM crate extraction are architectural prerequisites.

| Issue | Title | Status |
|-------|-------|--------|
| #592 | TinyClaw markdown-defined commands | Open |
| #591 | TinyClaw session tools and orchestration | Open |
| #590 | TinyClaw OpenClaw parity epic | Open |
| #588 | TinyClaw foundation hardening | Open |
| #689 | Migrate ADF Claude agents to Agent SDK | Open |
| #684 | Extract standalone LLM interaction crate | Open |
| #671 | TypeScript bindings for TLA+ | Open |
| #635 | Session handoff CLI | 0.0026 | No |
| #492 | Wire REPL robot/format flags | 0.0014 | Yes (1) |
| #466 | Token budget management for robot mode | 0.0014 | Yes (1) |
| #123 | Self-learning architecture with feedback loops | 0.0026 | No |
| #511 | Phase 1 test suite for robot mode | 0.0038 | No |

---

### Cluster F: Personal Assistant (PA) / System Operator (SO) Roles (10+ issues)
**PageRank Impact**: Medium (active deliverable)
**Status**: Several issues unblocked and ready for execution

An active workstream with clear deliverables. The PA role uses JMAP + Obsidian; the SO role handles durable paths and embedded config.

| Issue | Title | PageRank | Blocked | Ready |
|-------|-------|----------|---------|-------|
| #730 | PA: JMAP + Obsidian end-to-end epic | 0.0014 | Yes (7) | No |
| #731 | PA: rebuild terraphim-agent with jmap | 0.0066 | No | Yes |
| #732 | PA: add PA role to embedded_config | 0.0059 | Yes (1) | No |
| #733 | PA: wrapper script ~/bin/terraphim-agent-pa | 0.0051 | Yes (1) | No |
| #739 | PA: populate Obsidian vault | 0.0016 | No | Yes |
| #742 | SO: clone to durable path + add role | 0.0059 | No | Yes |
| #741 | SO demo refresh | 0.0014 | Yes (4) | No |
| #734 | PA: how-to docs | 0.0041 | Yes (1) | No |
| #735 | PA: blog post | 0.0029 | Yes (1) | No |

**Key insight**: #731, #739, and #742 are unblocked and ready to execute. #732 and #733 are blocked by #731, forming a clear chain. This cluster can proceed in parallel with Cluster A stabilisation because it touches different code paths (agent roles vs orchestrator core).

---

### Cluster G: Session Management & Cross-Agent Search (10+ issues)
**PageRank Impact**: Medium (enables PA/SO and learning)
**Status**: Mostly unblocked

| Issue | Title | PageRank | Blocked |
|-------|-------|----------|---------|
| #683 | Tree-structured session storage | -- | -- |
| #597 | Session store with real-time event capture | -- | -- |
| #611 | Sessions files and sessions by-file | -- | -- |
| #639 | Session persistence for Claude Code agents | -- | -- |
| #566 | Aider and Cline session connectors | 0.1500* | No |
| #465 | Full-text index for session search | 0.1500* | No |
| #512 | Session CLI commands | 0.0036 | No |
| #513 | KG concept extraction for imported sessions | 0.0026 | Yes (1) |
| #567 | Concept-based session discovery | 0.0014 | Yes (1) |

---

### Cluster H: Knowledge Graph & Content Parsers (8+ issues)
**PageRank Impact**: Medium
**Status**: Community integration focus

| Issue | Title | Status |
|-------|-------|--------|
| #651 | Context-hub haystack connector | Open |
| #604 | Obsidian format parser | Open |
| #607 | Logseq community plugin | Open |
| #606 | Obsidian community plugin | Open |
| #603 | Epic: Obsidian & Logseq community integration | Open |
| #605 | Concept relationship layer: wikilink edges | Open |
| #154 | Exa web search as Haystack retriever | 0.0037 | No |
| #155 | Integrate Exa search | 0.0020 | Yes (1) |

---

### Cluster I: Performance, Infrastructure & Tooling (10+ issues)
**PageRank Impact**: Medium-High (developer velocity)
**Status**: Mixed -- some critical, some deferred

| Issue | Title | Priority | Status |
|-------|-------|----------|--------|
| #486 | zlob 1.3.0 fails to build on macOS | 2 | Open |
| #514 | All 5 GitHub Actions runners STOPPED | 4 | Open |
| #351 | Missing systemd service unit | 0 | Open |
| #200 | Disk usage at 87% | 18 | Open |
| #178 | Web terminal demo | 25 | Open |
| #350 | Dynamic provider benchmark tool | 0 | Open |
| #232 | Criterion benchmarks for symbolic embeddings | 0 | Open |
| #231 | SNOMED/MeSH ontology benchmark | 0 | Open |
| #416 | Ship ADF agent logs to Quickwit | 64 | Open |
| #284-309 | terraphim_usage crate (token tracking) | 0 | Mixed |

---

### Cluster J: Marketing, Content & Community Launch (18+ issues)
**PageRank Impact**: Low-Medium (visibility)
**Status**: Mostly blocked by product readiness

| Issue | Title | PageRank | Blocked |
|-------|-------|----------|---------|
| #608 | Community launch | -- | -- |
| #630 | docs.terraphim.ai cleanup | -- | -- |
| #644 | Terraphim Promotion Campaign | 0.0036 | Yes (4) |
| #645-648 | Articles 1-4 | 0.0022 | Mixed |
| #721-745 | Story spine / blog posts / syndication | 0.0014-0.0032 | Mixed |
| #94 | CTO Blog: GPU KG vs Deterministic Automata | 0.1500* | No |

---

### Cluster K: TLA+ Formal Verification (12+ issues)
**PageRank Impact**: Low-Medium (correctness assurance)
**Status**: Deep, technical debt

| Issue | Title | Priority | Blocked |
|-------|-------|----------|---------|
| #349 | Epic: TLA+ formal verification | 15 | Yes (2) |
| #340-348 | 9 TLA+ bugs (Supervisor, Symphony, Messaging) | 40 | Mixed |
| #671 | TypeScript bindings for TLA+ | -- | -- |

---

### Cluster L: Token Tracking, Usage & Cost Management (12+ issues)
**PageRank Impact**: Medium (operational visibility)
**Status**: Well-scoped, partially implemented

| Issue | Title | Blocked |
|-------|-------|---------|
| #298-309 | terraphim_usage crate steps | Mixed |
| #286 | Implement ADF token tracking | No |
| #197 | Cost optimisation via self-improvement | No |
| #280 | Populate token counts in flow executor | No |
| #308 | Gitea cost attribution | No |

---

## 2. The Vital Few: Top 7 Highest-Impact Clusters

Using a composite score weighing PageRank dependency impact, strategic value, feasibility, and risk reduction, the Vital Few clusters are:

| Rank | Cluster | Composite Score | Rationale |
|------|---------|-----------------|-----------|
| 1 | **C: Security & Compliance Remediation** | 9.5/10 | Existential risk. Blocks all merges. CI is dead. RUSTSEC-2026-0049 is critical. |
| 2 | **A: ADF Reliability & Deployment** | 9.2/10 | 52% timeout rate, 87% disk, TOML crashes. Without this, nothing else ships. |
| 3 | **B: Inter-Agent Orchestration** | 8.8/10 | Highest PageRank (#225 = 0.0167). Strategic differentiator. Blocked by A. |
| 4 | **D: Learning & KG Evolution** | 7.5/10 | Core thesis. #330 is unblocked foundational work. Long-term value. |
| 5 | **F: PA/SO Roles** | 7.0/10 | Active deliverable with unblocked issues. Near-term user value. |
| 6 | **G: Session Management** | 6.5/10 | Enables PA/SO and learning. #512, #566, #465 are ready. |
| 7 | **E: TinyClaw / Agent SDK** | 6.0/10 | User-facing surface. #511 test suite is ready. #689 is architectural. |

**Why not others?**
- **Cluster I (Infrastructure)**: Important but mostly subsumed into Cluster A/C (disk, CI, security).
- **Cluster H (KG Parsers)**: Valuable but blocked by core infrastructure.
- **Cluster J (Marketing)**: Premature until product is stable.
- **Cluster K (TLA+)**: Critical for correctness but can be deferred until core is stable.
- **Cluster L (Token Tracking)**: Operational visibility; not on the critical path to product stability.

---

## 3. Dependency Analysis Between Clusters

### Critical Path

```
[C] Security Remediation
    |
    v
[A] ADF Reliability -----> [B] Inter-Agent Orchestration
    |                           |
    |                           v
    |                      [D] Learning & KG
    |                           |
    |                           v
    +--------------------> [G] Session Management
                                |
                                v
                           [F] PA/SO Roles
                                |
                                v
                           [E] TinyClaw / SDK
```

### Key Dependency Chains

1. **RUSTSEC-2026-0049 -> CI -> Everything**
   - #448/#451 (security) must be resolved to restore merge capability.
   - #514 (dead CI runners) must be fixed to verify any changes.
   - These are **zero-order blockers** for all other clusters.

2. **ADF Reliability -> Inter-Agent Orchestration**
   - #225 (inter-agent orchestration) depends on agents not crashing 52% of the time.
   - #320 (config-only reliability) is a prerequisite for #326 (test-guardian re-enable).
   - #367 (nightly failure clustering) depends on #366 (AgentRunRecord).

3. **SQLite Learning Store -> Cross-Agent Learning**
   - #330 (SQLite store) unblocks #331 (wiki sync), #332 (quality loop), #267 (cross-agent injection).
   - This is a natural waterfall -- execute in sequence.

4. **PA Rebuild -> PA Role -> PA Script/Docs**
   - #731 (rebuild with jmap) -> #732 (embedded_config role) -> #733 (wrapper script) / #734 (how-to).
   - #742 (SO clone) can proceed in parallel with #731.

5. **Session CLI -> KG Extraction -> Concept Discovery**
   - #512 (session CLI) -> #513 (KG concept extraction) -> #567 (concept discovery).

### Cross-Cluster Synergies

- **Cluster A + D**: The self-improving agent loop (#365) generates the failure data that feeds the nightly clustering (#367) and AgentRunRecord taxonomy (#366).
- **Cluster B + G**: Inter-agent orchestration (#225) requires session connectors (#566) to share context across agents.
- **Cluster F + H**: The PA role (#730) depends on Obsidian format parser (#604) and context-hub (#651).

---

## 4. Blocked vs Unblocked Work

### Immediately Executable (Unblocked, High Value)

| Issue | Cluster | Action |
|-------|---------|--------|
| #451 | C | Fix RUSTSEC-2026-0049 rustls-webpki upgrade |
| #514 | C | Restore GitHub Actions runners |
| #320 | A | Config-only agent reliability (A2-A5) |
| #366 | A | Structured AgentRunRecord with ExitClass |
| #120 | A | Execution tiers with risk classification |
| #225 | B | Inter-agent orchestration via Gitea mentions |
| #330 | D | SQLite shared learning store |
| #731 | F | Rebuild terraphim-agent with jmap |
| #742 | F | SO: clone to durable path |
| #739 | F | Populate Obsidian vault |
| #512 | G | Session CLI commands |
| #566 | G | Aider and Cline session connectors |
| #465 | G | Full-text index for session search |
| #511 | E | Phase 1 test suite for robot mode |
| #154 | H | Exa web search as Haystack retriever |

### Blocked by Dependencies (Do Not Start Yet)

| Issue | Cluster | Blocked By |
|-------|---------|------------|
| #318 | A | #321, #322, #323, #324, #319 |
| #326 | A | Partially blocked by #320, #324 |
| #331 | D | #330 |
| #332 | D | #330 |
| #267 | D | #266, #330 |
| #268 | D | #266 |
| #269 | D | #266 |
| #732 | F | #731 |
| #733 | F | #731 |
| #734 | F | #731 |
| #730 | F | #731, #732, #733, #734, #735, #736, #739 |
| #513 | G | #512 |
| #567 | G | #513 |
| #155 | H | #154 |
| #349 | K | #340-#348, #346 |

---

## 5. Risks and Unknowns

### Critical Risks

1. **Cascading ADF Architecture Failure**
   - *Observation*: 52% timeout rate (#256) + disk at 87% (#200) + TOML crashes (#255) + worktree race (#559).
   - *Risk*: These may not be independent bugs but symptoms of a deeper architectural issue (e.g., unbounded concurrency, missing backpressure, resource leaks).
   - *Mitigation*: Treat #320 (config-only reliability) as a diagnostic spike. Profile memory and disk usage before adding features.

2. **Security Debt Spiral**
   - *Observation*: RUSTSEC-2026-0049 affects rustls-webpki, which is a transitive dependency. Upgrading it may force upgrades of hyper, reqwest, and other core crates.
   - *Risk*: A "simple" security fix could trigger a dependency cascade requiring days of API migration.
   - *Mitigation*: Run `cargo audit` and `cargo tree` to map the blast radius before committing to an upgrade path. Consider `cargo update --precise` if a patch version is available.

3. **CI Infrastructure Decay**
   - *Observation*: All 5 GitHub Actions runners are stopped (#514). The `lint-and-format` job has a pre-existing failure (fff-search build.rs CI detection).
   - *Risk*: Without CI, every merge is a manual quality gate. This scales poorly with 212 open issues.
   - *Mitigation*: Prioritise #514 above feature work. The fff-search build.rs issue may require a separate feature flag fix.

4. **Scope Explosion via Epics**
   - *Observation*: There are at least 8 active epics (#225, #265, #328, #365, #590, #603, #730, #349), each with 5-15 sub-issues.
   - *Risk*: Epics can become infinite sinks. Without strict phase gates, sub-issues multiply faster than they close.
   - *Mitigation*: Apply the "sprint within an epic" pattern. Close #330 before opening #331. Close #731 before opening #732.

5. **Cross-Platform Build Breakage**
   - *Observation*: zlob 1.3.0 fails on macOS (#486). The `lint-and-format` job fails on CI due to build.rs logic.
   - *Risk*: Developer onboarding is blocked for macOS users. This shrinks the contributor pool.
   - *Mitigation*: Fix #486 immediately. It is isolated from other work.

### Unknowns Requiring Investigation

1. **Root Cause of 52% Timeout Rate**
   - Is this provider-side (OpenAI Codex session limits per #716), orchestrator-side (missing timeout tuning), or agent-side (infinite loops)?
   - *Action*: Analyse timeout logs before tuning (#256).

2. **Actual Dependency Graph of RUSTSEC-2026-0049**
   - How many crates need version bumps? Is there a compatible patch release?
   - *Action*: Run `cargo audit --json` and map affected crates.

3. **Disk Usage Breakdown**
   - Is the 87% disk from build artifacts, temp files, or LMDB stores?
   - *Action*: Run `du -sh target/`, `du -sh /tmp/`, `du -sh ~/.cache/` on the runner.

4. **PageRank 0.15 Ceiling**
   - Many issues show PageRank 0.1500, suggesting a scoring cap. Are these truly equal in impact, or is the graph resolution insufficient?
   - *Action*: Review gitea-robot graph weights for high-degree nodes.

---

## 6. Recommended Prioritisation Sequence

### Phase 1: Stop the Bleeding (Week 1)
**Goal**: Restore the ability to merge code safely.
**Clusters**: C (Security), I (Infrastructure)

1. **#451 + #448**: Fix RUSTSEC-2026-0049 (rustls-webpki upgrade)
2. **#514**: Restore GitHub Actions runners
3. **#486**: Fix zlob macOS build
4. **#200 + #319**: Disk cleanup (cargo clean, temp removal)
5. **#529 + #494**: Fix compliance-watchdog license fields
6. **#504 + #491**: Fix test-guardian Default role failures

**Exit Criteria**: `cargo audit` passes, CI is green, `cargo build --workspace` passes on macOS and Linux.

---

### Phase 2: Stabilise the Factory (Weeks 2-3)
**Goal**: Make ADF reliable enough to run agents without 52% failure.
**Clusters**: A (ADF Reliability)

1. **#320**: Config-only agent reliability (A2-A5) -- unblocked, quick wins
2. **#255**: Fix TOML config duplicate key crashes
3. **#256**: Diagnose and tune 52% timeout rate
4. **#321-#324**: Drift cooldown, failure cooldown, mention retry, compilation fixes
5. **#366**: Structured AgentRunRecord with ExitClass taxonomy
6. **#120**: Execution tiers with risk classification
7. **#326**: Re-enable test-guardian and full deployment

**Exit Criteria**: Agent timeout rate below 10%, test-guardian passes, ADF deploys successfully to D1-D3.

---

### Phase 3: Wire the Nervous System (Weeks 4-5)
**Goal**: Enable agents to coordinate via Gitea.
**Clusters**: B (Inter-Agent Orchestration)

1. **#225**: Epic -- Inter-agent orchestration via Gitea mentions
2. **#230**: Phase 5 -- Webhook-driven mention detection
3. **#160**: Phase 4 -- Suggestion approval workflow
4. **#728**: AgentEvolution haystack ServiceType
5. **#727**: Wire agent_evolution into ADF orchestrator
6. **#704**: Wire terraphim_agent_evolution into ADF (if not done in Phase 2)

**Exit Criteria**: Two or more agents can coordinate on a single issue via Gitea mentions with approval workflow.

---

### Phase 4: Build the Brain (Weeks 6-8)
**Goal**: Implement the self-improving learning loop.
**Clusters**: D (Learning), G (Sessions)

1. **#330**: Phase 1 -- SQLite shared learning store
2. **#512**: Session CLI commands
3. **#566**: Aider and Cline session connectors
4. **#266**: SharedLearning store with trust-gated promotion
5. **#331**: Phase 2 -- Gitea wiki sync for L2+ learnings
6. **#332**: Phase 3 -- Quality loop and verification
7. **#602**: EIDOS confidence-driven KG promotion
8. **#810**: Learning-driven command correction Phases 2 & 3

**Exit Criteria**: Cross-agent learning injection works end-to-end; sessions are searchable across Aider, Cline, and Claude Code.

---

### Phase 5: Deliver User Value (Weeks 9-10)
**Goal**: Ship the PA and SO roles.
**Clusters**: F (PA/SO), H (KG Parsers)

1. **#731**: PA: rebuild terraphim-agent with jmap
2. **#742**: SO: clone to durable path
3. **#732**: PA: add role to embedded_config
4. **#739**: PA: populate Obsidian vault
5. **#733**: PA: wrapper script
6. **#604**: Obsidian format parser
7. **#651**: Context-hub haystack connector

**Exit Criteria**: A user can install terraphim-agent, configure the PA role, and search their Obsidian vault via JMAP.

---

### Phase 6: Harden and Scale (Weeks 11-12+)
**Goal**: Formal verification, community launch, and content.
**Clusters**: E (TinyClaw), K (TLA+), J (Marketing), L (Usage)

1. **#588-#592**: TinyClaw foundation and OpenClaw parity
2. **#340-#348**: Fix TLA+ bugs
3. **#349**: TLA+ formal verification epic
4. **#286**: ADF token tracking
5. **#644-#648**: Content campaign
6. **#608**: Community launch

---

## 7. Synthesis: Key Trade-offs

### Speed vs Safety
- **Risk**: Pushing inter-agent orchestration (Phase 3) before ADF stabilisation (Phase 2) could compound the 52% failure rate across multiple coordinated agents.
- **Resolution**: Strict phase gating. Do not start #225 until #326 (test-guardian re-enabled) is green.

### Breadth vs Depth
- **Risk**: 8 active epics with 212 issues risks shallow progress everywhere.
- **Resolution**: The PA/SO cluster (F) can proceed in parallel with ADF stabilisation (A) because it uses different code paths. All other clusters should wait their phase.

### Technical Debt vs Features
- **Risk**: The TLA+ bugs (#340-#348) and security fixes (#448-#451) are not user-visible but block correctness.
- **Resolution**: Front-load debt in Phase 1-2. Defer TLA+ epic to Phase 6, but fix individual TLA+ bugs that affect runtime behaviour (e.g., #346 dispatch error paths) in Phase 2.

---

## 8. Metrics for Success

| Phase | Key Metric | Target |
|-------|------------|--------|
| 1 | CI pass rate | 100% |
| 1 | `cargo audit` findings | 0 critical |
| 2 | Agent timeout rate | < 10% |
| 2 | Disk utilisation | < 70% |
| 3 | Multi-agent coordination success | > 80% |
| 4 | Cross-agent learning injection latency | < 5s |
| 5 | PA role setup time | < 10 min |
| 6 | TLA+ invariant coverage | > 90% |

---

## Appendix A: Raw PageRank Data (Top 15)

| Rank | Issue | Title | PageRank |
|------|-------|-------|----------|
| 1 | #225 | Epic: Inter-agent orchestration via Gitea mentions | 0.0167 |
| 2 | #160 | Phase 4: Suggestion approval workflow | 0.0094 |
| 3 | #326 | ADF: Re-enable test-guardian and full deployment | 0.0079 |
| 4 | #230 | Phase 5: Webhook-driven mention detection | 0.0075 |
| 5 | #731 | PA: rebuild terraphim-agent with jmap | 0.0066 |
| 6 | #732 | PA: add PA role to embedded_config | 0.0059 |
| 7 | #742 | SO: clone to durable path + add role | 0.0059 |
| 8 | #120 | Implement execution tiers with risk classification | 0.0056 |
| 9 | #366 | Structured AgentRunRecord with ExitClass | 0.0054 |
| 10 | #733 | PA: wrapper script | 0.0051 |
| 11 | #367 | Nightly failure clustering via Flow DAG | 0.0047 |
| 12 | #154 | Exa web search as Haystack retriever | 0.0037 |
| 13 | #644 | Terraphim Promotion Campaign | 0.0036 |
| 14 | #512 | Session CLI commands | 0.0036 |
| 15 | #260 | SkillRegistry with progressive disclosure | 0.0032 |

---

## Appendix B: Recently Completed Context

- **PR #820** (merged): FFF Epic #222 -- multi_grep, frecency persistence, cursor pagination
- **Rust 1.95 clippy fixes**: 49 files across ~20 crates
- **Gitea issues closed**: #222, #224, #225, #226
- **Pre-existing CI failure**: `lint-and-format` job broken due to fff-search build.rs CI detection (not caused by PR #820)

---

*Document generated on 2026-04-17 using Gitea PageRank dependency graph + GitHub issue metadata. PageRank scores reflect downstream issue unblock potential -- higher scores indicate fixes that enable more subsequent work.*
