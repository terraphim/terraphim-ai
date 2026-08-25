# Design: allow `terraphim-proxy` provider prefix (C1 gate parity)

**Issue:** terraphim/digital-twins#161 (cross-repo; implementing repo: terraphim/terraphim-ai)
**Date:** 2026-08-25
**Status:** design gate — implementation preceded this artifact during incident
investigation; **no commit or PR will occur until the design gate and an
independent review gate pass.**

## Provenance disclosure

The working-tree changes (`ALLOWED_PROVIDER_PREFIXES` addition, error message,
Rust tests, `migrate-to-confd.py` pre-flight, Python tests) were written first,
while diagnosing why bigbox agents could not route through the Terraphim LLM
proxy during an incident. This document is the retrospective design gate for
that already-written diff. Per the workflow, the diff stays uncommitted until
this design passes review by an independent reviewer; the reviewer should treat
the code as a proposal, not a fait accompli.

## Problem

Bigbox runs a Terraphim LLM proxy that fronts subscription-only upstreams.
Agents that should use it cannot declare so: the C1 subscription allow-list in
`terraphim_orchestrator` rejects the `terraphim-proxy/...` prefix, forcing
operators onto per-provider entries and blocking the incident-time routing
need. Conversely, the author's audit of the Python migration pre-flight found
the opposite defect: `_is_allowed` exempted absolute executable paths in
`model`/`fallback_model`, which the Rust validator rejects — a parity break
that would let the pre-flight wave through configs the orchestrator then
refuses at load time (fail-late, confusing, and a gate-bypass primitive if the
Rust gate were ever relaxed).

## Scope

- Add `terraphim-proxy` to `ALLOWED_PROVIDER_PREFIXES` in
  `crates/terraphim_orchestrator/src/config.rs` (load-time validator and
  runtime gate share this list).
- Extend the `BannedProvider` error string with the new allowed id.
- Mirror the prefix in `scripts/adf-setup/migrate-to-confd.py`
  (`ALLOWED_PREFIXES`) so the C1 pre-flight matches the Rust gate.
- Remove the absolute-path exemption from the Python `model`/`fallback_model`
  validation (parity fix); keep `fallback_provider` unvalidated, as on the
  Rust side.
- Focused tests both sides; update `docs/adf/model-selection-and-spawn.md`.

## Current schema / contracts

Agent TOML (`[[agents]]`) fields relevant here:

| Field | Meaning | Validated? |
|---|---|---|
| `model` | LLM route string | yes — `validate_model_provider` at load; `is_allowed_provider` at runtime |
| `fallback_model` | LLM route used if primary fails | yes — same gates |
| `fallback_provider` | CLI binary path (e.g. `/home/alex/.bun/bin/opencode`) | **no** — names an executable, not an LLM route |

`is_allowed_provider` / `validate_model_provider` semantics (both languages):

- `provider/model` form: the prefix before the first `/` must be in the
  allow-list (`claude-code`, `opencode-go`, `kimi-for-coding`,
  `minimax-coding-plan`, `openai`, `zai-coding-plan`, `terraphim-proxy`) or in
  `ANTHROPIC_BARE_PROVIDERS` (`anthropic`); banned prefixes
  (`opencode`, `github-copilot`, `google`, `huggingface`, `minimax`) take
  precedence and are rejected.
- Bare form (no `/`): must be `sonnet`/`opus`/`haiku` (claude-code CLI),
  `anthropic`, or an allow-list provider id. Unknown bare names are rejected.
- Absolute paths (leading `/`): the prefix before the first `/` is empty, so
  they fall through to the unknown-prefix rejection. They are **never** valid
  `model`/`fallback_model` values; they are only legitimate as the unvalidated
  `fallback_provider`.

Drift control: Python tests parse the Rust constants from source and assert
list equality (`test_banned_list_matches_rust`, `test_allowed_list_matches_rust`).

## Exact-prefix policy

Matching is exact equality on the `prefix/` boundary — never substring.
`terraphim-proxy-evil/auto`, `terraphim-proxyx/auto`, and
`not-terraphim-proxy/auto` are all rejected; only `terraphim-proxy/…` matches.
The Rust list stores prefixes without the trailing slash and compares with
`split_once('/')`; the Python list stores slash-terminated prefixes and
compares with `startswith`, which is equally exact because of the trailing
slash. Both sides have tests pinning lookalike rejection so neither
implementation can drift into substring matching.

## Proxy semantic routes

`terraphim-proxy/` exposes three semantic routes, each mapping to
subscription-only upstream selection inside the Bigbox proxy:

| Route | Intended use |
|---|---|
| `terraphim-proxy/auto` | default routing, proxy picks the upstream |
| `terraphim-proxy/background` | long-running/background agents, cost-lean upstreams |
| `terraphim-proxy/think` | deep-reasoning tasks, heavyweight upstreams |

The bare id `terraphim-proxy` is also accepted (mirrors every other allow-list
id). Nothing after the `/` is validated by the C1 gate — the proxy owns route
semantics; the gate only owns the prefix (who gets traffic), not the routing.

## Fallback / fail-closed semantics

- All unknowns fail closed: unknown prefix, unknown bare name, and (after this
  fix) absolute paths in `model`/`fallback_model` all exit non-zero in the
  Python pre-flight and return `BannedProvider` at Rust load time.
- `fallback_provider` is deliberately unvalidated on both sides: it names a
  CLI binary that `spawn_with_fallback` launches when the primary fails, and
  requiring it to look like an LLM route would break every fleet config.
  Keeping it unvalidated is the parity-preserving behaviour, not an oversight.
- The Python pre-flight must never be *more* permissive than the Rust gate:
  a config that passes migration must not fail orchestrator load. The removed
  absolute-path exemption was exactly such a hole.
