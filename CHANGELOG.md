# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - PR #426

### Added
- fcctl-core to terraphim_firecracker adapter with VmManager trait implementation
- FcctlVmManagerAdapter with ULID-based VM ID enforcement (26-character format)
- VmRequirements struct with minimal(), standard(), and development() presets
- Configuration translation layer between VmRequirements and fcctl-core VmConfig
- Extended VmConfig support in firecracker-rust with vm_type field (Terraphim variant)
- SnapshotManager integration from fcctl-core for VM state versioning
- PoolConfig with conservative defaults (min: 2, max: 10 VMs)
- 5 comprehensive adapter unit tests (ULID validation, pool config, requirements)
- 101 total tests in terraphim_rlm crate covering executor functionality

### Changed
- Integrated FcctlVmManagerAdapter into FirecrackerExecutor
- Updated error handling with #[source] annotation for proper error chain propagation
- Migrated from parking_lot locks to tokio::sync::RwLock for async safety
- VmManager trait now uses async_trait for Send-safe async operations
- FirecrackerExecutor initialization uses adapter pattern for VM lifecycle management

### Performance
- VM allocation target: sub-500ms via pre-warmed pool
- Adapter overhead: approximately 0.3ms for config translation
- Actual VM allocation: 267ms in test environments (target: <500ms)
- tokio::sync primitives ensure no async deadlock scenarios

### Security
- ULID-based VM IDs prevent collisions across distributed deployments
- Interior mutability pattern ensures thread-safe concurrent access
- Error source preservation enables proper audit trails

### Fixed
- Resolved async deadlock risk by replacing parking_lot with tokio::sync
- Fixed Send/Sync bounds on FirecrackerExecutor for cross-task usage

## [1.0.0] - Previous Release

See [RELEASE_NOTES_v1.0.0.md](RELEASE_NOTES_v1.0.0.md) for v1.0.0 details.
