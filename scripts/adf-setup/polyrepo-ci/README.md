# Interim CI for the #1910 polyrepos (Part A)

`deploy-interim-ci.sh` wires the 6 new Gitea polyrepos
(terraphim-core, -config-persistence, -service, -agents, -kg-agents, -clients)
into the existing ADF orchestrator so each push/PR gets an `adf/build` commit
status, reusing rch + sccache (no act_runner).

It is **additive**: it only adds `conf.d/<repo>.toml`, clones, and Gitea webhooks
— it never edits `terraphim.toml` or the orchestrator binary. Mirrors the proven
`better-auth-rust.toml` pattern. Each build-runner runs `build-runner-llm.sh`
against the repo's own `BUILD.md` (the single command source shared with the
future native runner), so `cargo build/clippy/test` are transformed to
`rch exec` (sccache→SeaweedFS) and `cargo fmt` stays on host.

## Deploy (on bigbox, after review)
```bash
export GITEA_TOKEN=...           # bigbox gitea token (as used by other conf.d projects)
export ADF_WEBHOOK_SECRET=...    # orchestrator HMAC secret (op read)
bash scripts/adf-setup/polyrepo-ci/deploy-interim-ci.sh
sudo systemctl reload adf-orchestrator
```
Then push a no-op commit per repo and confirm `adf/build` goes pending→success.

## Cutover to the native runner
When `terraphim_gitea_runner` is proven, set `active_lane = "native"` for a repo
and add it to the runner's `active_repos`; swap branch protection from `adf/build`
to `terraphim-native/build`. Commands are unchanged (same `BUILD.md`).
