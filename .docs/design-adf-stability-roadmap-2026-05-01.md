# Design & Implementation Plan: ADF Stability Roadmap

## 1. Summary of Target Behaviour

After implementation, the ADF will:
- Build, review, and merge PRs automatically via the orchestrator
- Pass `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` on every PR
- Have no confidence-score injection vulnerabilities in any agent script
- Have a functional pr-compliance-watchdog agent
- Have running config that matches git-tracked config
- Have file permissions that prevent secret exposure

## 2. Key Invariants and Acceptance Criteria

| ID | Invariant | Acceptance Criteria |
|----|-----------|---------------------|
| AC-1 | No agent has confidence-score injection | `grep -c 'head -1' scripts/adf-setup/agents/pr-reviewer.toml` returns 0 |
| AC-2 | All agents sanitise ADF_PR_NUMBER | All 5 agents contain `tr -cd '0-9'` after the empty check |
| AC-3 | All agents guard ADF_PR_HEAD_SHA | All 5 agents check all 3 env vars |
| AC-4 | pr-compliance-watchdog is valid TOML | `python3 -c "import tomllib; tomllib.load(open('pr-compliance-watchdog.toml','rb'))"` succeeds |
| AC-5 | CI is green | `cargo clippy --workspace --all-targets -- -D warnings` exits 0 |
| AC-6 | Tests pass | `cargo test --workspace` exits 0 |
| AC-7 | Config reconciled | `diff <(git show main:scripts/.../orchestrator.toml) <(ssh bigbox cat /opt/.../orchestrator.toml)` shows only intentional differences |
| AC-8 | File permissions secure | All .toml files in /opt/ai-dark-factory/ are mode 600 |
| AC-9 | Backup files cleaned | No .bak files in conf.d/ |
| AC-10 | Reconciler observed working | At least one remediation issue opened by tick 40 |

## 3. High-Level Design and Boundaries

### 6 Parallel Streams

```
Stream A: Security Hardening     Stream B: CI Restoration     Stream C: Config Reconciliation
(agent TOML fixes)               (clippy + test fixes)        (git <-> bigbox sync)
         |                                |                            |
         v                                v                            v
Stream D: Branch Protection       Stream E: Watchdog Recovery  Stream F: Permissions & Cleanup
(status posting fix)              (TOML rewrite)               (chmod + cargo clean)
```

**Stream boundaries:**
- A, B, E, F: No dependencies, start immediately
- C: No dependencies but should be done in one coordinated pass
- D: Requires A (agents must post correct statuses)

### Changes Inside Existing Components

| Component | Change Type | Stream |
|-----------|------------|--------|
| `scripts/adf-setup/agents/pr-reviewer.toml` | Modify (security fix) | A |
| `scripts/adf-setup/agents/pr-spec-validator.toml` | Modify (security fix) | A |
| `scripts/adf-setup/agents/pr-security-sentinel.toml` | Modify (security fix) | A |
| `scripts/adf-setup/agents/pr-test-guardian.toml` | Modify (env var guards) | A |
| `scripts/adf-setup/agents/pr-compliance-watchdog.toml` | Rewrite | E |
| `crates/terraphim_orchestrator/tests/*.rs` (4 files) | Modify (add field) | B |
| `crates/terraphim_multi_agent/src/agent.rs` | Modify (prefix unused var) | B |
| `crates/terraphim_agent/tests/offline_mode_tests.rs` | Modify (exit codes) | B |
| `crates/terraphim_agent/tests/exit_codes_integration_test.rs` | Modify (listen validation) | B |
| `crates/terraphim_agent/tests/integration_tests.rs` | Modify (schema fix) | B |
| `crates/terraphim_agent/src/client.rs` or `terraphim_server/src/api.rs` | Modify (ThesaurusResponse) | B |
| `scripts/adf-setup/scripts/adf-setup/orchestrator.toml` | Modify (sync to running) | C |
| `/opt/ai-dark-factory/conf.d/*.toml` | Permissions change | F |

### New Components

None -- all changes are modifications to existing files.

## 4. File/Module-Level Change Plan

