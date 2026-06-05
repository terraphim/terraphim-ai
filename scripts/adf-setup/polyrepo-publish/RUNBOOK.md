# Polyrepo Publish Runbook

This runbook promotes the Gitea split repos into public GitHub repositories and
publishes dependency crates. It uses ADF locally first, then bigbox.

## 1. Local Dry-run

Run from the `terraphim-ai` repository root:

```bash
bash -n scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh
POLYREPO_DRY_RUN=1 adf-ctl --local flow polyrepo-publish-local
```

This runs `.terraphim/flows/polyrepo-publish-local.toml` and validates:

- Gitea split repos can be cloned.
- Secret scrub blocks unsafe content.
- Cargo registry rewrites do not delete dependency lines.
- Dry-run guards prevent Gitea pushes, GitHub repo creation, GitHub workflow
  dispatch, merge-back, and crates.io publish.

## 2. Optional Local Direct-dispatch Smoke

`--direct` is available on `adf-ctl --local trigger`, not on `flow`. Use it for
agent/event dispatch smoke tests when the production flow depends on webhook or
event-triggered agents:

```bash
adf-ctl --local trigger polyrepo-publish-smoke \
  --direct \
  --event push \
  --sha "$(git rev-parse HEAD)" \
  --ref-name refs/heads/main
```

If no flow-triggered agent changed, this step can be skipped.

## 3. Install on Bigbox

Copy the script and flow config to bigbox:

```bash
ssh bigbox 'mkdir -p /opt/ai-dark-factory/scripts/polyrepo-publish'
scp scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh \
  bigbox:/opt/ai-dark-factory/scripts/polyrepo-publish/polyrepo-publish.sh
scp scripts/adf-setup/polyrepo-publish/polyrepo-publish-flow.toml \
  bigbox:/opt/ai-dark-factory/polyrepo-publish-flow.toml
ssh bigbox 'chmod +x /opt/ai-dark-factory/scripts/polyrepo-publish/polyrepo-publish.sh'
```

Append or include `polyrepo-publish-flow.toml` in
`/opt/ai-dark-factory/orchestrator.toml`.

Validate before restart:

```bash
ssh bigbox 'cd /opt/ai-dark-factory && /usr/local/bin/adf --check orchestrator.toml'
```

Restart only after validation passes.

## 4. Bigbox Dry-run

```bash
POLYREPO_DRY_RUN=1 adf-ctl trigger polyrepo-publish --wait
```

This verifies production paths, tokens, `gh` authentication, staging directory
access, and ADF flow wiring without mutating GitHub, Gitea main, or crates.io.

## 5. Production Run

```bash
POLYREPO_DRY_RUN=0 POLYREPO_PUBLISH_MODE=dependency \
  adf-ctl trigger polyrepo-publish --wait
```

The production run executes each repo as a complete cycle before starting the
next repo:

1. Clone from Gitea.
2. Scrub secrets.
3. Rewrite Cargo registry references.
4. Push `publish/github-mirror` to Gitea.
5. Wait for Gitea `native-ci` on `terraphim-native`.
6. Create/update GitHub repo and workflows.
7. Push to GitHub.
8. Wait for GitHub Actions CI on `ubuntu-latest`.
9. Merge back to Gitea main.
10. Dispatch GitHub `publish-crates.yml` for dependency crates.

## 6. Abort and Rollback

If any gate fails, the flow aborts. Do not continue manually until the failed
runner log is reviewed.

Rollback GitHub publication for a repo by resetting the public repo to the
previous known-good tag or commit. Gitea remains authoritative; do not rewrite
Gitea main unless a published merge-back commit is known to be bad.
