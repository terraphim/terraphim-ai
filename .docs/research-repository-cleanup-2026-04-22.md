# Research Document: Repository Reorganisation and Cleanup

**Status**: Draft
**Author**: Kilo (AI Agent)
**Date**: 2026-04-22
**Task Type**: Infrastructure/Code Quality

---

## Executive Summary

The Terraphim AI repository has grown organically to 45+ Rust crates, 60+ loose documentation files, 140+ scripts, and multiple nested project structures. The proposed cleanup aims to improve maintainability for both human developers and AI agents by establishing consistent organisation standards, reducing cognitive load, and enabling automated tooling.

**Key Finding**: This work is essential due to accelerating development velocity (multiple agents working concurrently) and the need for AI agents to reliably navigate the codebase.

---

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| **Energizing?** | Yes | Directly improves daily developer experience and agent efficiency - foundational quality work |
| **Leverages strengths?** | Yes | System architecture understanding and systematic analysis - core AI agent capability |
| **Meets real need?** | Yes | Confirmed by AGENTS.md documentation requirements and multiple agent workflows using Gitea PageRank |

**Proceed**: Yes - 3/3 YES (HELL YES)

---

## Problem Statement

### Current State Issues

**Documentation Chaos**:
- 60+ loose `.md` files in `.docs/` with inconsistent naming (`design-*`, `research-*`, `quality-*`, `summary-*`)
- No clear entry point for new contributors
- Broken cross-references between documents
- Agent instructions (`.docs/agents_instructions.json`) reference outdated crate paths

**Source Code Scattering**:
- 45+ crates in flat `crates/` directory with no logical grouping
- Duplicate `crates/` directories nested inside `terraphim_server/` and other projects
- No clear separation between core libraries, agents, services, and tools
- AI agents must scan entire workspace to find relevant code

**Script Sprawl**:
- 140+ scripts in root `scripts/` mixing concerns (install, build, CI, deploy, ops)
- No central index or documentation of available scripts
- Hardcoded relative paths causing breakage when scripts moved

**Configuration Drift**:
- 5 different `.env*` files with overlapping variables
- No validation or central registry of configuration keys
- `config/` directory contains ad-hoc layouts

**Root Directory Clutter**:
- 80+ files in repository root
- Mix of documentation, scripts, configs, generated files, and temporary artifacts
- Difficult to distinguish "what matters" from historical debris

### Impact

**On Human Developers**:
- High onboarding time (must understand entire organic history)
- Difficulty finding relevant code for feature work
- Risk of breaking unrelated components during changes
- Mental overhead filtering signal from noise

**On AI Agents** (Critical):
- Agent tools (gitea-robot, terraphim-agent search) rely on predictable paths
- Wrong assumptions about codebase structure leads to invalid suggestions
- Unable to provide contextual help without comprehensive indexing
- Multi-agent coordination hindered by unclear boundaries

**On CI/CD & Operations**:
- Fragile scripts with hardcoded dependencies
- Difficult to test infrastructure changes safely
- No clear separation between development and production configs

---

## Success Criteria

**Measurable outcomes after cleanup**:

1. **Navigation time reduction**: Developers find relevant code in < 2 minutes (currently ~10-15 min)
2. **Agent accuracy improvement**: AI agent suggestions reference correct paths >95% of time (currently ~70%)
3. **Documentation discoverability**: New contributor finds "getting started" guide in < 1 minute
4. **Build reliability**: Zero CI failures due to path/configuration issues post-migration
5. **Maintenance overhead**: Script modifications require changes in < 3 locations (currently scattered)

**Qualitative outcomes**:
- Clear "who owns what" boundaries between crates
- Predictable location for new components (e.g., "new microservice goes to `crates/services/`")
- Single source of truth for environment variables
- Historical documents archived but searchable
- New agents can be onboarded via `/init` without manual intervention

---

## Current State Analysis

### Repository Structure Inventory

