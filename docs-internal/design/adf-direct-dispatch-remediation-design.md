# ADF Direct Dispatch Remediation -- Design Document

Gitea issue: `terraphim/terraphim-ai#1890`
Pull request: `terraphim/terraphim-ai#1885`
Phase: 2 of disciplined development (Design)
Author: OpenCode
Date: 2026-05-29

This document records the research summary and implementation plan for fixing
the structured PR review findings against the ADF direct-dispatch remediation
branch. No implementation is included in this document.

---

## 1. Research Summary

### 1.1 Problem

PR #1885 has three review findings:

1. `adf-ctl --local trigger project/agent --direct --wait` dispatches
   successfully, then fails during wait because `wait_for_agent_exit()`
   validates the unsplit `project/agent` value and rejects `/`.
2. Direct-dispatch UDS validates only bare agent names, so
   `{"project":"bad","agent":"build-runner"}` can return `ok` before the
   orchestrator later drops it.
3. `cmd_status --since` still interpolates user input into a shell command.

### 1.2 Current Data Flow

```text
adf-ctl trigger project/agent --direct
-> split_project_agent()
-> UDS payload { project, agent, context, synthetic_event }
-> direct_dispatch::handle_connection()
-> validates only agent
-> WebhookDispatch::SpawnAgent
-> handle_direct_dispatch()
-> mention::resolve_mention()
-> spawn_agent_with_event()
```

### 1.3 Key Code Locations

| File | Relevant Area |
|------|---------------|
| `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` | `cmd_trigger`, `wait_for_agent_exit`, `cmd_status`, `validate_agent_name_for_shell` |
| `crates/terraphim_orchestrator/src/direct_dispatch.rs` | `DispatchCommand`, `start_direct_dispatch_listener`, `handle_connection` |
| `crates/terraphim_orchestrator/src/lib.rs` | `handle_direct_dispatch`, direct listener startup |
| `crates/terraphim_orchestrator/src/mention.rs` | `resolve_mention` project-aware resolution |

### 1.4 Essential Constraints

| Constraint | Why It Matters |
|------------|----------------|
| UDS must return truthful success/failure | CLI automation depends on `ok` meaning spawn was accepted. |
| `project/agent` must work with `--wait` | This is the new documented direct-dispatch shape. |
| Shell interpolation must validate or avoid user input | Local and SSH modes run `sh -c` commands. |

---

## 2. Design Plan

### 2.1 Step 1: Fix Direct `--wait` Name Handling

Modify only the direct branch in `cmd_trigger`.

Current issue:

```rust
wait_for_agent_exit(local, name, host, timeout)?;
```

Planned change:

```rust
wait_for_agent_exit(local, &agent_name, host, timeout)?;
```

Acceptance tests:

- Add or extend `adf-ctl` unit coverage for
  `split_project_agent("project/agent")`.
- Add a test around validation expectation: bare agent name is accepted,
  project-qualified value is not passed to wait.
- If direct function testing is awkward, add a small helper to compute wait
  target from `name` and test that helper.

### 2.2 Step 2: Make UDS Validation Project-Aware

Change `start_direct_dispatch_listener` to receive enough information to
validate project-qualified requests synchronously.

Preferred minimal design:

```rust
pub struct DirectDispatchAgentIndex {
    bare_names: HashSet<String>,
    qualified_names: HashSet<(String, String)>,
}
```

Build it in `lib.rs` from `self.config.agents`:

```rust
let agent_index = DirectDispatchAgentIndex::from_agents(&self.config.agents);
```

Validation logic in `direct_dispatch.rs`:

```rust
match cmd.project.as_deref() {
    Some(project) => validate (project, cmd.agent),
    None => validate cmd.agent in bare_names,
}
```

Acceptance tests:

- `{"agent":"meta-learning"}` still returns `ok`.
- `{"project":"valid-project","agent":"build-runner"}` returns `ok`.
- `{"project":"bad-project","agent":"build-runner"}` returns `error` and
  emits no dispatch.
- Existing unknown-agent test still passes.

### 2.3 Step 3: Harden `cmd_status --since`

Add a narrow validator for status durations before interpolating into shell.

Function:

```rust
fn validate_since_for_shell(since: &str) -> Result<String>
```

Allowed grammar:

```text
^[0-9]+[smhdw]$
```

Examples accepted:

- `30m`
- `1h`
- `2d`
- `1w`

Examples rejected:

- `1h'; rm -rf /`
- `now`
- `1 hour`
- empty string

Apply before command construction:

```rust
let since = validate_since_for_shell(since)?;
```

Acceptance tests:

- Valid values pass unchanged.
- Shell metacharacters fail.
- `cmd_status` uses validated value.

### 2.4 Step 4: Proof of Implementation -- Fully Functional Local ADF Flow

The implementation is not considered complete until it is proven by a fully
functional local ADF flow, not only unit tests.

Initial proof must start with `k=1` to keep the verification small, observable,
and deterministic. `k` means one matrix slot / one local flow work item for the
first proof run. Larger `k` values are out of scope until `k=1` passes.

Proof target:

- Use the local flow pattern from branch
  `task/1875-adf-ctl-local-direct-dispatch`, specifically the
  `.terraphim/flows/adf-useful-work-proof.toml` style of useful-work proof.
- Reduce the matrix to a single slot for the first run (`k=1`).
- Run the flow locally with `adf-ctl flow` against the working tree.
- The flow must produce an artefact under `.docs/adf/<issue>/` proving that the
  local flow executed useful work end-to-end.

Proof acceptance criteria:

- A local ADF flow can be loaded from `.terraphim/flows/<name>.toml`.
- With `k=1`, exactly one work slot executes and records its output.
- The flow finishes successfully and reports completed steps.
- The generated proof artefact contains the issue id, flow name, slot id, and
  successful exit status.
- The proof is captured in the PR summary before merge.

Recommended first proof command, adjusted to the final local flow name:

```bash
cargo run -p terraphim_orchestrator --bin adf-ctl -- flow adf-useful-work-proof --context "issue=1890 k=1"
```

If the flow engine does not yet parse `k` from context, implement the proof by
committing a one-slot flow fixture or by reducing the matrix in a temporary local
test fixture. Do not expand to `k=3` until the `k=1` proof succeeds.

### 2.5 Step 5: Verification

Run:

```bash
cargo fmt
cargo test -p terraphim_orchestrator --bin adf-ctl
cargo test -p terraphim_orchestrator --lib direct_dispatch
cargo test -p terraphim_orchestrator --lib
```

Then run the local ADF flow proof from section 2.4 with `k=1`.

---

## 3. Out of Scope

- Replacing all `sh -c` usage in `adf-ctl`.
- Adding authoritative cancel/status admin socket support.
- Changing synthetic event env-var names.
- Refactoring direct dispatch into a separate service layer.
- Proving higher fan-out values before `k=1` is fully functional.

---

## 4. Implementation Order

1. Add tests for `--wait` target and `--since` validation.
2. Fix direct wait target.
3. Add `DirectDispatchAgentIndex` and project-aware UDS validation tests.
4. Harden `cmd_status --since`.
5. Add or adapt the local ADF useful-work proof so the first proof run uses
   `k=1`.
6. Run verification commands and the local ADF flow proof.
7. Update PR #1885 with the proof artefact path and command output summary.

---

## 5. Approval Gate

If this plan is approved, the next step is Phase 3 implementation against PR
branch `task/1890-adf-direct-dispatch-remediation`.
