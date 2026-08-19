# Design: restore ADF PR-gate model routing

**Issue:** terraphim/terraphim-ai#3266  
**Coordination:** terraphim/agent-tasks#96  
**Approved parent plan:** `/home/alex/clawd/.hermes/plans/2026-08-18_214843-adf-fleet-convergence-fix.md` at SHA-256 `3241db9d2321dcc22f2766619c876e99927ecb3ac34bbe9aaa02f003ab2e3b97`

## Problem

Historical gate logs showed `No models match pattern` followed by missing-result factory failures. Current live evidence supersedes that observation for routing: bigbox now resolves and executes `kimi-for-coding/k3`. Clean process exit and a negative or malformed gate verdict remain distinct. This slice reconciles versioned template drift and proves the current route; it must not rewrite a healthy deployed identifier merely because an earlier snapshot differed.

The authoritative deployment probe is bigbox: `/usr/local/bin/opencode` v1.18.18 listed `kimi-for-coding/k3`, and an exact live run returned `BIGBOX_KIMI_ROUTE_OK` on 2026-08-19. The local-only custom route `kimi-coding/kimi-k3` is not installed on bigbox and failed there, so it is not a deployable identifier. Repository templates still contain obsolete `kimi-for-coding/k2p5` for the three native gate producers.

## Scope

- Update only applicable native gate templates under `scripts/adf-setup/agents/`: `pr-reviewer.toml`, `pr-validator.toml`, and `pr-verifier.toml`.
- Preserve Claude subscription `sonnet` as primary unless source semantics prove that combination cannot express a provider-specific fallback safely.
- Make the OpenCode fallback provider/model pairing explicit if required by the existing config schema.
- Add focused contract coverage that parses the templates and asserts the three gate producers use an allowed subscription-only route.
- Document that `migrate-to-confd.py` is one-shot, not a live regeneration authority.

## Out of scope

- Deployed `/opt/ai-dark-factory/conf.d/*.toml` mutation; orchestrator performs that as a hash-captured canary after this PR is reviewed.
- Branch protection, auto-merge identities, other agent templates, provider credentials, or unrelated model migrations.

## Safety contracts

- No metered OpenCode Zen provider.
- No credentials in source, logs, fixtures, or PR evidence.
- Route configuration must fail closed when provider/model is unavailable.
- Existing valid fallback order and native gate-result validation remain unchanged.

## Tests

1. RED: parse the three template TOML files and assert their fallback route is exactly the bigbox-live-smoked OpenCode subscription route.
2. GREEN: make the smallest template/schema-compatible change.
3. Run the focused route-contract test, TOML/config parser tests, formatting, and scoped repository gates.
4. Independently review the resulting diff before commit/PR.

## Acceptance

- The three native gate templates no longer reference stale `kimi-for-coding/k2p5` or ambiguous `k3` routes.
- Their fallback resolves to live-smoked `kimi-for-coding/k3` via the configured OpenCode binary/provider semantics.
- Focused and scoped gates pass.
- Deployment remains a separate reversible canary transaction.