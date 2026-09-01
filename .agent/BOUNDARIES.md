# Global Boundaries (terraphim-ai)

Repo-wide rules. Apply to every agent, every component, every PR. Component-specific boundaries may be added next to source directories later, but they may not relax this file.

## Never Touch Without Explicit Instruction

- `.env`, `.env.*`, and any file containing real credentials. Use `op://` references, `op inject`, or exported environment variables.
- `Cargo.lock`, root `Cargo.toml`, and `.cargo/config.toml` when the change affects private `terraphim` registry resolution, workspace membership, or `[patch]` entries.
- `.terraphim/orchestrator.toml.bigbox` and deployed systemd files unless the task is explicitly about ADF deployment.
- Machine-specific Cargo overrides such as local path overrides for Bigbox. If required, keep them uncommitted and document the operational step.
- `manifest.yaml` and `.agent/POLICIES.md` without updating validation and the planning rationale.
- Generated artefacts unless the canonical generator is run and documented.
- Third-party vendored or parked code under `lab/parking-lot` unless the task is explicitly scoped there.

## Never Use

- Hardcoded secrets or tokens in code, config, tests, or documentation.
- Destructive git commands such as `git reset --hard` or `git checkout --` without explicit approval.
- `--no-verify` to bypass hooks.
- Manual copying to Bigbox for deployment. Use git push/pull according to `AGENTS.md`.
- `[patch]` in the workspace root for Bigbox-only orchestrator overrides.
- Test mocks for code that can be validated against real local services or hermetic fixtures.

## Always Do

- Read `manifest.yaml` first and identify the component whose `source_paths` cover the task.
- Apply the component's `agent_policy` from `.agent/POLICIES.md` before deciding whether an agent may implement, merge, or only report.
- Use Gitea as the task authority for `terraphim/terraphim-ai`.
- For registry-sensitive work, confirm whether dependencies resolve from local source, the private `terraphim` registry, crates.io, or git.
- Run the smallest relevant quality gate for changed code, and include evidence in the handoff.
- Keep runtime agent templates disabled unless they are explicitly wired into `.terraphim/orchestrator.toml.bigbox` or `conf.d/*.toml` after review.

## Component Policy Stacking

The strictest applicable rule wins. Registry, deployment, Gitea automation, sandbox, and server-runtime rules override general `auto-mode-ok` permissions.

If a task needs to cross component boundaries, escalate to the stricter policy and document the reason in the relevant Gitea issue or PR.
