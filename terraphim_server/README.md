Axum Server for Terraphim AI
============================
This is the Axum server for Terraphim AI. It is a simple server that serves /config and /search API endpoints.

Note: axum have it's own default/settings.toml file to configure the server depending on the device capabilities. 
it will copy default/settings.toml to ~/.config/terraphim/settings.toml if it doesn't exist on MacOS and Linux. 

To build locally, run:
```bash
earthly +save-fe-local
cargo build
```
