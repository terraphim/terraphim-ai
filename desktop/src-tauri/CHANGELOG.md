# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/terraphim/terraphim-ai/releases/tag/terraphim-ai-desktop-v0.1.0) - 2024-04-29

### Fixed
- fix

### Other
- Prefix all crates with `terraphim_` for consistency
- `config` -> `terraphim_config`
- Move types crate to `crates/` folder
- work on scorer
- Change API endpoint from `articles` to `documents`
- Add dummy `term_to_id.json`
- Fixes
- Rename `Settings` to `DeviceSettings`
- use `Document` and `url` everywhere
- update terraphim service interface
- Move shared logic to `terraphim_service`
- Limit tauri features and update to latest stable version
- Removed axum from dependencies
- Cleaned tauri cmd.rs and added config_update
- Port over thesaurus to Tauri
- cleanup
- cargo fmt
- build fixes
- api fixes
- clean up imports
- refactor
- Refactor config and thesaurus handling
- - Move core types into `terraphim_types` crate.
- make fmt happy
- Merge pull request [#50](https://github.com/terraphim/terraphim-ai/pull/50) from terraphim/layer_file
- clippy and formatter
- Added article cache into global config state
- Desktop default settings
- Pin dependencies to versions that are compatible with`http: 0.2.11` until all crates have updated
- Remove deprecated dependency on `terraphim_grep`
- persistance -> persistence
- * The `server-axum` folder got renamed to `terraphim_server` to align with the crate name. The behavior stays the same.
- Tauri desktop working in the same way as Axum closes [#24](https://github.com/terraphim/terraphim-ai/pull/24) and [#14](https://github.com/terraphim/terraphim-ai/pull/14)
- Search middleware: Ripgrep, moved to types
- Both axum and tauri configs work
- Axum server works and spins inside tauri, but tauri security prevents querying axum http api directly. See tauri localhost plugin
- trying to move axum server into lib - so I can reuse it with tauri
- Basic config
- new UI for desktop