- Consequence (behaviour change): the pre-flight now rejects unknown
  `provider/...` prefixes and unknown bare model names that previously
  migrated successfully — and would only have failed later at Rust load-time
  validation, if they ever got that far. Fail-late became fail-early, which
  is intentional, but it can break a fleet migration run that used to
  succeed. See *Deployment / rollback* for the fleet config sweep and canary
  required before the stricter script ships.

## Test plan

Rust (`cargo test -p terraphim_orchestrator`):

- `--test provider_gate_tests`: `terraphim_proxy_semantic_routes_pass_c1_gate`,
  `terraphim_proxy_lookalike_prefixes_rejected` (plus all pre-existing
  C1/C3/probe/cost tests stay green).
- `--test provider_gate_tests`:
  `banned_provider_error_guidance_lists_every_allowed_prefix` — review
  remediation (F1/F3): the rendered `BannedProvider` guidance, produced via
  `validate()`, must name every prefix in `ALLOWED_PROVIDER_PREFIXES`. It
  failed at the PR head because `openai` was missing from the message.
- `--lib`: `config::tests::test_terraphim_proxy_semantic_routes_allowed`,
  `config::tests::test_terraphim_proxy_lookalikes_rejected` (plus existing
  `is_allowed_provider` / `validate_model_provider` tests).

Python (`uv run pytest scripts/adf-setup/tests/test_migrate.py`):

- Acceptance: `terraphim-proxy/{auto,background,think}` and bare
  `terraphim-proxy` as `model` and as `fallback_model`.
- Acceptance (review remediation, F4):
  `test_compound_review_terraphim_proxy_model_accepted` —
  `compound_review.model = "terraphim-proxy/think"` migrates with exit 0 and
  is carried verbatim into the emitted base config, exercising the
  `[compound_review]` branch of the pre-flight gate.
- Rejection: lookalikes (`not-terraphim-proxy`, `terraphim-proxy-evil`,
  `terraphim-proxyx`, bare and prefixed), raw `opencode/`, unknown
  pay-per-use prefixes, and the two new parity regressions — absolute path as
  `model` and absolute path as `fallback_model` both exit non-zero with agent
  name, offending value, and failing field in stderr.
- `fallback_provider` untouched: fixture with
  `fallback_provider = "/home/alex/.bun/bin/opencode"` migrates successfully
  **and** the path is asserted verbatim in the emitted `conf.d` output —
  proving success comes from the field being unvalidated, not from any
  absolute-path allowance in the model gate.
- Drift: banned- and allowed-list equality against the Rust source.

Formatting: `cargo fmt -p terraphim_orchestrator -- --check`.

## Acceptance criteria

1. `terraphim-proxy/auto|background|think` and bare `terraphim-proxy` pass the
   C1 gate (Rust load-time and runtime, Python pre-flight) as `model` and
   `fallback_model`.
2. Lookalike and unknown prefixes fail closed on both sides with an actionable
   error naming agent, value, and field; the rendered Rust `BannedProvider`
   guidance lists every prefix in `ALLOWED_PROVIDER_PREFIXES` (pinned by
   `banned_provider_error_guidance_lists_every_allowed_prefix`).
3. Python and Rust allow-lists/ban-lists are byte-identical in membership
   (enforced by drift tests).
4. Absolute executable paths are rejected as `model`/`fallback_model` on both
   sides; `fallback_provider` remains unvalidated and migrates verbatim.
5. All focused Python tests, provider Rust tests, and `cargo fmt --check` pass.
6. `compound_review.model` accepts `terraphim-proxy/` routes through the real
   migration path (pinned by
   `test_compound_review_terraphim_proxy_model_accepted`).
7. Independent reviewer signs off on this design and the diff before any
   commit/PR.

## Out of scope

- Proxy-side route implementation, upstream selection, auth, or rate limiting
  (lives in the proxy repo, not this gate change).
- Changes to deployed `/opt/ai-dark-factory/conf.d/*.toml` or any fleet agent
  templates — switching existing agents to `terraphim-proxy/` routes is a
  separate, individually reviewable migration.
- `fallback_provider` validation or refactoring it into the model gates.
- Any change to banned-prefix policy, cost tracking, or routing engine
  internals.

## Deployment / rollback

- Deploy: merge to `main`, then on bigbox rebuild the orchestrator from the
  agents repo per AGENTS.md (`cargo build --release -p terraphim_orchestrator`
  → `adf` → systemd restart). Gate change is load-time; existing configs with
  previously allowed providers keep loading unchanged.
- Deploy the stricter pre-flight only after a fleet sweep + canary:
  `scripts/adf-setup/migrate-to-confd.py` now rejects unknown/bare model
  providers that previously migrated but would later fail Rust validation
  (see *Fallback / fail-closed semantics*). Before shipping that script:
  1. **Fleet config sweep** — scan every monolithic orchestrator TOML the
     fleet still feeds the migrator for `model`/`fallback_model` (agent and
     `[compound_review]`) values that are neither allow-listed
     `provider/...` routes nor known bare ids; fix or remove them.
  2. **Canary** — run the new script against one project's input
     (`--dry-run`, then a real single-project migration) and confirm exit 0
     plus verbatim `fallback_provider` carry-through before rolling it out
     fleet-wide. A previously-green migration that now exits non-zero is the
     sweep working as intended, not a script regression.
- Verify: `adf --check` on the deployed base config plus one agent switched to
  `terraphim-proxy/auto` as a canary; confirm the spawn route in logs.
- Rollback: revert the merge and redeploy the previous `adf` binary. The
  allow-list removal re-breaks `terraphim-proxy/` configs, so the canary agent
  must first be switched back to a previously allowed provider; no persisted
  state depends on the new prefix (it is config-parse-time only).