### Stream A: Security Hardening (5 files)

| File | Action | Change | Dependencies |
|------|--------|--------|--------------|
| `pr-reviewer.toml` | Modify | Replace `2>&1` with `--output-format text 2>/dev/null`, parse only `REVIEW_FILE`, use `tail -1` instead of `head -1`, add ADF_PR_NUMBER sanitisation, add ADF_PR_HEAD_SHA guard | None |
| `pr-spec-validator.toml` | Modify | Replace `head -1` with `tail -1`, add ADF_PR_NUMBER sanitisation, add ADF_PR_HEAD_SHA guard | None |
| `pr-security-sentinel.toml` | Modify | Replace `head -1` with `tail -1`, add ADF_PR_NUMBER sanitisation, add ADF_PR_HEAD_SHA guard | None |
| `pr-test-guardian.toml` | Modify | Add ADF_PR_NUMBER sanitisation, fix `HEAD_SHORT` env export bug | None |
| `pr-compliance-watchdog.toml` | Rewrite | Extract TOML from double-nested diff OR rewrite from pr-spec-validator template with compliance-specific prompt | None |

### Stream B: CI Restoration (7-8 files)

| File | Action | Change | Dependencies |
|------|--------|--------|--------------|
| `tests/orchestrator_tests.rs` | Modify | Add `gate_reconcile_interval_ticks: 20,` to `test_config()` | None |
| `tests/pause_and_breaker_tests.rs` | Modify | Add `gate_reconcile_interval_ticks: 20,` to `test_config()` | None |
| `tests/auto_merge_tests.rs` | Modify | Add `gate_reconcile_interval_ticks: 20,` to `test_config()` | None |
| `tests/auto_merge_execution_tests.rs` | Modify | Add `gate_reconcile_interval_ticks: 20,` to `test_config()` | None |
| `terraphim_multi_agent/src/agent.rs:1348` | Modify | Rename `agent` to `_agent` in `test_hook_manager_initialized()` | None |
| `terraphim_agent/tests/offline_mode_tests.rs` | Modify | Fix exit code assertions (3 for missing index, 6 for network error) | None |
| `terraphim_agent/tests/exit_codes_integration_test.rs` | Modify | Implement `--server` flag rejection in `listen` subcommand OR update test expectation | None |
| `terraphim_agent/tests/integration_tests.rs` + `client.rs` | Modify | Align ThesaurusResponse schemas between client and server | None |

### Stream C: Config Reconciliation (2 files)

| File | Action | Change | Dependencies |
|------|--------|--------|--------------|
| `scripts/adf-setup/scripts/adf-setup/orchestrator.toml` | Modify | Add `[mentions]`, `[pr_dispatch]`, `gate_reconcile_interval_ticks`, update `tick_interval_secs` to 300, `probe_ttl_secs` to 1800 | None |
| `/etc/systemd/system/adf-orchestrator.service` | Modify | Update `MemoryMax` to 64G, commit to git | None |

### Stream D: Branch Protection Fix (1 file)

| File | Action | Change | Dependencies |
|------|--------|--------|--------------|
| `scripts/adf-setup/agents/build-runner.toml` | Modify | Remove `event_only = true` or add PR-open dispatch logic | Requires A |

### Stream F: Permissions & Cleanup (bigbox only)

| File | Action | Change | Dependencies |
|------|--------|--------|--------------|
| `/opt/ai-dark-factory/*.toml` | Permission change | `chmod 600` on all .toml files | None |
| `/opt/ai-dark-factory/conf.d/*.bak*` | Delete | Remove 17 backup files | None |

## 5. Step-by-Step Implementation Sequence

### Stream A: Security Hardening (parallel agents can work simultaneously)

**Step A1**: Fix pr-reviewer.toml confidence-score injection
- Replace `2>&1` with `--output-format text 2>/dev/null`
- Remove `SCORE_TEXT="$SCORE_TEXT\n$REVIEW_OUTPUT"` concatenation
- Parse only `REVIEW_FILE` content for score
- Replace `head -1` with `tail -1`
- Add `ADF_PR_NUMBER=$(printf '%s' "$ADF_PR_NUMBER" | tr -cd '0-9')` with empty guard
- Add `ADF_PR_HEAD_SHA` to env var guard
- System deployable: yes (TOML is only read on next PR dispatch)