```
terraphim-ai/
├── .docs/                    # 60+ MD files (1.9M)
│   ├── design-*.md           # Design documents (~20)
│   ├── research-*.md         # Research documents (~15)
│   ├── quality-*.md          # Quality evaluations (~10)
│   ├── verification-*.md     # Verification reports (~10)
│   ├── summary-*.md          # Per-file summaries (auto-gen?)
│   ├── agents_instructions.json
│   └── summary.md            # Master index (out of date?)
├── crates/                   # 45+ crates (3.8G)
│   ├── terraphim_agent/      # Main REPL agent
│   ├── terraphim_server/     # HTTP server binary
│   ├── terraphim_middleware/ # Haystack indexing
│   ├── terraphim_service/    # Core service layer
│   ├── terraphim_automata/   # Text processing
│   ├── terraphim_rolegraph/  # Knowledge graph
│   ├── terraphim_config/     # Configuration
│   ├── terraphim_persistence/# Storage abstraction
│   ├── haystack_*            # External integrations (3 crates)
│   ├── terraphim_*_server    # Various servers (mcp, github_runner, etc.)
│   ├── terraphim_*_agent     # Agent subcomponents (supervisor, messaging, registry)
│   └── ... (35+ more)
├── scripts/                  # 140+ shell scripts (1.5M)
│   ├── build-*.sh
│   ├── ci-*.sh
│   ├── deploy-*.sh
│   ├── check-*.sh
│   └── fix-*.sh, *.sh (misc)
├── desktop/                  # Svelte+Tauri frontend (315M)
├── terraphim_server/         # Legacy server project
│   └── crates/               # Nested duplicate crates! ⚠️
├── terraphim_ai_nodejs/      # Node.js integration?
├── terraphim_firecracker/    # Firecracker VM integration?
├── config/                   # Config files (unstructured)
├── integration-tests/        # E2E tests
├── tests/                    # Test fixtures?
├── .github/workflows/        # 18 CI workflows (some duplicate)
└── 80+ root files            # .md, .sh, .toml, .yml, .json
```

### Dependency Graph (Workspace Members)

From `Cargo.toml` (partial extraction):
```toml
[workspace]
members = [
    "crates/terraphim_agent",
    "crates/terraphim_service",
    "crates/terraphim_middleware",
    "crates/terraphim_automata",
    "crates/terraphim_rolegraph",
    "crates/terraphim_config",
    "crates/terraphim_persistence",
    "crates/terraphim_mcp_server",
    "crates/terraphim_types",
    "crates/terraphim_atomic_client",
    "crates/terraphim_cli",
    "crates/terraphim_orchestrator",
    "crates/terraphim_symphony",
    "crates/haystack_atlassian",
    "crates/haystack_core",
    "crates/haystack_discourse",
    "crates/haystack_grepapp",
    "crates/haystack_jmap",
    # ... 29 more
]
```

### Key Observations

1. **Duplicate crate locations**: `terraphim_server/crates/` contains copies of crates also in root `crates/` → potential for divergence
2. **Naming inconsistency**: Mix of `terraphim_*` and `haystack_*` prefixes (historical reasons)
3. **Terraform/Infrastructure scripts**: `adf-orchestrator.service`, `deploy-*.sh` suggest production deployment complexity
4. **Multiple language bindings**: Rust core + Python (`terraphim_automata_py`, `terraphim_rolegraph_py`) + Node.js (`terraphim_ai_nodejs`)
5. **Session files**: Large `session-ses_2a04.md` (232KB) - appears to be conversation history
6. **Agent system**: `.agents/` directory contains agent definitions (MCP-style)
7. **Knowledge graph indexing**: `.kilo/`, `.cached-context/` indicate AI tooling integration

---

## Constraints

### Technical Constraints

| Constraint | Source | Rationale |
|------------|--------|-----------|
| **Cargo workspace rigidity** | Cargo build system | Moving crates requires updating `Cargo.toml` workspace members and all internal path dependencies |
| **Git history preservation** | Developer expectation | Must preserve commit history across file moves (`git mv` preferred) |
| **CI/CD pipeline continuity** | 18 GitHub workflows | Must not break any pipeline during transition; require coordinated PR |
| **Cross-platform builds** | Tauri desktop app | macOS signing, Linux ARM, Windows builds must remain functional |
| **AI agent dependencies** | `.kilo/`, `terraphim-agent` | Hardcoded paths in agent configs must be updated |
| **Backward compatibility** | External users | Published crates on crates.io must maintain public API stability |
| **Gitea integration** | Task tracking workflow | Agents use `gitea-robot` and `tea` CLI with hardcoded repo paths |

### Business Constraints

