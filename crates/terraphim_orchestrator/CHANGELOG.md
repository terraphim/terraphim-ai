# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.9.0] - 2026-03-20

### Added

#### Dual Mode Orchestrator
- **ModeCoordinator**: New coordinator that manages both TimeMode and IssueMode simultaneously
  - Supports three execution modes: `TimeOnly`, `IssueOnly`, and `Dual`
  - Unified shutdown with queue draining and active task waiting
  - Stall detection with configurable thresholds
  - Concurrency control via semaphore-based limiting
  
- **Unified Dispatch Queue**: Priority queue with fairness between task types
  - Time tasks get medium priority (50)
  - Issue tasks use variable priority (0-255) based on labels and PageRank
  - Round-robin fairness to prevent starvation
  - Bounded queue with backpressure

- **Issue Mode Integration**: Full support for issue-driven task scheduling
  - Gitea tracker integration via `terraphim_tracker` crate
  - Automatic issue-to-agent mapping based on labels and title patterns
  - Priority calculation using PageRank scores
  - Poll-based issue discovery with configurable intervals

#### Compatibility and Migration
- **Symphony Compatibility Layer** (`src/compat.rs`):
  - Type aliases for backward compatibility (`SymphonyOrchestrator`, `SymphonyAgent`)
  - `SymphonyAdapter` for config migration
  - `SymphonyOrchestratorExt` trait for runtime mode detection
  - Migration helper functions in `compat::migration` module

- **Migration Documentation** (`MIGRATION.md`):
  - Step-by-step migration guide from legacy to dual mode
  - Configuration examples for all three modes
  - Troubleshooting section for common issues
  - Backward compatibility notes

#### Testing
- **End-to-End Tests** (`tests/e2e_tests.rs`):
  - `test_dual_mode_operation`: Verify both time and issue tasks processed
  - `test_time_mode_only`: Legacy config compatibility
  - `test_issue_mode_only`: Issue-only config verification
  - `test_fairness_under_load`: No starvation between task types
  - `test_graceful_shutdown`: Clean termination with queue draining
  - `test_stall_detection`: Warning logged when queue exceeds threshold
  - Additional tests for concurrency limits, prioritization, and backward compatibility

#### Configuration
- New `[workflow]` section in `orchestrator.toml`:
  - `mode`: Execution mode selection (`time_only`, `issue_only`, `dual`)
  - `poll_interval_secs`: Issue polling frequency
  - `max_concurrent_tasks`: Parallel execution limit

- New `[tracker]` section for issue tracking:
  - `tracker_type`: `gitea` or `linear`
  - `url`, `token_env_var`, `owner`, `repo`: Connection details

- New `[concurrency]` section for performance tuning:
  - `max_parallel_agents`: Concurrent agent limit
  - `queue_depth`: Stall detection threshold
  - `starvation_timeout_secs`: Task timeout

### Changed

- **Enhanced AgentOrchestrator**: Extended with mode coordination capabilities
  - `reconcile_tick()` now includes stall detection and queue dispatch
  - `unified_shutdown()` for coordinated shutdown across modes
  - `check_stall()` for monitoring queue health
  - `dispatch_from_queue()` for spawner integration

- **Updated Documentation**:
  - `CLAUDE.md`: Added dual mode architecture section
  - `MIGRATION.md`: Comprehensive migration guide
  - Inline documentation for all new public APIs

### Deprecated

- None

### Removed

- None

### Fixed

- None

### Security

- None

## [1.8.0] - Previous Release

### Notes

This release introduces the Symphony orchestrator port completion with dual mode support. The changes are fully backward compatible - existing configurations without the `[workflow]` section continue to work in legacy time-only mode.

### Migration Path

To migrate from time-only to dual mode:

1. Add `[workflow]` section to `orchestrator.toml`
2. Add `[tracker]` section with Gitea/Linear credentials
3. Set environment variable for tracker token
4. Restart orchestrator

See `MIGRATION.md` for detailed instructions.
