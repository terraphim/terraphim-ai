# Documentation Gap Report

**Generated:** 2026-04-29 11:43 CEST
**Agent:** Ferrox (documentation-generator)
**Workspace:** terraphim-ai

## Summary

| Category | Count | Severity |
|----------|-------|----------|
| Broken doc links | 14 | Medium |
| Missing crate-level docs | 24 | Medium |
| Unclosed HTML tags | 7 | Low |
| Non-hyperlinked URLs | 6 | Low |
| Links to private items | 2 | Low |

**Total warnings:** 43

---

## 1. Broken Documentation Links (14)

### terraphim_orchestrator (6 links)
- `RoutingDecisionEngine::decide_route` -- unresolved link
- `DispatchTask::AutoMerge` -- unresolved link (3 occurrences)
- `AgentOrchestrator::poll_pending_reviews` -- links to private item `reconcile_tick`
- `GateConfig` -- unresolved link
- `handle_post_merge_test_gate_for_project` -- unresolved link

### terraphim_file_search (1 link)
- `ScoringContext` -- unresolved link

### terraphim_service (1 link)
- `kg:term` -- unresolved link (likely needs proper module path)

### terraphim_middleware (4 links)
- `with_change_notifications` -- unresolved link
- `Message` -- unclosed HTML tag (also affects rendering)

### terraphim_router (1 link)
- `set_commit_status` -- unresolved link

### terraphim_tracker (1 link)
- `new` -- unresolved link

### terraphim_persistence (2 links)
- `from_serializable` -- unresolved link

### terraphim_rolegraph (2 links)
- `HgncGene` -- unresolved link
- `HgncNormalizer` -- unresolved link

### terraphim_types (3 links)
- Multiple unresolved (see cargo fix suggestion)

---

## 2. Missing Crate-Level Documentation (24 crates)

The following crates lack a top-level `//!` module doc comment in `src/lib.rs`:

### Haystack crates
- `haystack_atlassian`
- `haystack_core`
- `haystack_discourse`
- `haystack_grepapp`
- `haystack_jmap`

### Agent & AI crates
- `terraphim_agent`
- `terraphim_agent_application`
- `terraphim_agent_evolution`
- `terraphim_agent_messaging`
- `terraphim_agent_registry`
- `terraphim_agent_supervisor`

### Core infrastructure
- `terraphim_atomic_client`
- `terraphim_automata_py`
- `terraphim_build_args`
- `terraphim_ccusage`
- `terraphim_config`
- `terraphim_file_search`
- `terraphim_kg_linter`
- `terraphim_lsp`
- `terraphim-markdown-parser`
- `terraphim_mcp_server`
- `terraphim_middleware`
- `terraphim_onepassword_cli`
- `terraphim_persistence`
- `terraphim_rolegraph`
- `terraphim_rolegraph_py`
- `terraphim_service`
- `terraphim_settings`
- `terraphim_usage`

---

## 3. HTML / Rendering Issues (7)

### Unclosed HTML tags
- `terraphim_orchestrator`: `<name>` (4 occurrences)
- `terraphim_orchestrator`: `<HandoffContext>` (1 occurrence)
- `terraphim_middleware`: `<Message>` (1 occurrence)
- `terraphim_persistence`: `<DeviceStorage>` (2 occurrences)

### Non-hyperlinked URLs
- `terraphim_tinyclaw` (1)
- `terraphim_service` (1)
- `terraphim_middleware` (3)
- `terraphim_tracker` (1)
- `terraphim_rolegraph` (1)
- `terraphim_types` (1)

---

## 4. Recommendations

### Immediate (next sprint)
1. Fix all broken doc links in `terraphim_orchestrator` -- 14 warnings concentrated here
2. Add `//!` crate-level docs to the 5 most-used crates:
   - `terraphim_agent`
   - `terraphim_service`
   - `terraphim_config`
   - `terraphim_persistence`
   - `terraphim_rolegraph`

### Short-term
3. Fix HTML tag issues (escape `<` as `\<` or use backticks)
4. Run `cargo fix --lib` on `terraphim_tinyclaw`, `terraphim_middleware`, `terraphim_tracker`, `terraphim_types` for auto-fixable warnings

### Process
5. Enable `#![warn(missing_docs)]` in CI for new crates
6. Add `cargo doc --no-deps` check to pre-commit hooks

---

## Appendix: Commands Used

```bash
# Generate warnings
cargo doc --workspace --no-deps 2>&1 | grep "warning:"

# Check for missing crate docs
for crate_dir in crates/*/; do
  first_line=$(head -1 "${crate_dir}src/lib.rs")
  [[ ! "$first_line" =~ ^//! ]] && echo "$(basename $crate_dir): missing"
done

# Check CHANGELOG
git log --oneline --since="30 days ago"
```