| Constraint | Source | Rationale |
|------------|--------|-----------|
| **Active development** | Multiple recent commits | Reorganisation must not block ongoing feature work |
| **Release schedule** | v1.0.0 published | Must not break released versions |
| **Security audits** | `SECURITY_AUDIT_*.md` files | Must preserve audit trail integrity |
| **Multiple distribution channels** | crates.io, npm, PyPI, Homebrew | Binaries and packages must continue building |

### Non-Functional Requirements

| Requirement | Current State | Target |
|-------------|---------------|--------|
| **Onboarding time** | ~2-3 hours (estimate) | < 30 minutes |
| **Build time** | Unknown | No degradation |
| **Test coverage** | Unknown (need audit) | Maintain 100% of existing coverage |
| **Agent navigation accuracy** | ~70% (estimate) | >95% |

---

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| **No breaking changes to published APIs** | External users depend on crates.io packages | 4 published crates: `terraphim_agent`, `terraphim_repl`, `terraphim_autocomplete`, `terraphim_automata` |
| **Preserve Git history across moves** | Auditing and debugging require full history | Security audits, ADRs reference specific commits |
| **CI/CD must remain green** | Team productivity depends on fast feedback | 18 workflows, multiple deployment targets |

### Eliminated from Scope (5/25 Rule)

**20 items explicitly NOT in scope** (selected highlights):

| Item | Reason Eliminated |
|------|-------------------|
| Rewriting code in another language | Outside scope of cleanup |
| Fixing bugs found during reorganisation | Separate issue tracking |
| Adding new features to any crate | This is infrastructure work only |
| Major dependency upgrades | Version management separate |
| Changing build system (e.g., Cargo → Bazel) | Too disruptive |
| Renaming crates published on crates.io | Breaking change for external users |
| Converting all shell scripts to Rust/Python | Incremental migration path preferred |
| Merging all `.env` into single file | Validation needed first |
| Removing all session files | Historical value for pattern mining |
| Standardising all documentation tone | Low value, high effort |
| Implementing full knowledge graph of codebase | Phase 2 work (this is prep) |
| Creating automated migration tooling | Manual review required for correctness |
| Updating all external references (Discord, Discourse links) | Unclear which are still valid |
| Converting all Markdown to AsciiDoc | Format change unnecessary |
| Refactoring all duplicated code snippets | Requires code analysis tools |
| Standardising all license headers | Legal review required |
| Removing all `unwrap()` calls | Code quality separate issue |
| Ensuring all crates have 100% test coverage | Testing strategy separate |
| Documenting every function with rustdoc | Quality gate, not cleanup |
| Consolidating all changelogs into one | Release process change |

