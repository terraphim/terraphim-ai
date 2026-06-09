# How to Publish Polyrepo Splits to crates.io

This guide explains how to publish the extracted polyrepo splits (terraphim-core, terraphim-service, terraphim-agents, etc.) from our private Gitea instance to the public crates.io registry.

## What This Pipeline Does

Our code lives in two places:
- **Gitea** (private): Where we do our daily work
- **GitHub** (public): Mirror for the open-source community
- **crates.io**: Where Rust developers download our libraries

The pipeline automates syncing from Gitea to GitHub, then publishes crates to crates.io in the correct order so dependencies resolve correctly.

## Before You Start

You need three tokens ready:

```bash
# 1. Gitea API token (for accessing our private repos)
export GITEA_TOKEN="your-gitea-token"

# 2. GitHub CLI auth (should already be configured)
gh auth status  # Should show "Logged in"

# 3. crates.io publish token
export CARGO_REGISTRY_TOKEN="your-crates-io-token"
```

## Quick Start

To publish a complete polyrepo split:

```bash
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh full terraphim-service
```

This one command does everything:
1. Clones the repo from Gitea
2. Checks for leaked secrets
3. Runs tests on our private CI
4. Strips internal registry references
5. Pushes to GitHub
6. Runs tests on GitHub Actions
7. Publishes all crates to crates.io

The whole process takes about 10-15 minutes depending on CI queue times.

## Available Repos

You can publish any of these six splits:

- `terraphim-core` - Core types and parsers
- `terraphim-config-persistence` - Configuration and persistence layer
- `terraphim-service` - Service layer, middleware, and haystack providers
- `terraphim-agents` - Agent orchestration and messaging
- `terraphim-kg-agents` - Knowledge graph agents
- `terraphim-clients` - CLI tools, MCP server, and integrations

## How It Works (Step by Step)

### Step 1: Clone from Gitea

The pipeline clones the latest code from our private Gitea instance.

### Step 2: Security Scan

Checks for accidentally committed secrets:
- **Blocking**: Actual secrets (API keys, tokens) -- stops the pipeline
- **Warning**: Infrastructure references (Gitea URLs, vault paths) -- logs a warning but continues

### Step 3: Test on Private CI (Gate 1)

Creates a `publish/github-mirror` branch and pushes it to Gitea. This triggers our private CI runner (`terraphim-native`) which:
- Compiles all crates
- Runs `cargo clippy` (linting)
- Runs unit tests

The pipeline waits up to 30 minutes for this to pass. If it fails, the pipeline stops.

### Step 4: Prepare for Public Release

Strips internal references from `Cargo.toml` files:

```toml
# Before (internal Gitea registry reference)
terraphim_types = { version = "1.0.0", registry = "terraphim" }

# After (clean, public-facing)
terraphim_types = { version = "1.0.0" }
```

Also removes `publish = ["terraphim"]` restrictions that prevent publishing to crates.io.

### Step 5: Test on Public CI (Gate 2)

Pushes the cleaned code to GitHub and triggers GitHub Actions. This validates that the code builds using **only** crates.io dependencies (no private registry).

### Step 6: Merge Back

Merges the publish branch back to Gitea `main` so both remotes stay in sync.

### Step 7: Publish to crates.io

Dispatches the publish workflow on GitHub, which uploads crates in dependency order.

## Crate Publish Order

Crates must be published in dependency order. If crate A depends on crate B, crate B must publish first.

### terraphim-service
```
1. terraphim_ccusage
2. terraphim_usage
3. terraphim_file_search
4. terraphim-session-analyzer
5. terraphim_spawner
6. terraphim_router
7. haystack_core
8. haystack_jmap
9. haystack_grepapp
10. terraphim_middleware
11. terraphim_service
```

### terraphim-agents
```
1. terraphim_agent_supervisor
2. terraphim_agent_evolution
3. terraphim_tracker
4. terraphim_task_decomposition
5. terraphim_agent_messaging
6. terraphim_agent_registry
7. terraphim_goal_alignment
8. terraphim_kg_orchestration
9. terraphim_multi_agent
10. terraphim_orchestrator
```

### terraphim-clients
```
1. terraphim_sessions
2. terraphim_hooks
3. terraphim_update
4. terraphim_negative_contribution
5. terraphim_command_runtime
6. terraphim_grep
7. terraphim_mcp_server
8. terraphim_agent
9. terraphim_cli
10. terraphim_lsp
```

## Common Problems and Solutions

### "registry index was not found in any configuration: terraphim"

**What it means**: A `publish = ["terraphim"]` restriction wasn't fully removed.

**How to fix**: Check that the sed pattern in the pipeline script deletes the entire line, not just replaces the directive. The old pattern `s/publish = ["terraphim"]//g` left comments that cargo still interpreted.

**Current fix**: `sed -i '/publish = ["terraphim"]/d'` (deletes the whole line)

### "keywords must have less than 20 characters"

**What it means**: A keyword in `Cargo.toml` is too long for crates.io.

**Example**: `"model-context-protocol"` is 21 characters.