**Step A2**: Fix pr-spec-validator.toml
- Replace `head -1` with `tail -1`
- Add ADF_PR_NUMBER sanitisation
- Add ADF_PR_HEAD_SHA guard
- System deployable: yes

**Step A3**: Fix pr-security-sentinel.toml
- Replace `head -1` with `tail -1`
- Add ADF_PR_NUMBER sanitisation
- Add ADF_PR_HEAD_SHA guard
- System deployable: yes

**Step A4**: Fix pr-test-guardian.toml
- Add ADF_PR_NUMBER sanitisation
- Export `HEAD_SHORT` or use `ADF_PR_HEAD_SHA` directly in Python subprocess
- System deployable: yes

**Step A5**: Recover pr-compliance-watchdog.toml
- Attempt to extract TOML from double-nested diff
- If extraction fails, rewrite from pr-spec-validator template
- Add same sanitisation/guarding pattern
- System deployable: yes

**Step A6**: Deploy all 5 TOMLs to bigbox
- Copy to `/opt/ai-dark-factory/conf.d/` (if that's where agents read from)
- OR run `migrate-to-confd.py` to deploy all 15 agents
- Restart ADF orchestrator

### Stream B: CI Restoration (parallel)

**Step B1**: Fix 4 orchestrator test files (add missing field)
- Add `gate_reconcile_interval_ticks: 20,` to each `test_config()` function
- Verify: `cargo test -p terraphim_orchestrator --no-run`

**Step B2**: Fix unused variable in multi_agent test
- Rename `agent` to `_agent` at `agent.rs:1348`
- Verify: `cargo clippy --workspace --all-targets -- -D warnings`

**Step B3**: Fix offline_mode_tests exit codes
- Update assertions to match `ExitCode` enum: 3 (ErrorIndexMissing), 6 (ErrorNetwork)
- Verify: `cargo test -p terraphim_agent --test offline_mode_tests`

**Step B4**: Fix ThesaurusResponse schema
- Option (a): Update `client.rs` to match server response (`thesaurus` field)
- Option (b): Update server `api.rs` to return `terms` field
- Recommendation: Option (b) -- server should return the same schema the client expects
- Verify: `cargo test -p terraphim_agent --test integration_tests`

**Step B5**: Fix listen_mode_with_server_flag test
- Either: implement `--server` rejection in the `listen` subcommand
- Or: update test to reflect current behaviour
- Recommendation: implement the rejection (3-line change in clap)
- Verify: `cargo test -p terraphim_agent --test exit_codes_integration_test`

**Step B6**: Fix test_full_feature_matrix offline path
- Ensure Default role has a knowledge graph configured in test fixtures, or skip when unavailable
- Verify: `cargo test -p terraphim_agent --test integration_tests test_full_feature_matrix`

**Step B7**: Full validation
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo fmt --check`

### Stream C: Config Reconciliation (sequential)

**Step C1**: Update git-tracked orchestrator.toml to match running values
- Set `tick_interval_secs = 300`
- Set `probe_ttl_secs = 1800`
- Add `[mentions]` section
- Add `[pr_dispatch]` section
- Add `gate_reconcile_interval_ticks = 20`
- Commit with message: `chore(adf): reconcile config with running values Refs #1118`

**Step C2**: Update git-tracked service file
- Set `MemoryMax=64G`
- Commit with message: `chore(adf): update MemoryMax to 64G Refs #1118`

**Step C3**: Clean backup files on bigbox
- `rm /opt/ai-dark-factory/conf.d/*.bak*`
- Verify no drift between git and running

### Stream D: Branch Protection Fix (after A completes)

**Step D1**: Fix build-runner to dispatch on PR open events
- Either: remove `event_only = true` from build-runner.toml
- Or: add a webhook handler that dispatches build-runner when a PR is opened/reopened
- Deploy updated build-runner.toml

**Step D2**: Verify pr-reviewer posts commit status
- Ensure pr-reviewer.toml posts `adf/pr-reviewer` status on every PR review (pass or fail)
- The orchestrator's Phase 5 `post_terminal_commit_status()` should handle this for the build context

**Step D3**: Observe status checks appearing on an open PR
- Wait for next tick cycle or trigger manually
- Verify `adf/build` and `adf/pr-reviewer` appear in commit statuses

### Stream F: Permissions & Cleanup (parallel, 5 minutes)

**Step F1**: Fix file permissions on bigbox
```bash
ssh bigbox "sudo chmod 600 /opt/ai-dark-factory/orchestrator.toml /opt/ai-dark-factory/conf.d/*.toml"
```

**Step F2**: Clean backup files
```bash
ssh bigbox "rm /opt/ai-dark-factory/conf.d/*.bak* /opt/ai-dark-factory/conf.d.backup-* -rf"
```

**Step F3**: Verify permissions
```bash
ssh bigbox "ls -la /opt/ai-dark-factory/*.toml /opt/ai-dark-factory/conf.d/*.toml"
```

## 6. Testing & Verification Strategy

| AC | Test Type | Test Command / Location |
|----|-----------|------------------------|
| AC-1 | Grep assertion | `grep -c 'head -1' scripts/adf-setup/agents/pr-reviewer.toml` == 0 |
| AC-1 | Grep assertion | `grep -c 'tail -1' scripts/adf-setup/agents/pr-reviewer.toml` >= 1 |
| AC-1 | Grep assertion | `grep -c 'REVIEW_OUTPUT' scripts/adf-setup/agents/pr-reviewer.toml` after SCORE_TEXT block == 0 |
| AC-2 | Grep assertion | `grep -l 'tr -cd' scripts/adf-setup/agents/*.toml` lists all 5 files |
| AC-3 | Grep assertion | `grep -l 'ADF_PR_HEAD_SHA' scripts/adf-setup/agents/*.toml` lists all 5 files |
| AC-4 | Unit test | `python3 -c "import tomllib; tomllib.load(open('pr-compliance-watchdog.toml','rb'))"` |
| AC-5 | CI gate | `cargo clippy --workspace --all-targets -- -D warnings` |
| AC-6 | CI gate | `cargo test --workspace` |
| AC-7 | Integration | `diff <(git show main:path) <(ssh bigbox cat path)` |
| AC-8 | Integration | `ssh bigbox stat -c '%a' /opt/ai-dark-factory/orchestrator.toml` == 600 |
| AC-9 | Integration | `ssh bigbox ls /opt/ai-dark-factory/conf.d/*.bak* 2>&1` == "No such file" |
| AC-10 | Observation | Gitea issue opened by reconciler within 2 hours |

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| Agent TOML change breaks PR review pipeline | Test locally with `bash -n`, deploy one agent at a time | Edge case in bash quoting |
| ThesaurusResponse fix breaks existing clients | Check all callers of server endpoint | Client version mismatch |
| Config reconciliation restarts ADF during active work | Deploy during low activity, keep backup | Tick in progress may be interrupted |
| Reconciler opens flood of remediation issues | Dedup via `remediation_key()`, limit to 1 per PR per tick | Initial burst after first run |
| pr-compliance-watchdog extraction fails | Rewrite from template (Step A5 fallback) | Compliance rules may differ |

## 8. Open Questions / Decisions for Human Review

1. **ThesaurusResponse fix direction**: Fix server to match client (option b), or fix client to match server (option a)? Server change risks breaking other consumers; client change is safer.

2. **listen --server rejection**: Implement the rejection in clap (adds validation), or update the test to match current permissive behaviour?

3. **build-runner dispatch trigger**: Remove `event_only = true` (runs on every tick that has a PR), or add webhook-based dispatch (more precise but more complex)?

4. **Config reconciliation timing**: Should Stream C be committed before or after Stream B? Before means CI tests the reconciled config; after means CI is green first.

5. **pr-compliance-watchdog recovery**: Extract from diff (preserves exact original) or rewrite from template (cleaner but may miss compliance-specific logic)?
