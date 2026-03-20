# ADR-005: kimi-for-coding/k2p5 as Implementation Tier Model

**Date**: 2026-03-20
**Status**: Accepted
**Deciders**: Alex (CTO)
**Tags**: architecture, model-selection, cost-optimisation

---

## Context and Problem Statement

In the context of selecting a primary model for the implementation tier (code generation, twin building, test writing, implementation swarm), facing multiple candidates (`github-copilot/claude-sonnet-4.6`, `opencode/claude-sonnet-4`, `kimi-for-coding/k2p5`), we decided for `kimi-for-coding/k2p5` as the implementation tier model, accepting dependency on the Kimi for Coding subscription.

## Decision Drivers

* Implementation swarm (5-15 agents) generates the highest volume of code-generation requests
* `kimi-for-coding/k2p5` is a code-specialised model optimised for programming tasks
* Kimi for Coding is a flat-rate subscription -- no per-token billing regardless of volume
* CJE calibration showed kimi-k2.5 has 62.5% NO-GO detection rate (best tested)
* `github-copilot/claude-sonnet-4.6` routes through Copilot which may have rate limits under heavy swarm usage
* `opencode/claude-sonnet-4` routes through Zen (banned per ADR-002)

## Considered Options

* **Option A**: `github-copilot/claude-sonnet-4.6` (Copilot free OSS quota)
* **Option B**: `kimi-for-coding/k2p5` (Kimi subscription, code-specialised)
* **Option C**: `zai-coding-plan/glm-4.7` (z.ai subscription)

## Decision Outcome

**Chosen option**: Option B -- `kimi-for-coding/k2p5`

**Reasoning**: Code-specialised model on flat-rate subscription is ideal for high-volume implementation workloads. The fallback to `opencode-go/kimi-k2.5` provides resilience within the same model family. GitHub Copilot remains available for ad-hoc use but is not the primary dispatch target for the swarm.

### Agents Using This Model

| Agent | Purpose |
|---|---|
| implementation-swarm (x5-15) | Gitea issue implementation |
| upstream-synchronizer | Repo sync, patch equivalence |
| test-guardian | PR testing, CI/CD quality gates |
| twin-implementer | Digital twin crate building |
| twin-verifier | SDK validation tests |

### Positive Consequences

* Predictable cost for highest-volume workload
* Code-specialised model likely produces better code than general-purpose alternatives
* Consistent model family (kimi) across implementation and fallback
* `kimi-k2-thinking` available as upgrade path for complex implementation tasks

### Negative Consequences

* Single vendor dependency for implementation tier
* If Moonshot subscription changes pricing, cost model breaks
* Less proven than Claude Sonnet for Rust code generation (needs validation)

## Links

* Related to ADR-002 (Subscription-only providers)
* Related to ADR-003 (Four-tier model routing)
* Implements Section 4.1 of `plans/autonomous-org-configuration.md`
* Gitea: terraphim/terraphim-ai #37 (OpenCodeSession implementation)