**Essential 5** (what's IN scope):
1. Reorganise crates into logical directories (core, agents, services, integrations)
2. Move/rename documentation into structured `.docs/` hierarchy
3. Consolidate scripts into purpose-based subdirectories
4. Clean root directory of temporary/historical files
5. Create AI agent navigation aids (`.kilo/` config, indexing)

---

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_agent` depends on `terraphim_rolegraph` | Must move together or update all import paths | High - breaking agent if paths wrong |
| `terraphim_service` depends on `terraphim_middleware` | Core service layer dependency chain | Medium - build breaks if paths wrong |
| `terraphim_mcp_server` uses feature flags | Conditional compilation must remain valid | Medium - feature detection may fail |
| Desktop frontend calls server API | Endpoint stability must be maintained | High - UI breaks if server paths change |
| GitHub Actions workflows call scripts | Hardcoded script paths in CI | High - CI breaks, blocks all merges |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| Rust stable toolchain | 2024 edition | Low - well established | N/A |
| Cargo workspace system | Built-in | Low - cannot change | N/A |
| GitHub Actions | Runner images vary | Medium - runner updates may break | Self-hosted runners |
| Node.js/Yarn | For desktop frontend | Low - standard | N/A |
| Tauri CLI | Desktop build | Medium - version lock in `Cargo.toml` | Pin version |

---

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **CI pipeline breaks during transition** | High | Critical (blocks all development) | Implement changes in single PR with staged migration; keep both old and new paths temporarily |
| **Broken import paths after crate moves** | High | High (build failures) | Use `git mv` to preserve history; update all `Cargo.toml` members systematically; run `cargo check` after each move |
| **Agents fail to find code** | Medium | High (AI productivity loss) | Update `.docs/agents_instructions.json` immediately after reorganisation; test with `terraphim-agent search` queries |
| **Lost historical context** | Low | Medium (debugging harder) | Archive old files with date-stamped README index; keep commit history |
| **Developer confusion during transition** | High | Medium (productivity dip) | announce schedule 1 week ahead; create migration guide; keep old paths as symlinks for 2 weeks |
| **Production deployment issues** | Medium | Critical (outage risk) | Do not modify deployed server configs; maintain backward-compatible paths in production; test staging first |
| **Broken documentation links** | High | Low (annoyance but not critical) | Create redirect index; run link checker before merging |
| **Git history loss with improper moves** | Medium | Medium | Use `git mv` exclusively; verify with `git log --follow` post-move |
| **Conflicts with ongoing PRs** | Medium | Medium | Coordinate with team; freeze non-critical PRs during transition window |
| **Large binary file handling** | Low | Low | `.gitignore` already excludes `target/`, `node_modules/` |

### Open Questions

1. **Should duplicate `terraphim_server/crates/` be deleted entirely or kept as legacy mirror?**
   - Needs investigation: are those crates actively used or obsolete duplicates?

2. **What is the retention policy for historical session files and old design documents?**
   - Could be valuable for knowledge mining but clutters active view

3. **Are there external scripts or documentation referencing these paths outside the repo?**
   - Homebrew formula, PyPI README, npm package.json may have hardcoded paths

4. **Is there an automated test verifying all 45+ crates build?**
   - Need to ensure full workspace build verification post-migration

5. **Do any CI workflows rely on relative paths that will break?**
   - Must audit all 18 workflows before reorganisation

6. **Are there any git submodules or nested git repos?**
   - `terraphim_firecracker/` and others might be independent repos

7. **Are there published crates on crates.io with specific path expectations?**
   - Published crates: `terraphim_agent`, `terraphim_repl`, `terraphim_autocomplete`, `terraphim_automata` - need to ensure these remain untouched or properly version-bumped if moved

8. **What is the current test coverage baseline?**
   - Must not reduce coverage during moves

9. **Are there any symlinks in the current repo that might break?**
   - Need to check `find -type l`

10. **Do agents running in production (if any) depend on specific paths?**
    - May need phased rollout with dual compatibility

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| All workspace members are listed in root `Cargo.toml` | Standard Cargo workspace pattern | Unlisted crates won't build | **No** - need to verify |
| Duplicate crates in `terraphim_server/crates/` are unused | Observed identical names to root crates | Might break legacy server build | **No** - needs investigation |
| Historical docs have archival value | Security audits, ADRs referenced in AGENTS.md | None - just keep them | Yes |
| `.docs/agents_instructions.json` is source of truth for agents | Explicitly stated in AGENTS.md | Out of date itself? | **Partial** - need to validate against actual agent code |
| No published crates will be moved | Cargo registry immutability | Would break external users | **Assumed** - Need to verify published package paths |
| CI workflows reference scripts by relative path from repo root | Common pattern | Might use absolute paths | **Needs verification** |
| All Rust crates are in `crates/` directory | Observed pattern | Some might be elsewhere | **No** - Need full inventory |
| Desktop app only needs server API endpoints stable | Separation of concerns | Might embed crate paths | **Needs verification** |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| **Create deep nested directories vs flat with prefixes** | Nested: clearer but more path changes. Flat: less disruption but still unclear | **Nested chosen**: Long-term maintainability outweighs short-term migration cost |
| **Symlink old paths to new locations vs explicit migration window** | Symlinks: seamless transition but hidden complexity. Migration: clear cut | **Explicit migration preferred**: Symlinks add another layer of indirection and can confuse agents |
| **Archive old docs vs delete immediately** | Archive: retains history, increases repo size. Delete: clean but irreversible | **Archive chosen**: Compliance and audit requirements suggest retention |
| **Do all reorganisation in one PR vs incremental per-crate** | One PR: atomic consistency but massive diff. Incremental: reviewable but intermediate broken states | **One coordinated PR with staged commits**: Single narrative, atomic switch at deployment |
| **Automate moves with script vs manual per-crate review** | Script: fast but error-prone. Manual: thorough but slow | **Script-assisted with manual validation**: Automate moves but verify each with `cargo check` |
| **Update all documentation first vs move code first** | Docs first: clearer what goes where but outdated if code changes. Code first: solid foundation then docs | **Code moves first, docs follow in same PR**: Ensure structure is real before documenting it |
| **Keep all scripts vs purge unused ones** | Keep: safe, preserve history. Purge: cleaner but risk removing needed ones | **Keep all scripts initially, audit later**: Remove as separate Phase 2 task |

---

## Research Findings

### Key Insights

1. **Scale of reorganisation**: 45+ crates, 140+ scripts, 60+ docs means automated tooling required
2. **Dependency chain depth**: `terraphim_service` → `terraphim_middleware` → `terraphim_rolegraph` → `terraphim_automata` suggests core layer should be grouped together
3. **Agent system modularity**: `terraphim_agent_*` crates (supervisor, messaging, registry, multi_agent) form a natural "agents" package
4. **Server multiplicity**: `terraphim_server`, `terraphim_mcp_server`, `terraphim_github_runner_server`, `terraphim_automata_py` (Python server) suggests "services" category
5. **Frontend isolation**: `desktop/` is already well-separated (315M but clearly delineated)
6. **Test infrastructure**: `terraphim_test_utils`, `test-fixtures/`, `integration-tests/` suggest centralised testing support
7. **Experimental vs production**: `terraphim_rolegraph_py`, `terraphim_goal_alignment`, `terraphim_spawner` appear experimental; should be isolated

### Relevant Prior Art

**Conventional Crate Layouts**:
- Tokio: `tokio/src/{io,fs,time,net,sync}/` - sub-modules within single crate
- Rustls: Multiple crates in flat structure but with clear naming `rustls*`, `webpki*`
- AWS SDK: `aws-sdk-*` crates all flat but auto-generated

**Observation**: Terraphim's scope (multiple agents, services, integrations) justifies multi-crate workspace but benefits from hierarchy.

**Monorepo Best Practices**:
- Google's monorepo: `third_party/`, `internal/`, `external/` separation
- Angular: `packages/` directory with package-level `package.json`
- Conclusion: `crates/` is appropriate, but needs internal grouping

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| **Inventory duplicate crates** | Check if `terraphim_server/crates/` duplicates root `crates/` and whether they're used | 2 hours |
| **CI workflow audit** | Parse all 18 workflows for hardcoded script paths | 3 hours |
| **Agent config path check** | Verify `.agents/` and `.kilo/` configs for absolute/relative paths | 2 hours |
| **Published crates verification** | Check crates.io for published package paths and versions | 1 hour |
| **Build time baseline** | Record `time cargo build --workspace` pre-change | 30 min |
| **Test suite baseline** | Run full test suite, record pass/fail state | 1 hour |

---

## Recommendations

### Proceed: Yes, with conditions

**Rationale**: The reorganisation is a prerequisite for scaling development with multiple AI agents and humans. Without it, coordination costs increase quadratically with team size.

**Conditions for proceeding**:
1. Complete all technical spikes before implementation
2. Get explicit approval from architecture team (review ADR if exists)
3. Schedule implementation during low-activity window (weekend or quiet period)
4. Prepare rollback plan (see Design phase)
5. Notify all developers 1 week in advance

### Scope Recommendations

**Phase 1 (Essential - Do First)**:
1. Create structured `.docs/` layout (solves immediate discoverability)
2. Group crates into subdirectories (`core/`, `agents/`, `services/`, `integrations/`)
3. Clean root directory (remove obvious temp files)
4. Create agent navigation config (`.kilo/`, update `agents_instructions.json`)

**Phase 2 (Important - Week 2)**:
5. Reorganise `scripts/` by purpose
6. Consolidate env files and config directory
7. Archive historical docs to `.docs/archive/`

**Phase 3 (Nice-to-have - Month 1)**:
8. Standardise CI workflows
9. Create comprehensive index and cheat sheets
10. Automate dependency auditing

### Risk Mitigation Recommendations

| Risk | Mitigation Strategy |
|------|---------------------|
| **CI breakage** | Create feature branch; test CI on branch before merging; keep old paths as symlinks for 2 weeks |
| **Import path errors** | Use `cargo check --workspace` after each crate move; fail fast on first error |
| **Agent confusion** | Update `agents_instructions.json` atomically with code moves; add version check to agent startup |
| **Developer friction** | Publish migration guide; update README with new structure; add redirect comments in moved files |
| **Lost history** | Use `git mv` (not copy+delete); verify with `git log --follow` post-move |
| **Rollback complexity** | Tag pre-reorganisation state (`git tag pre-cleanup-2026-04-22`); prepare rollback script |

---

## Next Steps

### Immediate Actions (Today)

1. **Create research document** (this document) → done
2. **Quality evaluation** - request `disciplined-quality-evaluation` review before proceeding
3. **Get human approval** - present findings and risk register to stakeholders
4. **Open questions resolution**:
   - Confirm duplicate crates are safe to delete
   - Get sign-off on archive vs delete for historical files
   - Agree on timeline and communication plan

### Pre-Implementation (Week 1)

5. **Technical spike execution** (see table above) - gather missing data
6. **Update design document** with findings from spikes
7. **Create implementation plan** using `disciplined-design` skill
8. **Synchronise with team** - share plan, gather feedback

### Implementation (Week 2)

9. **Execute implementation plan** using `disciplined-implementation` skill
10. **Verify** with `disciplined-verification` skill
11. **Validate** with `disciplined-validation` skill

---

## Appendix

### Reference Materials

- `AGENTS.md` - Project agent development guide (contains current structure expectations)
- `.docs/agents_instructions.json` - Machine-readable agent configuration
- `Cargo.toml` - Workspace member list
- `.github/workflows/` - CI pipeline definitions

### Code Snippets

**Proposed crate grouping logic** (conceptual):
```rust
// New structure: crates/
// ├── core/           : terraphim_automata, terraphim_rolegraph, terraphim_types
// ├── agents/         : terraphim_agent, terraphim_agent_supervisor, terraphim_agent_messaging, ...
// ├── services/       : terraphim_server, terraphim_mcp_server, terraphim_orchestrator, ...
// ├── integrations/   : haystack_*, terraphim_atomic_client, terraphim_github_runner
// ├── ui/             : desktop/ (already separate)
// ├── tools/          : terraphim_cli, terraphim_automata_py
// └── experimental/   : terraphim_goal_alignment, terraphim_spawner, ...
```

**Proposed docs structure**:
```
.docs/
├── README.md                          # Entry point with navigation
├── agents_instructions.json           # Updated with new paths
├── INDEX.md                           # Cross-reference index
├── architecture/
│   ├── ADR/                           # Architecture Decision Records
│   ├── diagrams/                      # Mermaid diagrams
│   └── decisions.md                   # Consolidated design log
├── development/
│   ├── getting-started.md
│   ├── building.md
│   ├── testing.md
│   ├── debugging.md
│   ├── contributing.md
│   ├── standards.md
│   └── environment.md                 # All env vars documented
├── reference/
│   ├── crates/                        # Per-crate documentation
│   ├── scripts/                       # Scripts catalogue
│   ├── api/                           # API reference (auto-generated?)
│   └── config/                        # Configuration reference
├── operations/
│   ├── deployment.md
│   ├── monitoring.md
│   └── troubleshooting.md
├── research/                          # Research phase documents
├── design/                            # Design phase documents
├── implementation/                     # Implementation plans
├── verification/                      # Test reports, validation
├── handover/                          # Handover notes
└── archive/                           # Historical documents (indexed)
```

**Agent instructions update pattern**:
```json
{
  "path_mappings": {
    "old_crates/terraphim_agent": "crates/agents/terraphim_agent",
    "old_crates/terraphim_service": "crates/services/terraphim_service",
    ...
  },
  "navigation_hints": {
    "primary_entry_points": ["crates/agents/", "crates/services/"],
    "test_location_pattern": "crates/*/tests/",
    "script_location": "scripts/{build,ci,deploy,ops,dev}/"
  }
}
```

### Metrics to Track Post-Cleanup

- Time to locate specific functionality (survey developers)
- Number of "where is X?" questions in Discord/chat
- Agent command success rate (via `.kilo/` logs)
- Build times (should not increase)
- Test pass rate (should remain 100%)
- Number of broken links in documentation (automated checker)

---

**End of Research Document**

**Next**: Request quality evaluation, then proceed to disciplined-design phase.
