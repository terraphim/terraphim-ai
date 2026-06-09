# Polyrepo Publish Pipeline Guide

## Overview

The polyrepo publish pipeline synchronises six extracted polyrepo splits from Gitea (authoritative) to GitHub (public mirror) and publishes their crates to crates.io. It uses **both CI runners as quality gates** and follows **topological dependency order** to ensure upstream crates are published before downstream crates attempt to resolve them.

## Architecture

### Dual-Runner Quality Gates

```
Gitea (terraphim-native runner)     GitHub (ubuntu-latest runner)
        |                                    |
   Gate 1: Validate                  Gate 2: Validate
   with registry refs                 without registry refs
   (internal deps resolve             (only crates.io deps)
    from Gitea registry)              
        |                                    |
        +------------ Merge back ------------+
                     |
              Publish to crates.io
                     |
              Topological order
```

### Why Two Gates?

1. **Gitea Gate 1**: Validates the code with `registry = "terraphim"` references intact. Downstream crates can still resolve upstream crates from the Gitea registry during this phase.
2. **GitHub Gate 2**: Validates the scrubbed code where all `registry = "terraphim"` references have been stripped. This ensures the public mirror builds correctly using only crates.io dependencies.
3. **Publish**: Only after both gates pass are crates published to crates.io in dependency order.

## Pipeline Steps

The pipeline is implemented in `scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh` and follows these steps:

### Step 1: Clone
Clones the Gitea repo at current `main` HEAD into `/tmp/polyrepo-publish/<repo>`.

### Step 2: Scrub Secrets
Scans for:
- **Blocking patterns**: Actual secrets (Bearer tokens, private keys)
- **Warning patterns**: Infrastructure references (Gitea URLs, vault paths) -- these warn but do not block

### Step 3: Prepare Gitea Branch
- Creates `publish/github-mirror` branch
- Injects GitHub Actions workflows (CI + publish-crates)
- Pushes to Gitea to trigger **Gate 1** (native-ci)

### Step 4: Wait for Gitea CI
Polls Gitea commit statuses for `native-ci` context. Fails if CI fails or times out (30 min).

### Step 5: Rewrite Cargo.toml
Strips all `registry = "terraphim"` references from Cargo.toml files:
```bash
sed -i 's/,[[:space:]]*registry = "terraphim"//g' "$f"
sed -i 's/registry = "terraphim",[[:space:]]*//g' "$f"
```

Also strips `publish = ["terraphim"]` restrictions:
```bash
sed -i '/publish = \["terraphim"\]/d' "$f"
```

### Step 6: Commit Rewrite
Commits the Cargo.toml changes with message: `chore: strip Gitea registry refs for crates.io publish`

### Step 7: Create GitHub Repo
Creates the GitHub repo if it doesn't exist and sets the `CARGO_REGISTRY_TOKEN` secret.

### Step 8: Push to GitHub
Pushes the scrubbed code to GitHub `main` and tags `publish/v<version>`.

### Step 9: Wait for GitHub CI
Polls GitHub Actions for CI completion. Fails if CI fails or times out (30 min).

### Step 10: Merge Back
Merges the `publish/github-mirror` branch back to Gitea `main` via PR (with temporary status check bypass).

### Step 11: Publish Crates
Dispatches the GitHub `publish-crates.yml` workflow with the crate list in topological order.

## Dependency Ordering

Crates must be published in topological order (leaves first, consumers last). The pipeline defines this per repo:

### terraphim-service
```
terraphim_ccusage -> terraphim_usage -> terraphim_file_search ->
terraphim-session-analyzer -> terraphim_spawner -> terraphim_router ->
haystack_core -> haystack_jmap -> haystack_grepapp ->
terraphim_middleware -> terraphim_service
```

### terraphim-agents
```
terraphim_agent_supervisor -> terraphim_agent_evolution -> terraphim_tracker ->
terraphim_task_decomposition -> terraphim_agent_messaging -> terraphim_agent_registry ->
terraphim_goal_alignment -> terraphim_kg_orchestration -> terraphim_multi_agent ->
terraphim_orchestrator
```

### terraphim-clients
```
terraphim_sessions -> terraphim_hooks -> terraphim_update ->
terraphim_negative_contribution -> terraphim_command_runtime -> terraphim_grep ->
terraphim_mcp_server -> terraphim_agent -> terraphim_cli -> terraphim_lsp
```

## Common Failure Modes

### 1. Publish Restrictions Not Fully Stripped
**Symptom**: `error: registry index was not found in any configuration: terraphim`

**Cause**: The old sed pattern `s/publish = ["terraphim"]//g` replaced the directive with nothing but left the rest of the line (including comments). Cargo still interpreted the remaining comment.

**Fix**: Use `/publish = ["terraphim"]/d` to delete the entire line.

