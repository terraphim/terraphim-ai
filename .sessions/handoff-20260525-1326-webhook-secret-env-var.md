# Incomplete Handoff: Issue #1326 — Webhook secret hardcoded in orchestrator.toml

**Agent**: Echo (Twin Maintainer)
**Branch**: `task/1326-webhook-secret-env-var`
**Commit**: `1a0b1bc7`
**Status**: PARTIAL — core logic complete, 2 files remain

---

## What Is Done

1. **Code guardrail (`config.rs`)**
   - `validate_webhook_secret()` added — rejects hex strings >= 32 chars, allows `${VAR}` references
   - Wired into `OrchestratorConfig::validate()` — orchestrator now refuses to start with a hardcoded secret
   - 8 unit tests added and passing:
     - `webhook_secret_hardcoded_hex_rejected`
     - `webhook_secret_env_var_allowed`
     - `webhook_secret_passphrase_allowed`
     - `webhook_secret_short_hex_allowed`
     - `webhook_secret_31_char_hex_allowed`
     - `webhook_secret_32_char_hex_rejected`
     - `test_validate_webhook_secret_in_config`
     - `test_validate_webhook_env_var_in_config_passes`

2. **adf-ctl fix (`src/bin/adf-ctl.rs`)**
   - `resolve_secret()` SSH fallback now detects `${VAR}` references in remote config and resolves the actual value from the remote shell environment via `bash -lc 'echo $VAR'`
   - Prevents adf-ctl from using the literal string `${ADF_WEBHOOK_SECRET}` as the HMAC key
   - All 16 adf-ctl tests pass

3. **Quality gates**
   - `cargo test -p terraphim_orchestrator --lib webhook_secret` → 7/7 pass
   - `cargo test -p terraphim_orchestrator --bin adf-ctl` → 16/16 pass
   - `cargo check -p terraphim_orchestrator --bin adf-ctl` → clean

---

## What Remains

| # | Task | File | Effort |
|---|------|------|--------|
| 1 | Add `[webhook]` section to example config with env-var syntax and security comment | `crates/terraphim_orchestrator/orchestrator.example.toml` | 5 min |
| 2 | Add `ADF_WEBHOOK_SECRET` documentation comment to systemd service file | `adf-orchestrator.service` | 2 min |
| 3 | Run `cargo fmt --all -- --check` and `cargo clippy -p terraphim_orchestrator -- -D warnings` | workspace | 5 min |
| 4 | Push branch: `git push -u origin task/1326-webhook-secret-env-var` | git | 1 min |
| 5 | Create PR via `gtr create-pull` | gitea-robot | 2 min |
| 6 | Post comment on issue #1326 referencing PR | gitea-robot | 1 min |

---

## Next-Agent Starting Position

```bash
cd /data/projects/terraphim/terraphim-ai
git checkout task/1326-webhook-secret-env-var
```

The branch is clean, committed, and ahead of `origin/main` by `1a0b1bc7`. Finish the two template/documentation edits, run the remaining quality gates, push, and open the PR.

---

## Operational Runbook (for merge-coordinator)

After this PR merges, the deployed file `/opt/ai-dark-factory/orchestrator.toml` must be updated manually:

1. Generate new secret: `openssl rand -hex 32`
2. Store in 1Password vault `TerraphimPlatform`
3. Update `orchestrator.toml` line 49: `secret = "${ADF_WEBHOOK_SECRET}"`
4. Export `ADF_WEBHOOK_SECRET` in `~/.profile` or systemd service env
5. Update Gitea webhook settings at `https://git.terraphim.cloud/terraphim/terraphim-ai/settings/hooks`
6. Restart orchestrator: `sudo systemctl restart adf-orchestrator`
7. Verify webhook delivery in Gitea admin panel
