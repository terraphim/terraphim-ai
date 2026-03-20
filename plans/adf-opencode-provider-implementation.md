# ADF opencode Provider Implementation Plan

**Date**: 2026-03-20
**Status**: Approved
**Owner**: Alex (CTO)
**Relates to**: `plans/autonomous-org-configuration.md` Section 4.1-4.3

## 1. Objective

Replace all expensive `opencode/` (Zen pay-per-use) and legacy `codex` model routing with subscription-based providers across the ADF agent fleet. Leverage terraphim-ai crates for orchestration, terraphim-skills for agent skill chains, and zestic-engineering-skills (from 6d-prompts) for business-domain workflows.

## 2. Provider Inventory (confirmed 2026-03-20)

All providers connected and verified via `opencode models` on local machine.

| Provider | Provider ID | Pricing | Models Available |
|---|---|---|---|
| opencode Go | `opencode-go/` | $10/mo flat ($60/mo cap) | `kimi-k2.5`, `glm-5`, `minimax-m2.5`, `minimax-m2.7` |
| Kimi for Coding | `kimi-for-coding/` | Subscription (Moonshot) | `k2p5`, `kimi-k2-thinking` |
| z.ai Coding Plan | `zai-coding-plan/` | Subscription (z.ai) | `glm-4.5` - `glm-5-turbo` (11 models) |
| MiniMax Coding Plan | `minimax-coding-plan/` | Subscription (MiniMax) | `MiniMax-M2` - `M2.7-highspeed` (6 models) |
| GitHub Copilot | `github-copilot/` | Included (free OSS quota) | 25 models (Claude, GPT, Gemini, Grok) |
| Anthropic (claude-code CLI) | `claude-code` | Anthropic subscription | `opus-4-6`, `sonnet-4-6`, `haiku-4-5` |
| OpenAI (codex) | `openai/` | OpenAI Team plan | `gpt-5.x-codex` models |
| **opencode Zen** | **`opencode/`** | **Pay-per-use with markup** | **BANNED -- never use** |

## 3. Agent-to-Provider Mapping

### 3.1 Four Model Tiers

| Tier | Provider + Model | Use Case | Cost |
|---|---|---|---|
| **Quick** | `opencode-go/minimax-m2.5` | Routine docs, advisory | $10/mo flat |
| **Deep** | `opencode-go/glm-5` | Quality gates, compound review | $10/mo flat |
| **Implementation** | `kimi-for-coding/k2p5` | Code generation, twins, tests | Kimi sub |
| **Oracle** | `claude-code --model opus-4-6` | Spec validation, deep reasoning | Anthropic sub |

### 3.2 Full Fleet Mapping

| Agent | Layer | Primary | Fallback | Tier |
|---|---|---|---|---|
| security-sentinel | Safety | `opencode-go/kimi-k2.5` | `opencode-go/glm-5` | Deep |
| meta-coordinator | Safety | `claude-code --model opus-4-6` | -- | Oracle |
| compliance-watchdog | Safety | `opencode-go/kimi-k2.5` | `zai-coding-plan/glm-4.7` | Deep |
| drift-detector | Safety | `zai-coding-plan/glm-4.7-flash` | `opencode-go/glm-5` | Quick |
| upstream-synchronizer | Core | `kimi-for-coding/k2p5` | `opencode-go/kimi-k2.5` | Implementation |
| product-development | Core | `claude-code --model sonnet-4-6` | -- | Oracle |
| spec-validator | Core | `claude-code --model opus-4-6` | -- | Oracle |
| test-guardian | Core | `kimi-for-coding/k2p5` | `opencode-go/kimi-k2.5` | Implementation |
| documentation-generator | Core | `opencode-go/minimax-m2.5` | `opencode-go/minimax-m2.7` | Quick |
| twin-drift-detector | Core | `opencode-go/kimi-k2.5` | `zai-coding-plan/glm-4.7` | Deep |
| implementation-swarm (x5-15) | Growth | `kimi-for-coding/k2p5` | `opencode-go/kimi-k2.5` | Implementation |
| compound-review (Quick x12) | Growth | `opencode-go/minimax-m2.5` | `zai-coding-plan/glm-4.7-flash` | Quick |
| compound-review (Deep x6) | Growth | `opencode-go/glm-5` | `kimi-for-coding/k2p5` | Deep |
| browser-qa | Growth | `claude-code --model sonnet-4-6` | -- | Oracle |
| brownfield-analyser | Growth | `claude-code --model opus-4-6` | -- | Oracle |
| twin-implementer | Growth | `kimi-for-coding/k2p5` | `opencode-go/kimi-k2.5` | Implementation |
| twin-verifier | Growth | `kimi-for-coding/k2p5` | `opencode-go/kimi-k2.5` | Implementation |
| twin-scenario-runner | Growth | `claude-code --model sonnet-4-6` | -- | Oracle |

