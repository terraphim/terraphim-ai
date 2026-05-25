# Incomplete Handoff: Fix #1851 - Shell injection in adf-ctl.rs

**Date**: 2026-05-25
**Agent**: pi session
**Status**: MOSTLY COMPLETE — needs `cargo clippy`, `cargo test`, and PR creation

## What's Done ✅

1. **Research complete**: Issue #1851 is a P1 CWE-78 shell injection in `adf-ctl.rs`
   - Vulnerable code is on branch `task/1326-webhook-secret-env-var` (commit d7613339), NOT on main
   - Fix branch `task/1851-shell-injection-adf-ctl` was created from that branch
   
2. **Implementation complete** (committed and pushed):
   - Added `is_valid_env_var_name()` to `crates/terraphim_orchestrator/src/config.rs` — validates `[A-Za-z_][A-Za-z_0-9]*`
   - Hardened `resolve_secret()` in `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` — validates var_name before constructing SSH command
   - Replaced `bash -lc 'echo ${}'` with `printenv {}` (no shell expansion)
   - Hardened `validate_webhook_secret()` in config.rs — rejects env-var refs with invalid names at config-parse time (defense in depth)
   - Added 5 tests in config.rs for injection rejection
   - Added 4 tests in adf-ctl.rs for `is_valid_env_var_name`
   
3. **`cargo check` passes** ✅

4. **Pushed to origin**: `task/1851-shell-injection-adf-ctl`

## What Remains ⏳

1. **Run quality gates**:
   ```bash
   cd /data/projects/terraphim/terraphim-ai
   git checkout task/1851-shell-injection-adf-ctl
   cargo clippy -p terraphim_orchestrator -- -D warnings
   cargo fmt --all -- --check
   cargo test -p terraphim_orchestrator --bin adf-ctl
   cargo test -p terraphim_orchestrator --lib config  # the config.rs tests
   ```

2. **Fix any clippy/test failures** (likely minor)

3. **Create PR on Gitea**:
   ```bash
   export GITEA_URL=https://git.terraphim.cloud && source ~/.profile
   /home/alex/go/bin/gitea-robot create-pull \
     --owner terraphim --repo terraphim-ai \
     --title "Fix #1851: prevent shell injection in adf-ctl env-var resolution" \
     --base task/1326-webhook-secret-env-var \
     --head task/1851-shell-injection-adf-ctl
   ```
   Note: PR base should be `task/1326-webhook-secret-env-var` (the branch containing the vulnerable code), NOT `main`.

4. **Post comment on issue #1851**:
   ```bash
   /home/alex/go/bin/gitea-robot comment --owner terraphim --repo terraphim-ai --index 1851 \
     --body "Implementation complete. PR targets task/1326-webhook-secret-env-var. Branch: task/1851-shell-injection-adf-ctl"
   ```

5. **Push to gitea remote** (after origin succeeds):
   ```bash
   git push gitea task/1851-shell-injection-adf-ctl
   ```

## Next-Agent Starting Position

```bash
cd /data/projects/terraphim/terraphim-ai
git fetch origin
git checkout task/1851-shell-injection-adf-ctl
# Then run quality gates above, create PR, done.
```

## Key Context

- The vulnerable code was introduced in commit d7613339 on branch `task/1326-webhook-secret-env-var`
- That branch has NOT been merged to main yet
- This fix branch is based on that branch, so the fix will be a PR against `task/1326-webhook-secret-env-var`
- Once both are merged (1326 first, then 1851 squash into 1326, or 1851 into main after 1326 merges), the vulnerability is eliminated
