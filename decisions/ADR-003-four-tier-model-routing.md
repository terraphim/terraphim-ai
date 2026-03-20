# ADR-003: Four-Tier Model Routing for ADF Agent Fleet

**Date**: 2026-03-20
**Status**: Accepted
**Deciders**: Alex (CTO)
**Tags**: architecture, model-routing, performance

---

## Context and Problem Statement

In the context of 18+ ADF agents with varying computational requirements, facing the need to balance cost, latency, and capability across different task types, we decided for a four-tier routing model (Quick/Deep/Implementation/Oracle) with automatic fallback chains, accepting increased configuration complexity in `orchestrator.toml`.

## Decision Drivers

* CJE calibration data (2026-03-19) proved different models excel at different tasks: minimax for advisory, GLM-5 for quality gates, kimi-k2.5 for NO-GO detection, opus-4-6 for deep reasoning
* Cost varies 100x between tiers (Go $10/mo vs Anthropic subscription)
* Agents need resilient dispatch -- single provider outage should not halt the fleet
* Latency requirements differ: docs generation tolerates 30s, security scanning needs sub-60s

## Considered Options

* **Option A**: Single model for all agents (simplest, but suboptimal)
* **Option B**: Per-agent model assignment without tiers (flexible, but no structure)
* **Option C**: Four-tier routing with fallback chains (structured, cost-aware)

## Decision Outcome

**Chosen option**: Option C -- Four-tier routing with fallback chains

**Reasoning**: Tiers provide a structured framework that maps task complexity to model capability. CJE calibration data validates the tier boundaries. Fallback chains provide resilience. The `ProviderTier` enum in `terraphim_config` makes this machine-readable.

### Tier Definitions

| Tier | Primary Provider/Model | Fallback | Latency Target | Use Case |
|---|---|---|---|---|
| **Quick** | `opencode-go/minimax-m2.5` | `zai-coding-plan/glm-4.7-flash` | <30s | Docs, advisory, routine tasks |
| **Deep** | `opencode-go/glm-5` | `opencode-go/kimi-k2.5` | <60s | Quality gates, compound review, security |
| **Implementation** | `kimi-for-coding/k2p5` | `opencode-go/kimi-k2.5` | <120s | Code generation, twins, tests, implementation swarm |
| **Oracle** | `claude-code --model opus-4-6` | -- | <300s | Spec validation, deep reasoning, brownfield analysis |

### Positive Consequences

* Cost-optimised: routine tasks never touch expensive Oracle tier
* Resilient: every non-Oracle agent has a fallback path
* Auditable: tier assignment is explicit in config, not implicit in code
* Extensible: new tiers can be added as provider landscape evolves

### Negative Consequences

* Oracle tier has no fallback (intentional -- these tasks require highest capability)
* Circuit breaker adds complexity to spawner
* Tier assignment may need recalibration as models improve

## Links

* Related to ADR-002 (Subscription-only providers)
* Validated by CJE calibration: `automation/judge/calibration-comparison-*-2026-03-19.json`
* Implements Section 4.3 of `plans/autonomous-org-configuration.md`
* Gitea: terraphim/terraphim-ai #29 (ProviderTier enum)