**How to fix**: Shorten or remove the keyword. Good alternatives: `"mcp"`, `"protocol"`.

### "no method named 'X' found"

**What it means**: The published crate version doesn't match what downstream expects.

**How to fix**: Bump the version number in the upstream crate and republish. For example, if `terraphim_spawner v1.20.3` is missing a method, bump to `v1.20.4`.

### "no matching package named 'X' found"

**What it means**: A dependency crate hasn't been published yet.

**How to fix**: The publish list is in the wrong order. Update `polyrepo-publish.sh` to put dependencies before consumers.

### "429 Too Many Requests"

**What it means**: crates.io rate-limits new crate publishes. Publishing many crates quickly triggers this.

**How to fix**: Wait 5-10 minutes and retry. The pipeline is idempotent -- already-published crates are skipped automatically.

**Tip**: You can trigger just the remaining crates:
```bash
gh workflow run publish-crates.yml -R terraphim/<repo> --ref main \
  -f crate_list="terraphim_agent terraphim_cli" \
  -f dry_run="false"
```

### "Not allowed to push to protected branch main"

**What it means**: Branch protection prevents direct pushes.

**How to fix**: The pipeline handles this automatically by creating a PR and temporarily disabling status checks for the merge.

## Dry Run (Test Without Publishing)

To test the entire pipeline without actually publishing:

```bash
export POLYREPO_DRY_RUN=1
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh full terraphim-service
```

This runs all steps except the actual push and publish.

## Running Individual Steps

Sometimes you need to run just one step:

```bash
# Just clone and scan
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh clone terraphim-service
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh scrub terraphim-service

# Just run the Gitea CI gate
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh prepare-gitea-branch terraphim-service
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh wait-gitea-ci terraphim-service

# Just publish crates (after CI passed)
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh crates-publish terraphim-service
```

## Monitoring a Running Pipeline

The pipeline runs in the foreground and prints progress:

```
[2026-06-09T10:30:06] [terraphim-service] Waiting for GitHub Actions CI
[2026-06-09T10:30:37] [terraphim-service]   GitHub CI pending (30s / 1800s)
[2026-06-09T10:32:00] [terraphim-service] GitHub CI PASSED
[2026-06-09T10:32:05] [terraphim-service] Dispatching GitHub Publish Crates workflow
```

You can also check GitHub Actions directly:
```bash
gh run list -R terraphim/terraphim-service --limit 5
```

## After Publishing

### Verify crates are live
```bash
cargo search terraphim_service --limit 5
```

### Check both remotes are in sync
```bash
git fetch origin && git fetch gitea
git diff origin/main gitea/main --stat  # Should be empty
```

### Update the monorepo
If you published a new version of a crate that the monorepo uses, update the version in the monorepo's `Cargo.toml`.

## Tips and Best Practices

**Start with dependency repos first.** Always publish upstream repos before downstream repos. For example:
1. terraphim-core (types, basic crates)
2. terraphim-config-persistence (depends on core)
3. terraphim-service (depends on persistence)
4. terraphim-agents (depends on service)
5. terraphim-kg-agents (depends on agents)
6. terraphim-clients (depends on everything)

**Check keywords before publishing.** Run this to find long keywords:
```bash
grep -r 'keywords = ' /tmp/polyrepo-publish/<repo>/crates/*/Cargo.toml
```

**Use tmux for long runs.** The pipeline can take 10-15 minutes:
```bash
tmux new-session -d -s polyrepo \
  "bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh full terraphim-service"
tmux attach -t polyrepo
```

**If a publish fails mid-way, retry with remaining crates.** The workflow skips already-published crates automatically.

## Environment Variables Reference

| Variable | Required | What It Does |
|----------|----------|--------------|
| `GITEA_TOKEN` | Yes | Authenticates with our private Gitea instance |
| `GITHUB_TOKEN` | Yes | Used by `gh` CLI (run `gh auth status` to verify) |
| `CARGO_REGISTRY_TOKEN` | Yes | Authenticates with crates.io for publishing |
| `POLYREPO_DRY_RUN` | No | Set to `1` to test without publishing |
| `POLYREPO_WORK_DIR` | No | Where repos are cloned (default: `/tmp/polyrepo-publish`) |

## Troubleshooting Checklist

Before running the pipeline, verify:

- [ ] `GITEA_TOKEN` is exported in your shell
- [ ] `gh auth status` shows you are logged in to github.com
- [ ] `CARGO_REGISTRY_TOKEN` secret exists on the GitHub repo (`gh secret list -R terraphim/<repo>`)
- [ ] No crate keywords exceed 20 characters
- [ ] Dependency order is correct in `polyrepo-publish.sh`
- [ ] You have push access to both Gitea and GitHub repos

## Getting Help

- **Pipeline script**: `scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh`
- **Architecture details**: `docs/architecture/adr/0002-polyrepo-github-publish-pipeline.md`
- **Tracking issue**: Gitea issue #2260
- **crates.io rate limits**: https://crates.io/docs/rate-limits
