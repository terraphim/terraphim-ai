# Release Runbook (Gitea-authoritative, GitHub publish)

Terraphim releases follow ADR-0001/0002: **Gitea is source of truth**; **GitHub publishes public artefacts**.

## Repositories

| Repo | Role |
|------|------|
| `git.terraphim.cloud/terraphim/terraphim-ai` | Monorepo: server, Docker, Debian, Homebrew |
| `git.terraphim.cloud/terraphim/terraphim-clients` | Client binaries: agent, cli, grep |
| `git.terraphim.cloud/terraphim/terraphim-service` | `terraphim_service` crate + Gitea registry |
| `github.com/terraphim/terraphim-ai` | Public mirror + GitHub Releases (auto-update URLs) |

Client binaries are **built** in `terraphim-clients` and **attached** to the `terraphim-ai` GitHub release.

## Prerequisites

```bash
source ~/.profile   # GITEA_URL, GITEA_TOKEN
export CARGO_REGISTRIES_TERRAPHIM_TOKEN="Bearer $GITEA_TOKEN"
```

Verify dual-remote sync before tagging:

```bash
git fetch origin && git fetch gitea
git diff origin/main gitea/main --stat   # must be empty
```

## Release procedure (vX.Y.Z)

### 0. Registry: republish broken or changed crates

If `terraphim_service` (or other registry deps) changed on Gitea:

```bash
# In terraphim-service repo on Gitea main
cargo check -p terraphim_service --features openrouter
cargo publish -p terraphim_service --registry terraphim --allow-dirty
```

Bump `terraphim_service = { version = "X.Y.Z", registry = "terraphim" }` in:
- `terraphim-ai` (`terraphim_server`, `terraphim_rlm`, `terraphim_github_runner*`, `terraphim_ai_nodejs`)
- `terraphim-clients` (via polyrepo publish)

### 1. Gitea CI green on main

Merge features on Gitea; wait for `native-ci` success.

### 2. Polyrepo publish (if client/core crates changed)

```bash
POLYREPO_DRY_RUN=0 bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh full terraphim-clients
# Repeat for terraphim-service, terraphim-core, etc. as needed
```

This injects `release-binaries.yml` into `terraphim-clients` and syncs to GitHub.

### 3. Bump workspace version on Gitea main

```bash
# Cargo.toml workspace.package.version and dependent crate pins
cargo metadata --no-deps --format-version=1 | jq '.packages[] | {name: .name, version: .version}'
```

### 4. Tag and push both remotes

```bash
git tag vX.Y.Z
git push origin main
git push gitea main
git push origin vX.Y.Z
git push gitea vX.Y.Z
git diff origin/main gitea/main --stat   # verify empty
```

### 5. Monitor GitHub workflows

On `terraphim/terraphim-ai` tag `vX.Y.Z`:

1. `release-comprehensive.yml` — server binaries, Docker, Debian, GitHub release
2. Dispatches `terraphim-clients` → `release-binaries.yml` (attaches agent/cli/grep)
3. `wait-for-client-binaries` — polls release asset count
4. `update-homebrew` — updates tap formulas

Check:

```bash
gh release view vX.Y.Z --repo terraphim/terraphim-ai
# Expect server + client assets and checksums.txt
```

### 6. Acceptance checks

| Check | Command |
|-------|---------|
| Server version | `./terraphim_server --version` |
| Release assets | `gh release view vX.Y.Z --repo terraphim/terraphim-ai --json assets` |
| Registry crate | `cargo search terraphim_service --registry terraphim` |
| Homebrew | `brew update && brew info terraphim-server` |

## Manual crate publish (monorepo only)

```bash
gh workflow run publish-crates.yml \
  --repo terraphim/terraphim-ai \
  -f crate_list="terraphim_update terraphim_github_runner" \
  -f dry_run=true
```

Extracted crates use each polyrepo's `publish-crates.yml` with `crate_list` from `polyrepo-publish.sh`.

## Recovery: failed release with zero assets

Do **not** force-push tags. Fix workflows, bump to next patch (e.g. v1.20.4 → v1.20.5), re-tag after CI green.

## Secrets

| Secret | Repo | Purpose |
|--------|------|---------|
| `CARGO_REGISTRIES_TERRAPHIM_TOKEN` | terraphim-clients | Private registry during build |
| `CLIENTS_REPO_TOKEN` | terraphim-ai | Dispatch clients workflow |
| `TERRAPHIM_AI_RELEASE_TOKEN` | terraphim-clients | Upload binaries to terraphim-ai release |
| `OP_SERVICE_ACCOUNT_TOKEN` | terraphim-ai, terraphim-clients | macOS signing via 1Password |