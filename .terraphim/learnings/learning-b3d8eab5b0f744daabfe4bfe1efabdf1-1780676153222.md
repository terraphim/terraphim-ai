---
id: b3d8eab5b0f744daabfe4bfe1efabdf1-1780676153222
command: export POLYREPO_DRY_RUN=0
export POLYREPO_WORK_DIR=/tmp/polyrepo-publish-prod
export POLYREPO_PUBLISH_MODE=dependency
rm -rf "$POLYREPO_WORK_DIR"
mkdir -p "$POLYREPO_WORK_DIR"

echo "Starting PRODUCTION run for terraphim-config-persistence at $(date -Iseconds)"
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh full terraphim-config-persistence 2>&1
exit_code: 1
source: Project
captured_at: 2026-06-05T16:15:53.222171665+00:00
working_dir: /home/alex/projects/terraphim/terraphim-ai
tags:
  - learning
  - exit-1
entities:
  - database
importance_total: 0.3800
importance_severity: 0.3000
importance_repetition: 3
importance_recency: 1.0000
importance_has_correction: false
---

## Command

`export POLYREPO_DRY_RUN=0
export POLYREPO_WORK_DIR=/tmp/polyrepo-publish-prod
export POLYREPO_PUBLISH_MODE=dependency
rm -rf "$POLYREPO_WORK_DIR"
mkdir -p "$POLYREPO_WORK_DIR"

echo "Starting PRODUCTION run for terraphim-config-persistence at $(date -Iseconds)"
bash scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh full terraphim-config-persistence 2>&1`

## Error Output

```
Starting PRODUCTION run for terraphim-config-persistence at 2026-06-05T17:15:21+01:00
[2026-06-05T17:15:21+01:00] [terraphim-config-persistence] Cloning terraphim/terraphim-config-persistence from Gitea
Cloning into '/tmp/polyrepo-publish-prod/terraphim-config-persistence'...
[2026-06-05T17:15:22+01:00] [terraphim-config-persistence] Cloned at 5a9470dd3a3f
5a9470d Merge pull request 'feat(adf): repo-local implementation-swarm (#2203)' (#3) from task/2203-repo-local-impl-swarm into main
5d295a6 feat(adf): repo-local implementation-swarm (#2203 rollout)
de3130f Merge pull request 'ci(#2080): add native-ci workflow (terraphim-native cutover)' (#2) from cutover/2080-native-ci into main
[2026-06-05T17:15:22+01:00] [terraphim-config-persistence] Scrubbing secrets and internal references
[2026-06-05T17:15:22+01:00] [terraphim-config-persistence] Secret scan passed
[2026-06-05T17:15:22+01:00] [terraphim-config-persistence] Rewriting Cargo.toml for public consumption
[2026-06-05T17:15:22+01:00] [terraphim-config-persistence]   Stripping registry refs from ./crates/terraphim_settings/Cargo.toml
[2026-06-05T17:15:22+01:00] [terraphim-config-persistence]   Stripping registry refs from ./crates/terraphim_config/Cargo.toml
[2026-06-05T17:15:22+01:00] [terraphim-config-persistence]   Stripping registry refs from ./crates/terraphim_persistence/Cargo.toml
[2026-06-05T17:15:22+01:00] [terraphim-config-persistence]   Removing Gitea registry from .cargo/config.toml
 .cargo/config.toml                      |  2 --
 crates/terraphim_config/Cargo.toml      | 12 ++++++------
 crates/terraphim_persistence/Cargo.toml |  4 ++--
 crates/terraphim_settings/Cargo.toml    |  2 +-
 4 files changed, 9 insertions(+), 11 deletions(-)
Switched to a new branch 'publish/github-mirror'
M	.cargo/config.toml
M	crates/terraphim_config/Cargo.toml
M	crates/terraphim_persistence/Cargo.toml
M	crates/terraphim_settings/Cargo.toml
[publish/github-mirror bba63dc] chore: prepare for GitHub public mirror
 6 files ch
```

