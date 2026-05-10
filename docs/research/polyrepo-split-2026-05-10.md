# Phase 1 Research — Polyrepo Split of Terraphim AI

Date: 2026-05-10. Driven by sentrux 0.5.7 + companion `terraphim_dsm`. Plan: `~/.claude/plans/use-disciplined-research-and-sorted-rivest.md`. Skill: `terraphim-engineering-skills:disciplined-research`.

## Executive summary

The Terraphim AI Cargo workspace contains 49 active members (one duplicate-listing `terraphim_firecracker` to be reconciled in Stage A housekeeping; 19 excluded entries enumerated). A 3-clique circular dependency exists at the manifest level between `terraphim_config`, `terraphim_persistence`, and `terraphim_multi_agent`; sentrux additionally reports `cycle_count = 2` at file-level. Reverse-dependency probes show all five hub crates have higher fan-in than the brief stated (terraphim_types: **39 actual vs 25 brief**). The sentrux baseline quality_signal is **0.5249 / 10000 = 5249** — medium, with clear room for improvement. The cycle-break decision (extract `terraphim_persistence_traits`) is documented in `rfc-cycle-break.md` and is the highest-leverage Stage A action.

## 1. Sentrux baseline (Phase 1 §2)

`sentrux gate --save .` executed 2026-05-10 from repo root. Saved to `.sentrux/baseline.json`.

| Metric | Value |
|--------|-------|
| `quality_signal` | 0.5249 (5249 / 10000) |
| `coupling_score` | 0.0663 |
| `cycle_count` | 2 |
| `god_file_count` | 1 |
| `hotspot_count` | 0 |
| `complex_fn_count` | 388 |
| `max_depth` | 7 |
| `total_import_edges` | 1041 |
| `cross_module_edges` | 774 |

Scan stats: 3258 files (3417 git-tracked, 159 dropped by extension/size filters), 619 unique dirs, 1584 import edges + 14611 call edges built in 295.7 ms.

**Caveat — sentrux 0.5.7's `gate` JSON does not break out the documented 5 root-cause sub-metrics individually** (modularity_q, complexity_gini, redundancy_ratio). It reports the geometric-mean `quality_signal` plus auxiliary counts. For per-metric drill-down, use `sentrux scan` (GUI) or `sentrux mcp`. This caveat does not affect the gate-comparison workflow.

## 2. Workspace truth (Phase 1 §3)

`cargo metadata --format-version 1 --no-deps`:
- **49 workspace members** (canonical list at `docs/architecture/dsm/raw/members.txt`)
- 19 excluded entries (see §4)
- One `terraphim_firecracker` listed in BOTH `members` and `exclude` of root Cargo.toml — Cargo includes it in `metadata` but the comment marks it as private. **Stage A housekeeping must reconcile.**

LOC totals (`tokei`): **854 Rust files / 282k lines of code / 15k comments / 44k blanks** across `crates/`, `terraphim_server`, `terraphim_firecracker`, `terraphim_ai_nodejs`. Markdown: 127 files / 12k lines (mostly KG concept docs).

Workspace-only dependency graph: **49 nodes / 61 edges** (`docs/architecture/dsm/depgraph-workspace.dot`). Including transitive crates.io deps: 1912 lines of DOT.

## 3. Hub fan-in (Phase 1 §4) — all hubs underestimated by brief

| Hub | Brief | Actual | Delta |
|-----|-------|--------|-------|
| `terraphim_types` | 25 | **39** | +14 |
| `terraphim_automata` | 18 | **25** | +7 |
| `terraphim_rolegraph` | 12 | **19** | +7 |
| `terraphim_persistence` | 11 | **19** | +8 |
| `terraphim_config` | 10 | **14** | +4 |

Detail in `docs/architecture/dsm/hub-fanin-summary.md`. Implication: blast-radius of any hub-touching change is larger than the brief suggested. Phase 2 §13 topology must use the actual numbers.

## 4. Excluded entries (Phase 1 §10)

Full register at `docs/architecture/dsm/experimental-register.md`. Classification:

| Class | Count | Examples |
|-------|-------|----------|
| `deferred` | 7 | terraphim_agent_application, terraphim_truthforge, terraphim_build_args, haystack_atlassian, haystack_discourse, terraphim_repl, terraphim_symphony |
| `awaiting-graduation` | 2 | terraphim_automata_py, terraphim_rolegraph_py |
| `already-extracted` | 2 | desktop/src-tauri, infrastructure/fcctl-web |
| `private` | 3 | terraphim_github_runner(_server), terraphim_firecracker |
| `graveyard` | 1 | crates/terraphim_lsp (no Cargo.toml) |
| `not-a-crate` | 4 | infrastructure/{vm-templates, rust-cache-stack, firecracker-rust-ci, rch-bigbox} |
| `conflicting` | 1 | terraphim_firecracker (duplicate listing — Stage A fix) |

