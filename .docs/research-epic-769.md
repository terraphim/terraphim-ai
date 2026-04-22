# Research Document: Technical Product Owner Agent (Epic #769)

## 1. Problem Restatement and Scope

### Problem
The `product-development` agent name is overloaded across the ADF fleet. It currently mixes:
- Roadmap and feature prioritization (planning)
- Frontend implementation (Lux persona)
- General code review

This semantic drift creates ambiguity in dispatch routing, agent contracts, and human expectations. There is no dedicated agent for repository stewardship -- synthesizing stability signals (errors, failures, regressions) and usefulness signals (docs friction, recurring issues, onboarding pain) into evidence-backed backlog actions.

### In Scope
- Normalize `product-development` to mean roadmap/feature prioritization only
- Add new `repo-steward` agent for repository stewardship
- Update dispatch routing (meta-coordinator) to distinguish both roles
- Update tests, docs, and architecture diagrams
- Pilot on one project

### Out of Scope
- New orchestrator Rust primitives or core logic changes
- New persistence schema or database tables
- Real product analytics ingestion
- Automatic merging/releasing by the stewardship agent
- Retiring `product-development` name entirely
- Creating a new dedicated product persona in v1

## 2. User & Business Outcomes

### Visible Changes
1. **Clear role separation**: Humans and agents can invoke `@adf:product-development` for roadmap questions and `@adf:repo-steward` for repo health questions without ambiguity
2. **Evidence-backed stewardship issues**: Repo-steward creates Gitea issues with structured evidence, not generic summaries
3. **Reduced duplication**: KG lookup and Theme-ID deduplication prevent spam
4. **Normalized dispatch**: Meta-coordinator routes planning issues to product-development and health issues to repo-steward

### Business Value
- Prevents metric gaming by requiring two-panel evidence (stability + usefulness)
- Reduces manual triage of repo health signals
- Maintains existing ADF infrastructure without core changes

## 3. System Elements and Dependencies

### Existing Components

| Component | Location | Role | Dependencies |
|-----------|----------|------|--------------|
| Agent templates | `scripts/adf-setup/agents/*.toml` | Define agent behavior, schedule, persona, task | None (config) |
| Meta-coordinator | `scripts/adf-setup/agents/meta-coordinator.toml` | Dispatches top ready issue to appropriate role | Gitea API, gtr CLI |
| Mention resolver | `crates/terraphim_orchestrator/src/mention.rs` | Parses and resolves `@adf:name` mentions | AgentDefinition, PersonaRegistry |
| Config parser | `crates/terraphim_orchestrator/src/config.rs` | Defines AgentDefinition struct, TOML parsing | serde |
| Example config | `crates/terraphim_orchestrator/orchestrator.example.toml` | 13-agent fleet example | None (docs) |
| Architecture docs | `.docs/adf-architecture.md` | Diagrams and layer tables | None (docs) |
| Cron cadence docs | `scripts/adf-setup/docs/cron-cadence.md` | Schedule policy | None (docs) |
| Migration tests | `scripts/adf-setup/tests/test_migrate.py` | Black-box tests for migration script | pytest, tomllib |
| Persona registry | `data/personas/` | Persona definitions and metaprompt template | Handlebars |
| Gitea robot CLI | `gtr` | Issue/comment operations | Gitea API |
| Terraphim agent | `terraphim-agent` | KG search, learning capture | Local KG |

### New Components Needed

| Component | Purpose |
|-----------|---------|
| `scripts/adf-setup/agents/product-development.toml` | Canonical planning-only template |
| `scripts/adf-setup/agents/repo-steward.toml` | Repository stewardship template |
| `scripts/adf-setup/tests/test_product_development_template.py` | Validate planning-only contract |
| `scripts/adf-setup/tests/test_repo_steward_template.py` | Validate stewardship contract |

### Modified Components

| Component | Changes |
|-----------|---------|
| `scripts/adf-setup/agents/meta-coordinator.toml` | Extend dispatch prompt with product-development and repo-steward |
| `scripts/adf-setup/docs/cron-cadence.md` | Document cadence for both agents |
| `crates/terraphim_orchestrator/orchestrator.example.toml` | Normalize product-development, add repo-steward |
| `crates/terraphim_orchestrator/src/mention.rs` | Remove Lux/frontend assumptions from product-development tests |
| `.docs/adf-architecture.md` | Update diagrams and layer tables |

## 4. Constraints and Their Implications

### C1: Subscription-Only Models
**Constraint**: Only subscription-based models allowed (no `opencode/`, `github-copilot/` prefixes). Allowed: `sonnet`, `kimi-for-coding/k2p5`, etc.
**Implication**: Both new agents must use allowed models. The design already specifies `sonnet` with `kimi-for-coding/k2p5` fallback.

### C2: No Orchestrator Core Changes
**Constraint**: v1 must not add new Rust primitives or change dispatcher/mention core logic.
**Implication**: Agents are implemented as configuration/prompt-only. The mention regex already supports `repo-steward` and `product-development` as valid agent names (pattern: `[a-z][a-z0-9-]{1,39}`).

### C3: Max One Issue Per Panel Per 24h
**Constraint**: repo-steward must not spam issues.
**Implication**: Template must implement idempotency guard using Gitea API to check for existing issues with same Theme-ID in last 24h.

