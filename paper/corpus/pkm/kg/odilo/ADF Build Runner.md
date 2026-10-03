# ADF Build Runner

CI/CD for Odilo runs through AI Dark Factory (ADF) agents on bigbox rather than hosted CI alone. The `build-runner` agent (configured in `conf.d/odilo.toml`) is triggered by a Gitea push webhook (`/webhooks/gitea`), fetches the commit, runs fmt/clippy/build/test, and posts an `adf/build` commit status (~486s wall time). A companion `odilo-pr-reviewer` agent runs code-review and structural-pr-review skills on PR events. The orchestrator is managed via systemd (`adf-orchestrator`). This work also seeded the Terraphim CI Compiler initiative (GitHub Actions to Gitea Actions in Firecracker microVMs).

synonyms:: adf build runner, build-runner, adf ci, odilo-pr-reviewer, adf orchestrator, gitea webhook build, dark factory ci

## Related Concepts
- ZDP
- Decision Quality Reviewer

## Sources
- `.agent/handoffs/2026-07-10-adf-build-runner-odilo-deployment.md`
- `odilo/.docs/research-ci-github-actions-vs-gitea-actions-firecracker-2026-07-10.md`
