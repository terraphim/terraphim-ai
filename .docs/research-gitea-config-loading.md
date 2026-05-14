# Research Document: GitOps-Style Agent Configuration Loading from Gitea

## 1. Problem Restatement and Scope

**Problem:** The ADF (AI Dark Factory) orchestrator loads agent skill definitions and agent configurations exclusively from local TOML files (`/opt/ai-dark-factory/conf.d/*.toml`) and local skill directories (`/opt/ai-dark-factory/skills/*/`). This creates operational friction: configuration changes require SSH access to bigbox, direct file edits, and orchestrator restarts. Skills live in multiple local directories (`~/.opencode/skills/`, `~/.claude/skills/`, `/opt/ai-dark-factory/skills/`) with no versioning, audit trail, or collaborative workflow.

**Core Question:** How can the orchestrator load skills and agent definitions from Gitea repositories instead of local files, while maintaining security, performance, and reliability?

**IN Scope:**
- Loading SKILL.md files from Gitea repositories
- Loading agent definition TOML fragments from Gitea repositories
- Caching strategies for offline resilience
- Security model for remote config
- Integration with existing orchestrator startup flow

**OUT of Scope:**
- Real-time config hot-reload without restart (Phase 3)
- Skill versioning with semantic version constraints
- Automated skill improvement/optimiser loops (P3 autonomy)
- ML-based skill selection/classification

## 2. User & Business Outcomes

### For Operators (Human Outcomes)
- **Version-controlled config:** Every skill/agent change is a Git commit with author, timestamp, and diff
- **Rollback capability:** Revert to previous config by checking out older commits
- **Collaborative editing:** Multiple operators can propose config changes via PRs with reviews
- **No SSH required:** Update agent behaviour by pushing to Gitea, not by editing files on bigbox

### For Agents (System Outcomes)
- **Consistent behaviour across fleets:** Development, staging, and production use identical skill definitions from the same Git ref
- **Faster skill iteration:** Test new skills on feature branches without deploying to bigbox
- **Audit trail:** Every agent spawn logs which Git SHA of skills it used

### For the Organisation (Business Outcomes)
- **Reduced operational toil:** Eliminates manual file edits on production servers
- **Improved compliance:** All configuration changes are tracked in Git (evidence graph, FPF A.10)
- **Fleet scaling:** New ADF instances can bootstrap by cloning a single Gitea repo instead of hand-copying TOML files

## 3. System Elements and Dependencies