### C4: Direct Mentions Only
**Constraint**: Automation must use `@adf:repo-steward` and `@adf:product-development`, not persona resolution.
**Implication**: Tests and docs must use explicit agent names, not persona aliases.

### C5: Reuse Existing Persona
**Constraint**: v1 should reuse existing persona (Carthos) rather than create new persona assets.
**Implication**: Both agents use `persona = "Carthos"` in v1. A dedicated product persona can be considered later.

### C6: Existing Test Patterns
**Constraint**: Must follow existing test patterns (black-box subprocess, no mocks, TOML validation).
**Implication**: New tests follow `test_migrate.py` patterns using `subprocess.run` and `tomllib`.

## 5. Risks, Unknowns, and Assumptions

### Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Prompt-only stewardship too weak | Medium | Pilot on one project first; if insufficient, design orchestrator-native aggregation in follow-up |
| Issue spam despite 24h guard | Low | Theme-ID dedup + KG search + max 1 per panel |
| Semantic confusion persists | Medium | Clear docs, explicit mentions, remove Lux coupling from tests |
| Cost overrun from 6h schedule | Low | Use subscription models, sparse schedule, no full test reruns |
| Duplicate with existing drift-detector | Low | Drift-detector monitors config/test drift; repo-steward synthesizes broader repo health |

### Unknowns

1. **Quickwit availability**: Is Quickwit endpoint reliably reachable from agent tasks? If not, repo-steward must degrade gracefully.
2. **Gitea API rate limits**: Will frequent `gtr` calls in repo-steward task hit rate limits?
3. **Theme-ID stability**: How to generate stable Theme-ID slugs that survive rewording?

### Assumptions

- **ASSUMPTION**: Existing `terraphim-agent search` and `learn query` commands are sufficient for KG lookup.
- **ASSUMPTION**: The `rg` (ripgrep) tool is available in the agent execution environment.
- **ASSUMPTION**: Gitea issues API supports filtering by label and title substring for duplicate detection.
- **ASSUMPTION**: Carthos persona is acceptable for both planning and stewardship in v1.

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity
1. **Multi-layer agent fleet**: Safety/Core/Growth layers with different priorities and schedules
2. **Persona resolution ambiguity**: Existing code couples `product-development` to Lux persona in tests
3. **Multiple dispatch paths**: Time-driven, mention-driven, issue-driven, ReviewPr each have different priority scores
4. **Cross-cutting docs**: Changes span templates, tests, Rust code, example config, and architecture docs

### Simplification Strategies
1. **Configuration-first approach**: No Rust changes beyond test normalization (removing Lux assumptions)
2. **Additive only**: Don't retire `product-development`, just narrow its meaning
3. **Template reuse**: Follow exact patterns from `meta-coordinator.toml` and `pr-reviewer.toml`
4. **Clear boundaries**: product-development plans, repo-steward synthesizes, implementation-swarm builds

## 7. Questions for Human Reviewer

1. **Q**: Should `product-development` keep its existing `schedule = "0 2 * * *"` (daily at 02:00) or change to a different cadence for planning work?
   **Why**: The design doc suggests planning work may need different frequency than the current daily schedule.

2. **Q**: Should repo-steward use `layer = "Core"` or `layer = "Growth"`?
   **Why**: Core layer has priority 1000, Growth has 2000. Repository stewardship could be considered either essential (Core) or enhancement (Growth).

3. **Q**: Is the `15 */6 * * *` cadence (every 6 hours at :15) acceptable for repo-steward, or should it be sparser?
   **Why**: Cost and noise trade-off. The design suggests this cadence but it's configurable.

4. **Q**: Should we create a dedicated product persona (e.g., "Meridian" or new) in v1, or strictly reuse Carthos?
   **Why**: The design says reuse Carthos pragmatically, but a dedicated persona might reduce confusion.

5. **Q**: For Theme-ID generation, should we use a simple slug of the theme title, or include a hash of evidence sources?
   **Why**: Affects deduplication reliability. Simple slug is easier; hash is more robust.

6. **Q**: Should repo-steward issues be created in the same repo being monitored, or in a central `adf-fleet` repo?
   **Why**: Same repo is more visible; central repo avoids cluttering project backlogs.

7. **Q**: The existing `product-development` in `orchestrator.example.toml` uses `persona = "ferrox"` and `terraphim_role = "Ferrox Rust"` with capabilities `["product-development", "feature-implementation", "api-design"]`. Should we change the persona to Carthos for consistency with the normalized planning role?
   **Why**: Ferrox is the implementation persona; using it for planning creates semantic confusion.

8. **Q**: Should the meta-coordinator dispatch prompt use keyword-based routing (scanning issue title/body for keywords) or LLM-based classification for choosing between product-development and repo-steward?
   **Why**: Keywords are faster and cheaper; LLM is more accurate but slower.

9. **Q**: For the mention.rs test updates, should we replace the Lux-coupled `product-development` test agent with a Carthos-coupled one, or remove the test agent entirely and add new tests for the normalized role?
   **Why**: Affects test coverage and backward compatibility.

10. **Q**: Should we include a pre_check for repo-steward that verifies Quickwit endpoint reachability, or keep it simple with no pre_check?
    **Why**: Pre-check adds robustness but complexity. The design says fail-open.
