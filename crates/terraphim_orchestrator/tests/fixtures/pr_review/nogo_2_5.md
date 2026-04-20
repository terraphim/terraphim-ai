<h3>Summary</h3>

This PR introduces a new `/admin/run-arbitrary` endpoint that executes
shell commands passed in a JSON body. The endpoint is behind a bearer
token but the token check is racy and the command string is interpolated
into a shell without escaping. Two P0 data-loss/RCE risks and one P1
retry-loop issue must be resolved before this can merge.

Key changes:

- **admin.rs**: new route `/admin/run-arbitrary`.
- **orchestrator.rs**: registers the route unconditionally.

What remains problematic:

- Command injection via unescaped shell interpolation.
- Token check short-circuits before the constant-time compare.
- Retry loop swallows auth failures.

Acceptance criteria:

- [ ] security review sign-off
- [ ] integration test coverage
- [ ] staging deployment dry-run

<h3>Confidence Score: 2/5</h3>

- Do not merge until the two P0 findings are resolved.
- P0 findings are an RCE vector and an auth bypass race. One P1 in the
  retry loop obscures the failure. Two P2 hygiene notes.
- `crates/terraphim_orchestrator/src/admin.rs` must not ship.

<h3>Important Files Changed</h3>

| Filename | Overview |
|----------|----------|
| `crates/terraphim_orchestrator/src/admin.rs` | New route with RCE and auth bypass; blocks merge. |

<h3>Inline Findings</h3>

**P0 crates/terraphim_orchestrator/src/admin.rs, line 64**: **Shell command injection**

`let cmd = format!("sh -c '{}'", req.command);` interpolates user input
directly into a shell string. Any caller can break out of the quotes and
run arbitrary commands. Use `std::process::Command::new("sh").arg("-c").arg(&req.command)`.

**P0 crates/terraphim_orchestrator/src/admin.rs, line 42**: **Token comparison short-circuits**

`if token == expected { ... }` uses ordinary `==`, which short-circuits
on the first differing byte and leaks timing information. Use a
constant-time comparison (`subtle::ConstantTimeEq`).

**P1 crates/terraphim_orchestrator/src/admin.rs, line 110**: **Retry loop swallows 401s**

The retry loop catches `reqwest::Error` and retries on every variant,
including `401 Unauthorized`. This masks auth failures as transient.

**P2 crates/terraphim_orchestrator/src/admin.rs, line 21**: **Hard-coded log level**

`tracing::debug!` should be `tracing::trace!` for the per-request payload
to avoid log volume in staging.

**P2 crates/terraphim_orchestrator/src/orchestrator.rs, line 1204**: **Route registration not feature-gated**

The `/admin/run-arbitrary` route is registered unconditionally. Gate it
behind the `admin` cargo feature so non-admin builds do not expose it.

<sub>Last reviewed commit: 62672e38 | Reviews (1)</sub>