| Element | Location | Role | Dependencies |
|---------|----------|------|--------------|
| **OrchestratorConfig** | `crates/terraphim_orchestrator/src/config.rs:72` | Top-level config struct parsed from TOML | `toml` crate, `serde` |
| **OrchestratorConfig::from_file()** | `config.rs:1184` | Loads TOML + expands `include` globs | `glob` crate, filesystem |
| **load_skill_chain_content()** | `lib.rs:1479` | Resolves skill names to SKILL.md content | Filesystem walk of `skill_data_dir` + HOME dirs |
| **Gitea API Client** | `output_poster.rs` | Existing HTTP client for Gitea issue/PR operations | `reqwest` (transitive dep) |
| **skill_data_dir** | `orchestrator.toml` field | Local directory for skills (e.g. `/opt/ai-dark-factory/skills/`) | Filesystem |
| **conf.d/*.toml** | `/opt/ai-dark-factory/conf.d/` | Per-project agent definitions | Filesystem, `IncludeFragment` parser |
| **GITEA_TOKEN** | Environment variable | Auth token for Gitea API | Already used by gitea-robot |
| **Gitea Server** | `https://git.terraphim.cloud` | Git forge hosting config repos | HTTPS, API v1 |
| **Deny-First Gate** | ADR-006 | Security gate constructed before skill/agent load | Must be initialised from local config |

### Data Flow (Current)
```
1. systemd starts adf-orchestrator.service
2. OrchestratorConfig::load_and_validate("/opt/ai-dark-factory/orchestrator.toml")
3.   → from_file() reads TOML + include globs for conf.d/*.toml
4.   → validate() checks project refs, banned providers, etc.
5. At agent spawn: load_skill_chain_content() walks skill_data_dir + HOME dirs
6.   → Finds SKILL.md files, injects content into agent prompt
```

### Data Flow (Proposed)
```
1. systemd starts adf-orchestrator.service
2. OrchestratorConfig::load_and_validate("/opt/ai-dark-factory/orchestrator.toml")
3.   → from_file() reads TOML + include globs (local files)
4.   → If gitea_skill_repo configured: fetch skills from Gitea → cache locally
5.   → If agent_definitions_repo configured: fetch agent TOMLs from Gitea → cache locally
6.   → validate() checks merged config (local + remote)
7. At agent spawn: load_skill_chain_content() checks cache first, then local dirs
```

## 4. Constraints and Their Implications

### C1: Security (ADR-006 Deny-First Gate)
- **Constraint:** The deny-first permission gate must be constructed BEFORE any skill or agent config is loaded.
- **Implication:** Gate rules must remain in local, immutable config. Remote skill/agent loading cannot influence gate construction. This means `orchestrator.toml` (or a local gate-rules file) must always be local.
- **Shape of solution:** Hybrid model -- local gate config + remote skills/agents.

### C2: Offline Resilience
- **Constraint:** ADF must continue operating if Gitea is unreachable.
- **Implication:** Must implement local disk caching with fallback. Cache must persist across restarts. Cache invalidation must be explicit, not automatic.
- **Shape of solution:** Cache directory (`/opt/ai-dark-factory/.cache/skills/`) + stale-cache fallback.

### C3: Startup Performance
- **Constraint:** Orchestrator restart must not take more than a few seconds.
- **Implication:** Cannot make blocking HTTP calls to Gitea at startup without timeout and fallback.
- **Shape of solution:** Async pre-fetch or lazy loading with cache. systemd `ExecStartPre` for cache warming.

### C4: Existing Provider Constraints
- **Constraint:** Only subscription-based providers allowed (C1 from config.rs). No pay-per-use.
- **Implication:** Remote config must not introduce new provider strings that bypass validation.
- **Shape of solution:** `validate_model_provider()` runs after remote config merge; banned providers still rejected.

### C5: Skill Chain Loading Order
- **Constraint:** `load_skill_chain_content()` tries multiple roots in order: `skill_data_dir`, `~/.opencode/skills`, `~/.claude/skills`.
- **Implication:** Gitea skills should be inserted into this hierarchy without breaking existing resolution.
- **Shape of solution:** Add Gitea cache dir as first root (override), keep local roots as fallbacks.

### C6: Include Fragment Parsing
- **Constraint:** `IncludeFragment` only accepts `projects`, `agents`, `flows`, `pr_dispatch` fields.
- **Implication:** Remote agent definitions must conform to this schema.
- **Shape of solution:** Remote agent TOMLs are parsed as `IncludeFragment`, same as local conf.d files.

## 5. Risks, Unknowns, and Assumptions

### Risks

| Risk | Likelihood | Impact | De-risking |
|------|------------|--------|------------|
| Gitea API rate limiting or downtime | Medium | High | Local cache with 24h TTL; fallback to cached skills |
| Skill cache becomes stale | Medium | Medium | Log cache age at startup; allow `--force-refresh` flag |
| Token compromise exposes all config | Low | High | Read-only token; short expiry; audit logging |
| Malicious skill injection via compromised Gitea account | Low | High | Branch protection + PR reviews; deny-first gate limits blast radius |
| Increased complexity in config loading | Medium | Medium | Extensive tests; clear error messages; fallback to local |
| Circular dependency: config references itself | Low | Medium | Validate before load; max depth limit |

### Unknowns
- **U1:** What is the latency of Gitea API calls from bigbox? (Need to benchmark `/api/v1/repos/terraphim/terraphim-ai/raw/README.md`)
- **U2:** How large is the full skill set? (Need to measure `/opt/ai-dark-factory/skills/` total size)
- **U3:** Does Gitea support conditional requests (ETag, If-None-Match) for cache validation?
- **U4:** What is the Gitea server load capacity for adding config fetches to existing traffic?
- **U5:** How will firecracker VM instances (Terraphim Forge) authenticate to Gitea without local tokens?

### Assumptions
- **A1:** Gitea API is stable and accessible from bigbox via HTTPS.
- **A2:** Existing `GITEA_TOKEN` has sufficient permissions (read access to config repos).
- **A3:** Skills are static text files (SKILL.md) that rarely change (daily/weekly, not per-second).
- **A4:** Operators prefer Git workflow over SSH file editing for config changes.

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity
1. **Multiple skill roots:** Local + remote + HOME directories create a complex resolution hierarchy.
2. **Two-phase config loading:** Skills loaded at spawn time, agent definitions at startup -- different timing, different caching needs.
3. **Security-critical path:** Remote config is in the trust boundary; must not bypass gate construction.
4. **Multi-fleet consistency:** Development laptops, bigbox, Firecracker VMs all need to resolve config correctly.

### Simplification Strategies

**S1: Clear separation of concerns**
- Local config: Gate rules, working_dir, nightwatch, compound_review, Gitea credentials
- Remote config: Skills, agent definitions, flows
- This mirrors Kubernetes' separation of cluster config (local) and workload config (GitOps).

**S2: Immutable config references**
- Pin all remote config to commit SHAs, not branch names.
- This eliminates "what version is running?" ambiguity and makes debugging deterministic.
- Update mechanism: edit SHA in local config + restart (simple, explicit).

**S3: Cache as single source of truth**
- Orchestrator never reads directly from Gitea at spawn time; always reads from cache.
- Cache is populated at startup (or by `ExecStartPre`).
- This makes skill loading identical to current local loading from the orchestrator's perspective.

## 7. Questions for Human Reviewer

1. **Should we start with skills only (Phase 1) or do skills + agent definitions together?**
   - Why: Skills are lower risk (read-only, no structural config changes); agent definitions affect orchestrator startup.

2. **What Gitea repo structure do you prefer for the skills registry?**
   - Option A: Single repo `terraphim/skills-registry` with subdirectories
   - Option B: One repo per skill (`terraphim/skill-disciplined-research`, etc.)
   - Why: This affects cache strategy and discovery logic.

3. **Should agent definitions live in the same repo as skills or a separate `terraphim/adf-config` repo?**
   - Why: Separate repos allow different access controls; single repo is simpler.

4. **What is the maximum acceptable startup delay if Gitea is slow or unreachable?**
   - Why: Determines timeout values and fallback strategy.

5. **Should we implement webhook-based cache invalidation (push to repo triggers refresh) or periodic polling?**
   - Why: Webhooks are faster but more complex; polling is simpler but has lag.

6. **How should we handle secrets in remote agent definitions (e.g., webhook secrets in TOML)?**
   - Why: Current local TOMLs may contain secrets; moving to Gitea requires env var substitution or encryption.

7. **Do we need a dry-run / validation command (e.g., `adf-ctl check --remote-config`) before enabling this in production?**
   - Why: Validates remote config without affecting running orchestrator.

8. **Should we support multiple skill registries (e.g., terraphim/skills + zestic-ai/odilo-skills)?**
   - Why: Multi-tenant fleets may need project-specific skill libraries.

9. **What is the retention policy for the local skill cache?**
   - Why: Cache grows unbounded over time as skills are updated.

10. **Should this feature be gated behind a feature flag or enabled by default once implemented?**
    - Why: Allows gradual rollout; operators can opt-in per fleet instance.

---

## References

- `terraphim-ai/crates/terraphim_orchestrator/src/config.rs` -- Config loading and validation
- `terraphim-ai/crates/terraphim_orchestrator/src/lib.rs` -- Skill loading hierarchy
- `terraphim-ai/crates/terraphim_orchestrator/orchestrator.example.toml` -- Example fleet config
- `cto-executive-system/AGENTS.md` -- P1-P3 self-improving framework
- `cto-executive-system/FPF-Spec.md` -- A.4 (Temporal Duality), A.10 (Evidence Graph)
- `cto-executive-system/decisions/ADR-006-adf-deny-first-permission-gate.md` -- Security constraints
- Gitea API documentation: `/api/v1/repos/{owner}/{repo}/raw/{filepath}?ref={branch}`
