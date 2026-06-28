Axum Server for Terraphim AI
============================
This is the Axum server for Terraphim AI. It is a simple server that serves /config and /search API endpoints.

## Health / Readiness Probe

`GET /health` returns HTTP `200` with a fixed JSON body:

```json
{"status":"ok"}
```

The endpoint is available as soon as the Axum router is serving requests.
Integration tests use it as a deterministic readiness signal (polling instead
of a fixed `sleep`) — see `tests/common/mod.rs::wait_for_health`.

Note: axum have it's own default/settings.toml file to configure the server depending on the device capabilities.
it will copy default/settings.toml to ~/.config/terraphim/settings.toml if it doesn't exist on MacOS and Linux.

To build locally, run:
```bash
earthly +save-fe-local
cargo build
```
