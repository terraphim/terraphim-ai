# Build System Documentation

Part of Epic #1423: Fast/cheap LLM build-runner.

## Overview

The terraphim-ai project uses an adaptive build system (`build-runner-llm`) that automatically detects CI configuration and transforms build commands using the DevOpsRunner knowledge graph.

## Architecture

```
Push Event → build-runner-llm → detect CI config → extract commands → transform via KG → validate → execute → post status
```

**Key principle:** KG-first architecture. Commands are matched via Aho-Corasick automata (0.1s latency) rather than LLM extraction, keeping costs below $0.01 per build.

## Build Command Detection

The build-runner detects CI configuration in this priority order:

1. **GitHub Actions** (`.github/workflows/*.yml`) - Extracts `run` steps from all workflows that trigger on push/pull_request
2. **BUILD.md** - Project-specific build documentation with bash code blocks
3. **Cargo workspace** (`Cargo.toml`) - Standard Rust commands
4. **Makefile** - Runs `make`
5. **Earthfile** - Extracts `RUN` lines containing cargo/build/test
6. **package.json** - Node.js projects (bun install/build/test)

### GitHub Actions Integration

The build-runner automatically discovers and executes commands from all GitHub Actions workflows that trigger on `push` or `pull_request` events. This means your existing CI configuration is leveraged directly:

- No need to duplicate build commands
- Uses the same commands as GitHub Actions
- Extracts `run` steps from all jobs

Example: If `.github/workflows/ci-pr.yml` contains:
```yaml
jobs:
  build:
    steps:
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace -- -D warnings
      - run: cargo test --workspace
```

The build-runner will execute these exact commands locally.

### Default Rust Build Sequence

When `Cargo.toml` is detected:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --profile ci
cargo test --workspace --no-fail-fast --profile ci
```

## Command Transformation

The DevOpsRunner knowledge graph (`~/.config/terraphim/docs/src/kg/devops/`) provides semantic command transformations:

| Original Command | Transformed Command | Context |
|-----------------|---------------------|---------|
| `cargo build` | `rch build` | Remote compilation |
| `npm install` | `bun install` | Package manager |
| `npm run build` | `bun run build` | Build step |
| `npm test` | `bun test` | Test runner |

Transformations are applied via `terraphim-agent replace --role DevOpsRunner`.

## Cost Tracking

Every build tracks costs automatically:

- **KG lookup:** $0.0001 per command transformation
- **LLM call:** $0.005 (only used for cold-start extraction)
- **Warning threshold:** $0.01
- **Fail threshold:** $0.05

Cost metrics are sent to Quickwit telemetry:

```json
{
  "timestamp": "2026-05-11T16:30:00Z",
  "agent": "build-runner-llm",
  "project": "terraphim-ai",
  "sha": "abc123",
  "cost_cents": 0.0001,
  "kg_lookups": 4,
  "llm_calls": 0,
  "status": "success"
}
```

## Command Validation

All commands are validated against a whitelist before execution:

**Allowed:** cargo, make, npm, yarn, pnpm, bun, docker, yq, test, echo, cat, ls, cd, mkdir, rm, cp, mv, git, curl, wget, tar, unzip, zip, chmod, chown, source, export, eval

**Rejected patterns:**
- `sudo`
- `curl ... | sh` or `wget ... | sh`
- `rm -rf /`
- Direct device writes (`> /dev/sd*`)
- Disk formatting (`mkfs.*`, `dd if=...of=...`)

## Triggering Builds

Builds are triggered automatically on push events to Gitea:

1. Developer pushes commit
2. Gitea webhook fires
3. ADF orchestrator receives push event
4. `build-runner` agent spawned with `ADF_PUSH_*` environment variables
5. Build executes and posts `adf/build` commit status

### Manual Trigger

To manually trigger a build for testing:

```bash
export ADF_PUSH_SHA=<commit-sha>
export ADF_PUSH_REF=refs/heads/<branch>
export ADF_WORKING_DIR=/path/to/repo
export GITEA_OWNER=terraphim
export GITEA_REPO=terraphim-ai
export GITEA_TOKEN=<token>
export GITEA_URL=https://git.terraphim.cloud

bash scripts/build-runner-llm.sh
```

## Monitoring

Check build status via Gitea commit status API:

```bash
curl -H "Authorization: token $GITEA_TOKEN" \
  "$GITEA_URL/api/v1/repos/terraphim/terraphim-ai/commits/<sha>/statuses"
```

## Rollback

If the adaptive build-runner causes issues, restore the deterministic build-runner:

```bash
ssh bigbox
sudo systemctl stop adf-orchestrator
# Restore from git history
git -C /opt/ai-dark-factory/conf.d checkout HEAD -- terraphim.toml
sudo systemctl start adf-orchestrator
```

## Adding New Build Commands

To add new build command transformations:

1. Create a new file in `~/.config/terraphim/docs/src/kg/devops/`
2. Use the format:

```markdown
# command name

Description of the command.

synonyms:: alt1, alt2, alt3
related:: other-command, another-command
transforms:: old-command → new-command
context:: build
cost:: low|medium|high
```

3. Reload terraphim-agent config: `terraphim-agent config reload`
4. Test: `terraphim-agent search --role DevOpsRunner "your command"`

## Files

- `scripts/build-runner-llm.sh` - Build runner implementation
- `~/.config/terraphim/docs/src/kg/devops/` - Build ontology knowledge graph
- `crates/terraphim_orchestrator/tests/fixtures/conf.d/terraphim.toml` - Agent definition

## References

- Epic #1423: Fast/cheap LLM build-runner
- ADR-001: Build-runner architecture decisions
- `.docs/research-fast-cheap-build-runner.md`
- `.docs/design-build-runner-llm.md`
