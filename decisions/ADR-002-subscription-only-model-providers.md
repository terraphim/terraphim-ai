# ADR-002: Subscription-Only Model Providers for ADF Agent Fleet

**Date**: 2026-03-20
**Status**: Accepted
**Deciders**: Alex (CTO)
**Tags**: architecture, cost-optimisation, model-routing

---

## Context and Problem Statement

In the context of the ADF agent fleet dispatching tasks to LLM providers via the opencode CLI, facing the discovery that the `opencode/` (Zen) provider prefix routes through a pay-per-use proxy with significant markup, we decided to ban the `opencode/` prefix entirely and route all agent dispatch through subscription-based providers, accepting that we must maintain multiple provider subscriptions.

## Decision Drivers

* `opencode/kimi-k2.5` via Zen costs significantly more than `opencode-go/kimi-k2.5` via Go subscription ($10/mo flat)
* The ADF fleet dispatches hundreds of requests daily -- per-request markup compounds rapidly
* All required models are available through subscription providers at predictable monthly costs
* Subscription providers already connected and verified in local `auth.json`

## Considered Options

* **Option A**: Continue using `opencode/` (Zen) prefix for convenience
* **Option B**: Ban `opencode/` prefix, use subscription providers only
* **Option C**: Run local inference to avoid all provider costs

## Decision Outcome

**Chosen option**: Option B -- Ban `opencode/` prefix, subscription providers only

**Reasoning**: All required models (kimi-k2.5, glm-5, minimax-m2.5, k2p5) are available through subscription providers at predictable flat-rate costs. The Go subscription alone ($10/mo) covers 4 models with ~100K requests/mo for minimax. Adding a runtime guard in `terraphim_spawner` prevents accidental use of the expensive Zen proxy.

### Positive Consequences

* Predictable monthly costs across all providers
* No risk of unexpected per-request charges
* Runtime guard catches configuration errors before they incur cost

### Negative Consequences

* Must maintain 5+ provider subscriptions (opencode-go, kimi-for-coding, zai-coding-plan, minimax-coding-plan, github-copilot)
* Provider auth tokens must be renewed/refreshed across all subscriptions
* Some models only available via Zen (e.g., `opencode/big-pickle`) become inaccessible

## Approved Providers

| Provider | Prefix | Pricing |
|---|---|---|
| opencode Go | `opencode-go/` | $10/mo flat |
| Kimi for Coding | `kimi-for-coding/` | Subscription |
| z.ai Coding Plan | `zai-coding-plan/` | Subscription |
| MiniMax Coding Plan | `minimax-coding-plan/` | Subscription |
| GitHub Copilot | `github-copilot/` | Free OSS quota |
| Anthropic | `claude-code` CLI | Subscription |
| **BANNED** | ~~`opencode/`~~ | ~~Pay-per-use~~ |

## Links

* Related to ADR-003 (Four-tier model routing)
* Implements Section 4.1 of `plans/autonomous-org-configuration.md`
* Gitea: terraphim/terraphim-ai #31 (Subscription guard implementation)