## 4. Implementation Phases

### Phase 1: Model Routing in terraphim_orchestrator (Issues #28-#30)

**Crates**: `terraphim_orchestrator`, `terraphim_spawner`, `terraphim_config`
**Skills**: terraphim-engineering-skills (architecture, implementation, testing)

1. **Extend `orchestrator.toml` schema** to support `provider`, `model`, `fallback_provider`, `fallback_model` fields per agent (currently only `cli_tool` and `model` as flat strings)
2. **Add `ProviderTier` enum** to `terraphim_config`: `Quick`, `Deep`, `Implementation`, `Oracle` -- maps to provider/model pairs
3. **Implement fallback dispatch** in `terraphim_spawner`: if primary model returns error/timeout, retry with fallback. Timeout thresholds: Quick=30s, Deep=60s, Implementation=120s, Oracle=300s
4. **Add provider health tracking**: simple circuit breaker per provider (3 consecutive failures = open circuit for 5 minutes, then half-open probe)

### Phase 2: Subscription Guard (Issue #31)

**Crates**: `terraphim_goal_alignment`, `terraphim_config`
**Skills**: terraphim-engineering-skills (security-audit, testing)

1. **Provider allowlist** in config: `allowed_providers = ["opencode-go", "kimi-for-coding", "zai-coding-plan", "github-copilot", "claude-code"]`
2. **Runtime guard** in `terraphim_spawner`: reject any dispatch to `opencode/` prefix with error log and alert. Pattern match on model string prefix before spawn.
3. **Budget tracking per provider**: monthly spend counters in `terraphim_goal_alignment`. Soft limit at 80%, hard pause at 100% of provider monthly cap.

### Phase 3: Agent Persona Integration (Issues #32-#33)

**Crates**: `terraphim_config`, `terraphim_rolegraph`
**Skills**: terraphim-engineering-skills (disciplined-design, implementation)

1. **Add persona fields to agent config**: `persona_name`, `persona_symbol`, `persona_vibe`, `meta_cortex_connections` in `[[agents]]` blocks
2. **Inject persona into agent prompt**: `terraphim_spawner` prepends persona identity section to task prompt. Template loaded from `automation/agent-metaprompts/{role}.md`
3. **Meta-cortex routing**: when an agent needs cross-agent consultation, route to agents listed in its `meta_cortex_connections` field

### Phase 4: Skill Chain Configuration (Issues #34-#36)

**Crates**: `terraphim_config`, `terraphim_agent_supervisor`
**Skills mapping**: Each agent role gets a skill chain from terraphim-skills or zestic-engineering-skills

| Agent Role | Skill Chain (terraphim-skills) | Skill Chain (zestic-engineering-skills) |
|---|---|---|
| security-sentinel | security-audit, code-review | quality-oversight, responsible-ai |
| meta-coordinator | session-search, local-knowledge | insight-synthesis, perspective-investigation |
| compliance-watchdog | security-audit | responsible-ai, via-negativa-analysis |
| upstream-synchronizer | git-safety-guard, devops | -- |
| product-development | disciplined-research, architecture | product-vision, wardley-mapping |
| spec-validator | disciplined-design, requirements-traceability | business-scenario-design |
| test-guardian | testing, acceptance-testing | -- |
| documentation-generator | documentation, md-book | -- |
| implementation-swarm | implementation, rust-development | rust-mastery, cross-platform |
| compound-review | code-review, quality-gate | quality-oversight |
| browser-qa | visual-testing, acceptance-testing | frontend |
| brownfield-analyser | architecture, disciplined-research | -- |
| twin-implementer | implementation, rust-development | rust-mastery |
| twin-verifier | testing, disciplined-verification | -- |
| twin-scenario-runner | acceptance-testing | business-scenario-design |

