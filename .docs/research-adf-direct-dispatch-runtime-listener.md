# Research: ADF Direct Dispatch Runtime Listener

Date: 2026-05-27 10:02 BST

Issue: #1875

## Problem Statement

The current direct dispatch implementation adds the Unix domain socket protocol and real socket-level tests, but the listener is not started by production orchestrator runtime code. As a result, `[direct_dispatch]` configuration is parsed and `adf-ctl trigger --local --direct` can attempt a socket connection, but no orchestrator process creates or accepts connections on the configured socket.

The latest review also identified a formatting failure: `cargo fmt -- --check` wants the module declarations in `crates/terraphim_orchestrator/src/lib.rs` reordered.

## Current Implementation Findings

### Existing direct dispatch module

`crates/terraphim_orchestrator/src/direct_dispatch.rs` already provides the core listener function:

- `start_direct_dispatch_listener(socket_path, dispatch_tx, agent_names) -> JoinHandle<()>`
- Removes stale socket files only when the path is already a socket.
- Refuses to remove non-socket paths.
- Binds a `tokio::net::UnixListener`.
- Sets socket permissions to `0600` on Unix.
- Reads newline-delimited JSON requests from `adf-ctl`.
- Validates agent names against the configured agent set.
- Forwards valid requests as `WebhookDispatch::SpawnAgent`.
- Returns JSON `ok` or `error` responses to the client.

The module has real Unix socket tests covering valid dispatches, unknown agents, invalid JSON, stale socket cleanup, non-socket path refusal, and closed orchestrator channels.

### Existing orchestrator event loop

`crates/terraphim_orchestrator/src/lib.rs` already has a dispatch event path suitable for reuse:

- `AgentOrchestrator` stores `webhook_dispatch_rx: Option<mpsc::Receiver<WebhookDispatch>>`.
- `run()` defines `LoopEvent::Webhook(webhook::WebhookDispatch)`.
- `run()` creates a loop channel and bridges `webhook_dispatch_rx` into `LoopEvent::Webhook`.
- `LoopEvent::Webhook` calls `handle_webhook_dispatch(dispatch).await`.
- `handle_webhook_dispatch()` already contains the common `SpawnAgent` logic used by the webhook path.

This means the runtime gap does not require a new dispatch path. Direct dispatch should feed the same `WebhookDispatch` queue already consumed by the reconciliation loop.

### Existing webhook startup

The webhook startup block currently creates its own `mpsc::channel(64)` inside `if let Some(ref webhook_cfg) = self.config.webhook`, assigns the receiver to `self.webhook_dispatch_rx`, and gives the sender to `webhook::WebhookState`.

That shape is the root of the direct dispatch integration risk: if direct dispatch creates a separate channel, the reconciliation loop will only consume one receiver unless the design explicitly merges them. The smallest safe change is to create a single shared dispatch channel before optional webhook/direct startup, clone the sender into each producer, and store the single receiver once.

### Configuration state

`OrchestratorConfig` has `direct_dispatch: Option<DirectDispatchConfig>` and all test/runtime literal initialisers have been updated with `direct_dispatch: None`. The config field is currently inert because `run()` does not inspect it.

The expected runtime behaviour is:

- If `[direct_dispatch]` is absent, no socket should be created and existing webhook-only behaviour should remain unchanged.
- If `[direct_dispatch]` is present, the orchestrator should start the Unix socket listener during `run()` startup.
- The listener should reuse the same `WebhookDispatch` event path as webhooks.
- `adf-ctl --local trigger --direct` should work without SSH and without HTTP webhook/HMAC.

### Formatting state

`cargo fmt -- --check` reports a module ordering difference in `lib.rs`; rustfmt wants `direct_dispatch` before `dispatcher`. This is mechanical and should be fixed with `cargo fmt` rather than hand-formatting.

## Risks and Constraints

### Concurrency and channel ownership

The event loop currently consumes at most one receiver. Multiple optional producers must share one `mpsc::Sender<WebhookDispatch>`. Creating independent channels would silently strand one producer.

### Listener lifetime

`start_direct_dispatch_listener()` returns a `JoinHandle<()>`, but the current webhook server handle is not stored either. The minimal consistent runtime change can spawn the listener and allow the task to live for the process lifetime, matching the existing webhook server pattern.

Future shutdown improvements could track and abort listener handles, but that is not required to satisfy the issue and would broaden the change beyond the review finding.

### Webhook handler side effects

`handle_webhook_dispatch()` starts with `let mention_cfg = match self.config.mentions.as_ref() { Some(cfg) => cfg, None => return }`. Direct dispatch through `WebhookDispatch::SpawnAgent` will therefore require `mentions` configuration to be present or it will be ignored.

This behaviour predates direct dispatch and is shared with webhook dispatch. It may be acceptable for current ADF deployments if mention dispatch is configured. If local direct dispatch is expected to work without `[mentions]`, that is a separate semantic gap that should be addressed explicitly because it changes dispatch gating behaviour.

### Synthetic issue and comment IDs

Direct dispatch currently emits `issue_number: 0` and `comment_id: 0`. The event loop calls `mark_webhook_comment_processed(comment_id)` for every webhook loop event. This should be reviewed for harmlessness with `comment_id == 0`. If harmful, the direct path should either avoid marking synthetic comments or introduce a more explicit dispatch origin.

The smallest safe first pass is to keep the existing protocol and verify tests around `LoopEvent::Webhook` handling do not depend on a positive comment ID.

### Socket path safety

The listener already refuses non-socket paths and sets `0600` permissions. Runtime wiring must pass through the configured socket path unchanged and must not introduce fallback cleanup that can remove arbitrary files.

### Platform support

The implementation uses Unix domain sockets. The existing code is Unix-specific. The immediate workspace target is Linux. No Windows support should be added as part of this issue.

## Acceptance Criteria

1. `AgentOrchestrator::run()` starts `start_direct_dispatch_listener()` when `config.direct_dispatch` is `Some`.
2. Webhook and direct dispatch use one shared `mpsc::Receiver<WebhookDispatch>` consumed by the existing reconciliation loop.
3. Existing webhook dispatch behaviour remains unchanged when webhook config is present.
4. Existing orchestrator behaviour remains unchanged when both webhook and direct dispatch config are absent.
5. The direct dispatch socket is created at the configured path with existing listener safety properties.
6. `cargo fmt -- --check` passes.
7. Focused tests cover runtime wiring, not only the direct dispatch module in isolation.
8. `adf-ctl trigger --local --direct` can connect to a socket created by the orchestrator runtime after live validation is available.

## Recommended Implementation Direction

Create one optional shared dispatch channel in `AgentOrchestrator::run()` when either `self.config.webhook.is_some()` or `self.config.direct_dispatch.is_some()` is true. Store the receiver in `self.webhook_dispatch_rx` as today, then clone the sender into the webhook state and direct dispatch listener.

This keeps all dispatch handling in the existing event loop and avoids introducing a second event type, a second receiver bridge, or a new runtime service manager.
