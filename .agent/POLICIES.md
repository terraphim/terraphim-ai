# Agent Policies (terraphim-ai)

Named policies referenced from `manifest.yaml -> components.*.agent_policy`. Dispatchers and human operators must honour the policy attached to the component that owns the changed paths.

## production-code-pairing

- Human approval required before merge or deployment.
- Auto-mode implementation may prepare a PR only when the issue explicitly allows it and all quality gates pass.
- Push gate: human approval plus CI/ADF green.
- Use for: server runtime, sandbox runtime, ADF orchestration, Gitea automation, merge coordination, deployment-sensitive scripts, and production-facing UI integrations.

## auto-mode-ok

- Agents may implement, test, commit, and open PRs after selecting or being assigned a Gitea issue.
- CI/ADF checks are authoritative; failures must be fixed or the issue must be marked blocked.
- Registry-sensitive dependency changes are not covered by this policy and must escalate to `human-review-only` or `production-code-pairing`.
- Use for: isolated tooling crates, validation tools, and low-blast-radius changes with clear tests.

## human-review-only

- Agents may analyse, document, validate, and propose changes.
- Agents must not auto-merge, activate runtime configuration, publish crates, close governance issues, or change private registry boundaries.
- Push gate: explicit human approval.
- Use for: `manifest.yaml`, `.agent/*`, registry configuration, polyrepo boundaries, governance documents, parked/experimental code activation, and release decisions.

## event-only

- Agents may only run when dispatched by the orchestrator from a push, PR, webhook, status, or explicit mention event.
- Agents must not run on a schedule unless moved to another policy through review.
- Use for: deterministic CI agents, native PR gate agents, webhook/status agents, and build runners.

## Promotion Path

- `auto-mode-ok` to `production-code-pairing`: allowed when blast radius increases, user-facing behaviour is affected, or registry/deployment sensitivity appears.
- `production-code-pairing` to `auto-mode-ok`: requires documented evidence of reduced blast radius, test coverage, and stable CI/ADF results.
- `human-review-only` to any more permissive policy: requires a reviewed manifest change.

## Dispatcher Rules

1. Load `manifest.yaml`.
2. Match changed paths to `components.*.source_paths`.
3. Select the strictest matching `agent_policy`.
4. Refuse auto-mode if the policy has `auto_mode_forbidden: true`.
5. For event-only agents, verify an orchestrator event or explicit mention context exists.