### Phase 5: opencode CLI Integration in Spawner (Issues #37-#38)

**Crates**: `terraphim_spawner`, `terraphim_orchestrator`
**Skills**: terraphim-engineering-skills (implementation, testing, devops)

1. **opencode dispatch**: Add `OpenCodeSession` alongside existing `ClaudeCodeSession` and `CodexSession` in spawner. Invoke: `opencode run -m {provider}/{model} --format json "{prompt}"`. Parse NDJSON output events (`step_start`, `text`, `step_finish`).
2. **Provider auth setup on bigbox**: Run `opencode providers` + `/connect` for each subscription provider. Store auth in `~/.local/share/opencode/auth.json`.
3. **Integration tests**: Test each provider tier with a simple "echo hello" prompt. Verify NDJSON parsing, timeout handling, fallback dispatch.

### Phase 6: orchestrator.toml Update on bigbox (Issue #39)

**Where**: `ssh alex@bigbox`, edit `/opt/ai-dark-factory/orchestrator.toml`

Update all agent definitions with new provider/model fields. Example:

```toml
[[agents]]
name = "security-sentinel"
layer = "Safety"
cli_tool = "opencode"
provider = "opencode-go"
model = "kimi-k2.5"
fallback_provider = "opencode-go"
fallback_model = "glm-5"
persona = "Vigil"
skill_chain = ["security-audit", "code-review", "quality-oversight"]
```

## 5. ADRs to Record

| ADR | Title | Decision |
|---|---|---|
| ADR-002 | Subscription-only model providers | Ban `opencode/` Zen prefix; all routing via subscription providers |
| ADR-003 | Four-tier model routing | Quick/Deep/Implementation/Oracle tiers with fallback chains |
| ADR-004 | Terraphim persona identity layer | Named AI personas (species: Terraphim) for all human-facing agents |
| ADR-005 | kimi-for-coding as implementation tier | `kimi-for-coding/k2p5` for all code generation tasks (implementation swarm, twins, tests) |

## 6. Gitea Issues to Create

| # | Title | Labels | Depends On | Phase |
|---|---|---|---|---|
| 28 | [ADF] Extend orchestrator.toml schema for provider/model/fallback | `type/enhancement` | -- | 1 |
| 29 | [ADF] Add ProviderTier enum to terraphim_config | `type/enhancement` | #28 | 1 |
| 30 | [ADF] Implement fallback dispatch with circuit breaker in spawner | `type/enhancement` | #28, #29 | 1 |
| 31 | [ADF] Subscription guard -- reject opencode/Zen prefix at runtime | `type/security` | #28 | 2 |
| 32 | [ADF] Add persona fields to agent config schema | `type/enhancement` | #28 | 3 |
| 33 | [ADF] Inject persona identity into agent prompts via spawner | `type/enhancement` | #32 | 3 |
| 34 | [ADF] Map skill chains to agent roles in config | `type/enhancement` | #28 | 4 |
| 35 | [ADF] Integrate terraphim-skills into agent dispatch | `type/enhancement` | #34 | 4 |
| 36 | [ADF] Integrate zestic-engineering-skills into agent dispatch | `type/enhancement` | #34, #35 | 4 |
| 37 | [ADF] Implement OpenCodeSession in terraphim_spawner | `type/enhancement` | #28, #29 | 5 |
| 38 | [ADF] Integration tests for opencode provider dispatch | `type/test` | #37 | 5 |
| 39 | [ADF] Update orchestrator.toml on bigbox with new routing | `type/ops` | #30, #37 | 6 |

## 7. Dependencies

- `terraphim_orchestrator` (existing, running on bigbox)
- `terraphim_spawner` (existing, manages CLI subprocess lifecycle)
- `terraphim_config` (existing, TOML config parsing)
- `terraphim_goal_alignment` (existing, budget tracking)
- `terraphim_rolegraph` (existing, role-based KG lookup)
- `terraphim_agent_supervisor` (existing, OTP-style supervision trees)
- `terraphim_persistence` (existing, SQLite/S3 storage)
- terraphim-skills (35 skills, Gitea: terraphim/terraphim-skills)
- zestic-engineering-skills (16 skills, GitHub: zestic-ai/6d-prompts)
- opencode CLI v1.2.27+ (installed at `~/.bun/bin/opencode`)