### 2. Long Keywords
**Symptom**: `"model-context-protocol" is an invalid keyword (keywords must have less than 20 characters)`

**Fix**: Shorten or remove keywords exceeding 20 characters.

### 3. API Mismatch Between Published and Expected
**Symptom**: `no method named 'with_stderr_log' found for struct 'terraphim_spawner::SpawnContext'`

**Cause**: The published version on crates.io (1.20.3) differed from the source code that included `with_stderr_log`. This happens when the published artifact was built from different code than expected.

**Fix**: Bump the version number and republish.

### 4. Missing Dependencies on crates.io
**Symptom**: `no matching package named 'terraphim_command_runtime' found`

**Cause**: The publish list was not in topological order. A consumer crate was published before its dependency.

**Fix**: Reorder the crate list so dependencies publish first.

### 5. crates.io Rate Limiting
**Symptom**: `status 429 Too Many Requests: You have published too many new crates in a short period of time`

**Cause**: crates.io limits new crate publishes per time window. Publishing 10+ crates in rapid succession triggers this.

**Fix**: Wait for the rate limit window to expire (usually 5-10 minutes), then retry publishing the remaining crates. The workflow is idempotent -- already-published crates are skipped.

### 6. Branch Protection Blocks Merge
**Symptom**: `error: Not allowed to push to protected branch main`

**Fix**: The pipeline creates a PR and temporarily disables required status checks to merge it, then re-enables them.

## Running the Pipeline

### Full Pipeline (all steps)
```bash
export GITEA_TOKEN="your-token"
export GITHUB_TOKEN="your-token"  # via gh auth
export CARGO_REGISTRY_TOKEN="your-crates-io-token"

bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh full <repo-name>
```

### Individual Steps
```bash
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh clone <repo>
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh scrub <repo>
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh prepare-gitea-branch <repo>
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh wait-gitea-ci <repo>
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh rewrite-cargo <repo>
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh commit-rewrite <repo>
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh create-github <repo>
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh push-github <repo>
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh wait-github-ci <repo>
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh merge-back <repo>
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh crates-publish <repo>
```

### Dry Run
```bash
export POLYREPO_DRY_RUN=1
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh full <repo>
```

## Workflow File

The pipeline injects two workflows into each polyrepo:

### `.github/workflows/ci.yml`
Runs on every push/PR:
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo build --workspace`
- `cargo test --workspace --lib --no-fail-fast`

### `.github/workflows/publish-crates.yml`
Triggered manually via `workflow_dispatch`:
- Takes `crate_list` (space-separated package names)
- Takes `dry_run` ("true" or "false")
- Publishes crates in order, skipping already-published versions
- Uses `CARGO_REGISTRY_TOKEN` secret for authentication

## Remote Sync Protocol

After pushing to origin (GitHub), the pipeline pushes to gitea (Gitea) to maintain convergence:

```bash
git fetch origin
git merge origin/main --no-edit
git push origin main
git push gitea main
git diff origin/main gitea/main --stat  # Must be empty
```

## Idempotency

The publish loop is idempotent:
1. Checks if crate version already exists on crates.io
2. Skips if already published
3. Publishes if not present
4. Retries on "already exists" errors

This means partial publishes (e.g., due to rate limits) can be safely retried without republishing already-uploaded crates.

## Environment Variables

| Variable | Required | Purpose |
|----------|----------|---------|
| `GITEA_TOKEN` | Yes | Gitea API authentication |
| `GITHUB_TOKEN` | Yes | GitHub CLI authentication (via `gh auth`) |
| `CARGO_REGISTRY_TOKEN` | Yes | crates.io publish token |
| `POLYREPO_DRY_RUN` | No | Set to "1" to skip all pushes/publishes |
| `POLYREPO_WORK_DIR` | No | Working directory (default: `/tmp/polyrepo-publish`) |
| `POLYREPO_PUBLISH_MODE` | No | "approved" or "dependency" (default: "dependency") |

## Troubleshooting Checklist

- [ ] `GITEA_TOKEN` is set and valid
- [ ] `gh auth status` shows logged in
- [ ] `CARGO_REGISTRY_TOKEN` is set on GitHub repos (check with `gh secret list`)
- [ ] Gitea native-ci workflow exists and is triggered on `publish/github-mirror` branch
- [ ] Crate versions are correct (no `version.workspace = true` in published manifests)
- [ ] Keywords are under 20 characters
- [ ] No `publish = ["terraphim"]` restrictions remain after rewrite
- [ ] Dependency order is correct (leaves first)

## References

- Original ADR: `docs/architecture/adr/0002-polyrepo-github-publish-pipeline.md`
- Pipeline script: `scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh`
- Tracking issue: Gitea #2260
