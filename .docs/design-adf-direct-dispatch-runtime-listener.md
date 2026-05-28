# Design: ADF Direct Dispatch Runtime Listener Remediation

Date: 2026-05-27 10:02 BST

Issue: #1875

## Objective

Wire the existing Unix domain socket direct dispatch listener into production orchestrator startup so `adf-ctl trigger --local --direct` can dispatch to a running local orchestrator without SSH or HTTP webhook delivery.

## Scope

In scope:

- Start `direct_dispatch::start_direct_dispatch_listener()` from `AgentOrchestrator::run()` when `[direct_dispatch]` is configured.
- Reuse the existing `WebhookDispatch` event flow into `handle_webhook_dispatch()`.
- Preserve current webhook behaviour.
- Fix rustfmt module ordering in `lib.rs`.
- Add focused runtime wiring tests where feasible.

Out of scope:

- Redesigning dispatch protocol.
- Adding admin sockets or new auth schemes.
- Adding new dependencies.
- Reworking graceful shutdown for existing webhook tasks.
- Supporting non-Unix platforms.
- Changing mention-gating semantics unless tests prove it is necessary for direct dispatch acceptance.

## File Changes

### `crates/terraphim_orchestrator/src/lib.rs`

Apply `cargo fmt` so module declarations are rustfmt-compliant, including ordering `direct_dispatch` before `dispatcher`.

In `AgentOrchestrator::run()` replace the webhook-only dispatch channel creation with a shared optional channel:

```rust
let dispatch_channel = if self.config.webhook.is_some() || self.config.direct_dispatch.is_some() {
    let (dispatch_tx, dispatch_rx) = tokio::sync::mpsc::channel(64);
    self.webhook_dispatch_rx = Some(dispatch_rx);
    Some(dispatch_tx)
} else {
    None
};
```

Then update webhook startup to use `dispatch_channel.as_ref().expect(...).clone()` rather than creating its own channel.

Add direct dispatch startup near webhook startup:

```rust
if let Some(ref direct_cfg) = self.config.direct_dispatch {
    if let Some(dispatch_tx) = dispatch_channel.as_ref() {
        let agent_names = self
            .config
            .agents
            .iter()
            .map(|agent| agent.name.clone())
            .collect();

        direct_dispatch::start_direct_dispatch_listener(
            direct_cfg.socket_path.clone(),
            dispatch_tx.clone(),
            agent_names,
        );
    }
}
```

Use the configured socket path directly. Do not add fallback paths in the orchestrator; path resolution for `adf-ctl` already belongs in the CLI.

### `crates/terraphim_orchestrator/src/direct_dispatch.rs`

No protocol changes expected. Leave existing listener tests intact.

Optional cleanup only if touched by rustfmt: remove redundant blank doc-comment lines around `start_direct_dispatch_listener()`.

### `.docs/summary-*.md` and `.docs/summary.md`

Update only if required by repository documentation practice after code changes. The immediate remediation artefacts are this design document and the companion research document.

## Test Plan

Run focused checks first:

```bash
cargo fmt -- --check
cargo clippy -p terraphim_orchestrator -- -D warnings
cargo test -p terraphim_orchestrator --lib direct_dispatch
cargo test -p terraphim_orchestrator --bin adf-ctl
```

Run broader orchestrator verification after focused checks pass:

```bash
cargo test -p terraphim_orchestrator --lib
```

Run scoped UBS on changed source files:

```bash
ubs crates/terraphim_orchestrator/src/lib.rs crates/terraphim_orchestrator/src/direct_dispatch.rs crates/terraphim_orchestrator/src/bin/adf-ctl.rs crates/terraphim_orchestrator/src/config.rs
```

Check coverage after implementation with the existing project coverage workflow if available. If `cargo llvm-cov` is installed, run package-scoped coverage for the orchestrator crate; otherwise record that coverage tooling is unavailable and preserve the earlier direct-dispatch coverage evidence.

## Runtime Validation Plan

After code verification passes, validate behaviour with a real local orchestrator process:

1. Prepare a temporary orchestrator config containing a known agent and `[direct_dispatch] socket_path = "/tmp/adf-ctl-validation.sock"`.
2. Start the orchestrator in tmux so logs can be inspected without using `sleep`.
3. Confirm the socket file exists and is a Unix socket.
4. Confirm permissions are `0600`.
5. Run `adf-ctl --config <config> --local trigger <known-agent> --direct --context "direct dispatch validation"`.
6. Confirm `adf-ctl` exits successfully and logs show the direct dispatch socket accepted and the agent dispatch path was reached.
7. Run an unknown-agent command and confirm it returns a JSON error without spawning an agent.

## Implementation Sequence

1. Run `cargo fmt` to fix formatting drift.
2. Refactor `AgentOrchestrator::run()` to create one shared dispatch channel for webhook and direct dispatch producers.
3. Update webhook startup to use the shared sender.
4. Start the direct dispatch listener when `config.direct_dispatch` is configured.
5. Add a focused unit or integration test proving direct dispatch configuration causes the runtime startup path to invoke listener creation or creates an observable socket in a controlled run harness.
6. Run focused tests and clippy.
7. Run broader orchestrator tests.
8. Perform live `adf-ctl --local --direct` validation if a suitable local config can be run safely.

## Design Notes

The design intentionally keeps `LoopEvent::Webhook` as the common dispatch event. The name is slightly broader than its original HTTP webhook use, but changing it would touch more code without improving behaviour for this fix. A future cleanup could rename it to `LoopEvent::Dispatch` once direct dispatch is fully shipped.

Listener `JoinHandle` storage is deliberately omitted for parity with the existing webhook server, which is also spawned without a stored handle. If shutdown leak detection or clean task cancellation becomes a requirement, it should be handled as a separate lifecycle-management issue covering both webhook and direct dispatch services.
