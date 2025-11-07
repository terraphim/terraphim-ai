# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v1.0.0-sync] - 2024-11-07

### Added
- Agent pool busy/available tracking with proper lifecycle management
- Smart routing of gpt-* models to Ollama for local testing
- Linux x86_64 binaries for Ubuntu 22.04
- Debian package for terraphim_server

### Fixed
- Persistable::new() now correctly extracts agent_id from persistence keys
- Agent pool race conditions on task completion
- Pool metrics accuracy and graceful shutdown
- Clippy configuration for newer versions (literal-representation-threshold)

### Changed
- Updated tests to use Ollama instead of OpenAI
- Changed configuration from openai_model to llm_model
- Updated references from Rig to rust-genai throughout tests
- Improved structured concurrency using tokio::select! with cancellation

### Technical Details
- Cherry-picked from main branch (commit d44b09c7)
- Based on release/v1.0.0 branch (commit 5ab3d07d)
- 40 agent evolution tests passing
- Native binaries tested and working

## [v1.1.0] - 2024-11-07
- Previous release

## [v1.0.2] - 2024-11-05
- Previous release

## [v1.0.1] - 2024-11-05
- Previous release

## [v1.0.0] - 2024-11-05
- Initial v1.0.0 release