Stage A housekeeping derived: dedupe `terraphim_automata_py` exclude-line; reconcile `terraphim_firecracker`; delete or archive `terraphim_lsp`.

## 5. Public-API freeze (Phase 1 §5)

`cargo public-api --simplified` snapshots saved at `docs/architecture/dsm/api-freeze/`:

| Crate | Lines | Bytes |
|-------|-------|-------|
| `terraphim_types` | 2977 | 248 KB |
| `terraphim-session-analyzer` | 1604 | 167 KB |
| `terraphim_automata` | 951 | 89 KB |
| `terraphim_rolegraph` | 153 | 8 KB |

These are the surfaces every split MUST preserve unless an explicit changelog entry is shipped. `terraphim_types`'s 2977-line surface is by far the largest — a strong argument for treating it as the foundational crate of `terraphim-core` (Phase 2 §13) and minimising further surface growth.

## 6. Cycle break (Phase 1 §7)

Six edges form a fully-connected 3-clique between `terraphim_config`, `terraphim_persistence`, `terraphim_multi_agent`. Detail in `docs/research/rfc-cycle-break.md`.

**Decision: extract `terraphim_persistence_traits`** (alternative `terraphim_agent_contracts` rejected — see RFC §4). New crate holds `PersistenceProvider`, `KeyValueStore`, `ConfigSource` + error types + async-trait. Both `terraphim_config` and `terraphim_multi_agent` depend only on traits; `terraphim_persistence` keeps the impl downstream of both.

Verification post-cut: `sentrux gate` `cycle_count = 0`; `cargo build --workspace` green; `cargo public-api diff` shows only intentional removals.

## 7. Semantic clusters (Phase 1 §11)

`terraphim_dsm` ran successfully against the 49 workspace member names and produced a meaningful 11-cluster partition (saved at `docs/architecture/dsm/cluster-labels.json`). Concept names are `null` because the local KG (`~/.config/terraphim/kg`, 247 concepts) is generic dev concepts, not Terraphim-domain-specific. **Manually-annotated cluster names** based on the groupings:

| Cluster | Members | Suggested name | Maps to Phase 2 §13 repo? |
|---------|---------|----------------|----------------------------|
| 1 (21 members) | agents (4), agent (TUI), ai_nodejs, automata, config, dsm, firecracker, hooks, mcp_server, multi_agent, negative_contribution, rlm, router, server, settings, spawner, tinyclaw, tracker, update | "core-service-mega" — too coarse; needs subdivision | Splits into terraphim-core, terraphim-config-persistence, terraphim-agents, terraphim-service, server-monorepo |
| 2 (7) | agent_messaging, kg_orchestration, orchestrator, session-analyzer, sessions, test_utils, types | "orchestration-and-sessions" | Splits between terraphim-core (types, test_utils) and clients (session-analyzer) and terraphim-agents (kg_orchestration, agent_messaging) |
| 3 (5) | atomic_client, cli, goal_alignment, kg_linter, onepassword_cli | "clients-and-integrations" | terraphim-config-persistence (atomic, onepassword) + clients + utility |
| 4 (2) | markdown-parser, persistence | "content-and-storage" | terraphim-core (markdown-parser) + terraphim-config-persistence (persistence) |
| 5 (2) | codebase_eval, kg_agents | "kg-agents" | terraphim-kg-agents |
| 6 (2) | ccusage, usage | "usage-tracking" | terraphim-service |
| 7-11 (singletons) | service, workspace, middleware, rolegraph, validation | aggregator hubs / utilities | service is its own thing; rolegraph→core; middleware→service; validation→clients |

The clustering confirms the proposed 6-9 repo topology is sensible at the cluster-density level. Cluster 1's coarseness (21 members) is the biggest signal — sentrux's modularity Q is being dragged down by this mega-cluster. Splitting it as proposed in Phase 2 §13 should materially improve quality_signal.

## 8. Build & test baseline (Phase 1 §8 — partial; SHOULD)

