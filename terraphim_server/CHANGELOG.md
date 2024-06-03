# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/terraphim/terraphim-ai/compare/terraphim_server-v0.1.0...terraphim_server-v0.1.1) - 2024-06-03

### Other
- faster doc build
- cargo fmt
- Fixing test and consistent folder management
- Fixing test and search logic for default scorer
- Spring clean config - refactor redundant fields and kg optional
- Spring clean config - refactor redundant fields and kg optional
- cargo fmt
- node yarn fixes
- Logo fixes
- Update Tauri code after search endpoint changes ([#90](https://github.com/terraphim/terraphim-ai/pull/90))
- bumped rust version 1.76.0
- Formatter applied
- Merge remote-tracking branch 'origin' into replacer
- Replaces uncommented

### Removed
- removed sorting by key

## [0.1.0](https://github.com/terraphim/terraphim-ai/releases/tag/terraphim_server-v0.1.0) - 2024-04-29

### Fixed
- fix some tests
- fix lints
- fix test

### Other
- Prefix all crates with `terraphim_` for consistency
- `config` -> `terraphim_config`
- Move types crate to `crates/` folder
- Use local haystack
- cleanup
- work on scorer
- wip
- Change API endpoint from `articles` to `documents`
- Fix haystack path
- Make API return proper JSON response even for errors
- Use thesaurus in fixtures
- Better error messages
- Fixes
- Integrate scorer
- Rename `Settings` to `DeviceSettings`
- cleanup
- Introduce `AutomataPath` for easier testing and more idiomatic automata loading
- use `Document` and `url` everywhere
- merge article and document
- Make document body and article id non-optional
- Fix ordering; better logging
- cleanup
- update terraphim service interface
- Move shared logic to `terraphim_service`
- Fix config tests ([#59](https://github.com/terraphim/terraphim-ai/pull/59))
- test setup and run tests sequentially
- work on tests
- integrate thesaurus
- more log messages
- cleanup
- build fixes
- api fixes
- clean up imports
- refactor
- Split up into indexer and kb_builder middleware
- `load_automata` -> `load_thesaurus`
- Refactor config and thesaurus handling
- rebase
- cleanup
- Move tests to `tests` folder as they are integration tests
- Fix server start
- Two other methods to start axum server before tests - using tokio OnceCell and ctor
- Axum start before test
- - Move core types into `terraphim_types` crate.
- messing with Layerfile
- clippy and formatter applied
- clippy and formatter
- clippy and formatter
- Added article cache into global config state
- Readme update
- Load from default config if config doesn't exist
- Improve settings handling
- cargo fmt
- All tests pass and test dataset cloned before test
- POST return empty
- Tests are green
- Takes default settings from CARGO_MANIFEST_DIR
- Embed default config
- Fixed build
- Pin dependencies to versions that are compatible with`http: 0.2.11` until all crates have updated
- persistance -> persistence
- * The `server-axum` folder got renamed to `terraphim_server` to align with the crate name. The behavior stays the same.