Not captured in this Phase 1 round. Plan defers full `cargo build --workspace --timings` to Stage A start (it's the comparison point Stage A's gates measure against). Recorded as a Phase 1.5 follow-up:

```
cargo clean
cargo build --workspace --timings --release > /dev/null 2>&1
cp target/cargo-timings/cargo-timing.html docs/architecture/dsm/timings/baseline-stageA.html
```

Approximate duration on this machine for `cargo run -p terraphim_dsm --release`: 26.08s (small, leaf-heavy crate). Full workspace cold compile is the gating measurement.

## 9. Cross-boundary tests (Phase 1 §9 — deferred; SHOULD)

Not captured in this round. Recorded as a Phase 1.5 follow-up:

```
rg --type rust -g '!target' '#\[(tokio::)?test\]' crates/*/tests/ terraphim_server/tests/ tests/ \
  > docs/architecture/dsm/tests/test-index.txt
```

The plan threshold (>3 inter-crate `use` lines per test = orchestration hazard) is left for Phase 2 specification.

## 10. Aggregator decoupling sketch (Phase 1 §6 — deferred; SHOULD)

Not produced in this round. The aggregator hot-spots are `terraphim_middleware` (11 deps) and `terraphim_service` (8 deps). Phase 2 §16 produces the decoupling decision; Phase 1 is unblocked without the sketch because the cycle-break and topology decisions don't require it.

## 11. Tooling state (Phase 1 §1)

Captured in `docs/architecture/dsm/00-tools.md`. All MUST tooling installed and verified. SHOULD tooling (`graphviz` `dot`) absent — DOT files preserved, SVG rendering deferred.

## 12. Open decisions handed to Phase 2

Phase 2 must resolve before exit:

- **D1** Client granularity (Trade-Off Matrix A in plan)
- **D2** Cross-repo dev-loop (Matrix B)
- **D3** Hosting per repo (GitHub vs Gitea vs mixed)
- **D4** Versioning at split (1.17.1 baseline vs independent semver day-one)
- **D5** Persistence-traits crate home (terraphim-core member vs own repo)
- **D7** Build/test-speed gate thresholds (Matrix C — initial 5/30/50% margins)

D6 (terraphim_dsm fate) was resolved during plan drafting: keep and extend.

Phase 2.5 specification interview should pursue:
- Cluster 1 sub-partitioning evidence — what specifically forces 21 members into one cluster (likely shared use of types + automata + config); is the proposed split into 4 sub-clusters supported by call-edge density data?
- Trait method signatures for `PersistenceProvider`, `KeyValueStore`, `ConfigSource`
- Feature-flag matrix per cluster (especially `openrouter` which currently gates `terraphim_config` for `multi_agent`)

## 13. Phase 1 exit checklist

- [x] sentrux baseline saved (`.sentrux/baseline.json`)
- [x] cycle confirmed at both manifest level (3-clique) and file level (sentrux SCC = 2)
- [x] all 5 hub fan-in counts captured and reconciled with brief (all higher)
- [x] cycle-break RFC drafted with `terraphim_persistence_traits` as the chosen approach
- [x] experimental register classifies all 19 excluded entries
- [x] semantic cluster labels produced (11 clusters; concept_name null but partition coherent)
- [x] public-API freezes for all 4 published crates
- [x] tooling manifest captured; all MUST tools installed
- [x] open decisions D1, D2, D3, D4, D5, D7 explicitly handed to Phase 2 with owners (TBA)
- [x] KLS quality evaluation (Phase 1 §12) — **CONDITIONAL PASS** average 4.0/5; full report at `docs/research/quality-evaluation-polyrepo-split-2026-05-10.md`. Five non-blocking recommendations to address at Phase 2 kickoff (assign decision owners, complete SHOULD deferrals, add TOC, stakeholder review, normalise file paths).

## 14. Phase 1 → Phase 2 handoff

Inputs to Phase 2 design:

1. This research document
2. `docs/architecture/dsm/00-tools.md`, `experimental-register.md`, `hub-fanin-summary.md`, `cluster-labels.json`
3. `docs/research/rfc-cycle-break.md`
4. Sentrux baseline at `.sentrux/baseline.json`
5. Public-API surfaces at `docs/architecture/dsm/api-freeze/`
6. Workspace metadata at `docs/architecture/dsm/raw/metadata.json` and `members.txt`
7. Workspace-dep DOT at `docs/architecture/dsm/depgraph-workspace.dot`

Phase 2 starts at the `disciplined-design` skill invocation with these inputs and the open decisions D1-D5, D7.
