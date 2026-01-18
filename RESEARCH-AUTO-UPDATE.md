# Research Document: Automatic Updates Feature

**Status**: Draft
**Author**: Research Agent
**Date**: 2025-01-09
**Reviewers**: [Pending]

## Executive Summary

Terraphim AI currently provides only manual update functionality for `terraphim-agent` (TUI) via explicit user commands. The system lacks any automated update mechanism, scheduling, or background checking capabilities. This research examines the feasibility of implementing automatic updates across both `terraphim-agent` and `terraphim-cli`, identifying current gaps, dependencies, constraints, and implementation considerations.

## Problem Statement

### Description

Terraphim AI binaries require users to manually check for and install updates. Users must:
1. Be aware that an update exists
2. Manually run `terraphim-agent check-update` or `terraphim-agent update`
3. Remember to do this periodically
4. For `terraphim-cli`, there is currently NO update mechanism at all

This manual process creates friction for users and can lead to:
- Outdated installations with missing security patches
- Missed bug fixes and new features
- Uneven user experience across the user base
- Support burden from users running old versions

### Impact

**Primary Impact:**
- **Users**: Must manually manage updates, leading to outdated installations
- **Maintainers**: Increased support burden from users on old versions
- **Security**: Delayed security patch deployment

**Secondary Impact:**
- **terraphim-cli**: Cannot update itself at all, no update infrastructure
- **terraphim-agent**: Has manual commands only, no automation
- **Release process**: Creates binaries and packages but no auto-update delivery

### Success Criteria

1. Users receive automatic notifications of available updates
2. Updates can be automatically downloaded and installed (with opt-out)
3. Configuration options exist for:
   - Enabling/disabling auto-updates
   - Setting update check frequency
   - Choosing automatic vs. manual installation
4. `terraphim-cli` gains update capability parity with `terraphim-agent`
5. Backward compatibility is maintained (manual commands still work)
6. Updates do not interrupt active sessions unexpectedly

## Current State Analysis

### Existing Implementation

**terraphim-agent** (`crates/terraphim_agent/src/main.rs`):
- Line 255-264: `CheckUpdate` command checks for updates
- Line 864-876: `Update` command installs updates
- Line 34: Uses `terraphim_update` crate: `check_for_updates`, `update_binary`
- NO automatic scheduling
- NO background checking
- NO configuration for auto-updates
- Updates only when explicitly invoked

**terraphim-cli** (`crates/terraphim_cli/src/main.rs`):
- NO update functionality
- NO update commands
- Does NOT depend on `terraphim_update` crate
- Cannot update itself at all

**terraphim_update crate** (`crates/terraphim_update/src/lib.rs`):
- Provides `check_for_updates()`, `update_binary()`, `update_binary_silent()`
- Uses `self_update` crate v0.42 for GitHub Releases
- `UpdaterConfig` struct with: `bin_name`, `repo_owner`, `repo_name`, `current_version`, `show_progress`
- NO scheduling or auto-update logic
- NO persistent state management
- Check at line 145: `get_latest_release()` from GitHub
- Update at line 265: `update()` downloads and replaces binary

**Configuration** (`crates/terraphim_config/src/lib.rs`, `crates/terraphim_settings/src/lib.rs`):
- `Config` struct: roles, haystacks, LLM settings
- `DeviceSettings` struct: server settings, data paths, storage profiles
- NO update-related configuration fields
- NO update scheduling settings

**Release Process** (`scripts/build-release.sh`, `scripts/release.sh`):
- Creates `.deb`, `.rpm`, `.tar.gz` packages
- Docker images with multi-architecture support
- GitHub Releases as distribution channel
- NO auto-update configuration
- NO service files or launchers for background tasks

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| terraphim-agent update commands | `crates/terraphim_agent/src/main.rs:255-264, 864-876` | Manual update check and install |
| terraphim_cli main | `crates/terraphim_cli/src/main.rs` | CLI interface (no update functionality) |
| Update crate | `crates/terraphim_update/src/lib.rs` | Shared update logic using self_update |
| Configuration | `crates/terraphim_config/src/lib.rs` | Role and user configuration |
| Device Settings | `crates/terraphim_settings/src/lib.rs` | Server/device configuration |
| Release build | `scripts/build-release.sh` | Multi-platform release build |
| GitHub release | `scripts/release.sh` | Automated GitHub release creation |

### Data Flow

**Current Manual Update Flow:**
1. User runs: `terraphim-agent check-update`
2. `check_for_updates("terraphim-agent")` called (line 853)
3. Uses `self_update::backends::github::Update` to check latest release
4. Compares versions via `is_newer_version()` (simple version comparison)
5. Returns `UpdateStatus::Available` or `UpdateStatus::UpToDate`
6. User runs: `terraphim-agent update`
7. `update_binary("terraphim-agent")` downloads and replaces binary
8. Next invocation uses new binary

**Missing Auto-Update Flow:**
- NO scheduling mechanism (cron, systemd timer, etc.)
- NO background daemon or service
- NO persistent state for last check time
- NO notification system for available updates
- NO configuration for update frequency or preferences

### Integration Points

**Current Integrations:**
- `self_update` crate v0.42: GitHub Releases backend
- `tokio`: Async runtime for update operations
- GitHub Releases: Update source (terraphim/terraphim-ai repo)

**Missing Integrations:**
- Scheduling: cron, systemd timers, launchd (macOS), Windows Task Scheduler
- Notification: Desktop notifications, in-app notifications
- Configuration: Update settings in config files
- Background execution: Daemon/service infrastructure
- State persistence: Last check time, update history

## Constraints

### Technical Constraints

**Language & Runtime:**
- Rust async runtime (tokio) for update operations
- Cross-platform support required: Linux, macOS, Windows
- `self_update` crate only supports certain platforms (not Windows support documented)

**Platform-Specific Constraints:**
- **Linux**: systemd services, cron jobs, desktop notifications (libnotify)
- **macOS**: launchd agents, NotificationCenter
- **Windows**: Task Scheduler, Windows notifications
- **Background services**: Different mechanisms per OS

**Dependency Constraints:**
- `self_update` crate v0.42: Limited documentation, unclear platform support
- GitHub Releases: Must use existing release pipeline
- Binary replacement: Requires write permissions to installation path
- Restart mechanism: Must restart binary after update

**Installation Path Constraints:**
- System-wide installs: `/usr/local/bin`, `/usr/bin` - require root/sudo
- User installs: `~/.local/bin` - no special permissions
- Package managers (.deb, .rpm): Managed by package manager, not self-update
- Docker containers: Updates via image rebuild, not in-container updates

### Business Constraints

**User Experience:**
- Updates must not interrupt active work sessions
- Users should be able to opt-out of automatic updates
- Update process should be transparent and informative
- Rollback capability for failed updates

**Security:**
- Updates must be cryptographically verified
- No automatic updates from untrusted sources
- Signed releases or checksums required
- Safe binary replacement (atomic, verified)

**Release Management:**
- Must work with existing GitHub Release pipeline
- Cannot require changes to build/release scripts
- Must support multiple release channels (stable, beta)

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Update check frequency | Configurable (default: daily) | Not implemented |
| Update installation time | < 30 seconds | Manual: depends on connection |
| Binary verification | Signed/verified | Not verified |
| Rollback time | < 1 minute | Not implemented |
| User notification | Desktop + in-app | Not implemented |
| Background resource usage | < 1% CPU, < 50MB RAM | Not implemented |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_update` | Core update logic, provides check/update functions | Medium - needs extension for auto-update |
| `terraphim_config` | Should store update settings | Low - add fields to structs |
| `terraphim_settings` | Device-level update configuration | Low - add update config |
| `tokio` | Async runtime for update operations | Low - already used |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `self_update` | 0.42 | High - unclear docs, limited platform support | `update-informer`, `self_update_crate` |
| GitHub Releases | API | Low - well-documented, stable | GitLab releases, custom server |
| Platform schedulers | Varies | Medium - different per platform | Rust scheduler crates |
| Notification systems | Varies | Medium - different per platform | Rust notification crates |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `self_update` crate limitations | High | High | Spike: test platform support, consider alternatives |
| Binary replacement failures (permissions) | Medium | High | Graceful degradation, user prompt for elevated permissions |
| Update during active session interruption | Medium | High | Queue update for next restart, or prompt user |
| OS-specific scheduler complexity | High | Medium | Use Rust abstractions (e.g., `tokio-cron-scheduler`) |
| User resistance to automatic updates | Low | Medium | Make auto-update opt-in with clear value prop |
| Package manager conflicts (deb/rpm) | Medium | Medium | Detect package-managed installs, disable self-update |
| Network failures during update | High | Low | Retry logic, partial download resume |
| Malicious release compromises | Low | High | Signature verification, checksums |
| Update loop (failing update) | Low | High | Update history, rollback after N failures |

### Open Questions

1. **Platform Support**: Does `self_update` crate support Windows? Windows is listed in constraints but unclear if crate supports it. (Required investigation)

2. **Installation Detection**: How to detect if binary was installed via package manager vs. manual copy? Package-managed installs should use package manager for updates, not self-update. (Required investigation)

3. **Scheduling Mechanism**: Should we use platform-native schedulers (systemd, launchd, cron) or Rust-based scheduling? What's the tradeoff? (Required stakeholder input)

4. **Update Frequency**: What should be the default check frequency? Daily? Weekly? Monthly? (Required stakeholder input)

5. **Background Service**: Do we need a dedicated update daemon/service, or can updates run on-demand when binary starts? (Required architectural decision)

6. **User Notification**: What notification method should be used? Desktop notifications? In-app banners? Both? (Required UX input)

7. **Rollback Strategy**: How should rollback work if update fails? Keep backup of old binary? Use system rollback (e.g., btrfs snapshots)? (Required design decision)

8. **Configuration Storage**: Should update settings be per-user or system-wide? Where should they be stored? (Required design decision)

9. **CLI Update Parity**: Should `terraphim-cli` have full update capabilities, or just basic update check? (Required scope decision)

10. **Release Channels**: Do we need to support multiple release channels (stable, beta, nightly) with different update policies? (Required stakeholder input)

### Assumptions

1. **GitHub Releases Stability**: Assumes GitHub Releases API remains stable and accessible. (Based on current implementation using GitHub)

2. **Binary Write Permissions**: Assumes update process can write to installation directory or can prompt user for elevated permissions. (Based on typical CLI tool behavior)

3. **Network Connectivity**: Assumes updates are downloaded from internet, not from local mirrors. (Based on current implementation)

4. **User Consent**: Assumes users want automatic updates but with ability to opt-out. (Based on industry standard behavior)

5. **Restart Acceptable**: Assumes restarting binary after update is acceptable behavior. (Based on self-update requirements)

6. **Platform Homogeneity**: Assumes we need to support Linux, macOS, and Windows. (Based on project release scripts targeting multiple platforms)

## Research Findings

### Key Insights

1. **Partial Infrastructure Exists**: `terraphim_agent` has update commands, but `terraphim_cli` has none. Shared `terraphim_update` crate provides core logic but lacks scheduling.

2. **No Auto-Update Logic**: Current implementation is purely manual. No scheduling, no background checking, no persistence, no notifications.

3. **Platform Complexity**: Automatic updates require OS-specific mechanisms:
   - Linux: systemd timers, cron, inotify
   - macOS: launchd agents
   - Windows: Task Scheduler
   - Cross-platform: Rust abstraction layer needed

4. **Self-Update Crate Uncertainty**: `self_update` crate v0.42 has limited documentation. Platform support unclear. May need to spike test on all target platforms.

5. **Configuration Gap**: No existing configuration fields for updates. Need to extend `Config` and `DeviceSettings` structs.

6. **Installation Method Detection**: Need to detect package-managed vs. manual installs to avoid conflicts.

7. **Session Management**: Updates must not interrupt active sessions. Need queue-and-apply-on-restart mechanism.

8. **Verification Gap**: Current implementation does not verify binary signatures or checksums. Security risk for automatic updates.

### Relevant Prior Art

- **Rust Analyzer**: Uses `self_update` crate, checks for updates on startup, prompts user
- **Ripgrep**: Manual updates only, uses `cargo install` for updates
- **Docker CLI**: Uses version check on startup, prompts user to download manually
- **Homebrew**: Automatic updates via `brew upgrade` scheduled task
- **apt-get**: Uses `unattended-upgrades` package for automatic security updates

**Lessons from prior art:**
- Manual updates are simpler but lead to outdated installations
- Automatic updates require careful UX (notifications, opt-out, rollback)
- Package manager detection is critical to avoid conflicts
- Restart after update is unavoidable for binary tools

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| `self_update` platform test | Test `self_update` crate on Linux, macOS, Windows to verify platform support and behavior | 4-6 hours |
| Installation detection | Implement detection of package-managed vs. manual installs | 4-8 hours |
| Scheduling mechanism | Evaluate Rust scheduling libraries (tokio-cron-scheduler) vs. platform-native schedulers | 6-8 hours |
| Notification system | Test Rust notification crates (notify-rust) for cross-platform desktop notifications | 4-6 hours |
| Binary verification | Implement GPG signature verification or checksum validation for downloaded binaries | 8-12 hours |
| Rollback mechanism | Design and implement rollback strategy (backup binary, test restore) | 8-12 hours |
| Configuration UI | Design configuration structure for update settings (frequency, auto-install, channel) | 4-6 hours |

## Recommendations

### Proceed/No-Proceed

**Recommendation**: **PROCEED with conditions**

Justification:
- High user value: Automatic updates will significantly improve user experience
- Partial infrastructure exists: Update logic is already implemented, just needs scheduling and automation
- Technical feasibility: Clear path forward with known solutions
- Manageable complexity: Can implement incrementally (MVP first, then enhancements)

**Conditions for proceeding:**
1. Complete technical spikes first (especially `self_update` platform test)
2. Confirm `self_update` crate supports all target platforms or find alternative
3. Define update policy (opt-in vs. opt-out, default frequency)
4. Approve architectural approach (daemon vs. on-demand)

### Scope Recommendations

**Phase 1: MVP (Minimum Viable Product)**
1. Add update configuration to `DeviceSettings`
2. Implement check-on-startup for `terraphim-agent` and `terraphim-cli`
3. Add in-app notification when update available
4. Manual update command only (no auto-install)
5. Basic scheduling: check on startup only

**Phase 2: Automatic Installation**
1. Add auto-install option (disabled by default)
2. Implement queue-and-apply-on-restart mechanism
3. Add desktop notifications
4. Add update history tracking

**Phase 3: Advanced Features**
1. Scheduled background checks (configurable frequency)
2. Multiple release channels (stable, beta)
3. Automatic rollback on failure
4. Update telemetry (with user consent)
5. Update health metrics

**Out of Scope (deferred):**
- GUI update prompts (desktop application)
- Delta updates (download only changed portions)
- Peer-to-peer updates (P2P distribution)
- Custom update servers (non-GitHub)

### Risk Mitigation Recommendations

**High-Priority Mitigations:**
1. **Complete technical spikes**: Reduce uncertainty about `self_update` and platform support before implementation
2. **Package manager detection**: Critical to avoid breaking package-managed installations
3. **Graceful degradation**: If auto-update fails, fall back to manual update commands
4. **Extensive testing**: Test on all target platforms, edge cases (network failures, permission errors)
5. **User communication**: Clear documentation, release notes, upgrade guides

**Medium-Priority Mitigations:**
1. **Opt-in by default**: Reduce user resistance, clear opt-out instructions
2. **Rollback capability**: Maintain backup of previous binary for 7 days
3. **Update validation**: Verify checksums/signatures before installing
4. **Rate limiting**: Don't check too frequently, respect GitHub API limits
5. **Testing in CI**: Mock GitHub API, test update scenarios in automated tests

**Low-Priority Mitigations:**
1. **Telemetry**: Track update success rates (with user consent)
2. **A/B testing**: Test different update frequencies and notifications
3. **User feedback**: Collect feedback on update experience
4. **Documentation**: Write troubleshooting guides for common update issues

## Next Steps

If approved:

1. **Complete technical spikes** (2-3 days):
   - Spike 1: Test `self_update` on all platforms
   - Spike 2: Installation detection research
   - Spike 3: Scheduling mechanism evaluation
   - Spike 4: Notification system research

2. **Create design document** (Phase 2 of disciplined development):
   - Detailed architecture for auto-update system
   - Configuration schema design
   - Data flow diagrams
   - Error handling strategy
   - Rollback mechanism design

3. **Implementation plan** (Phase 3):
   - File changes list
   - Test strategy
   - Rollout plan
   - Backward compatibility plan

4. **Stakeholder review**:
   - Present research findings
   - Review design document
   - Approve implementation plan
   - Set timeline and milestones

## Appendix

### Reference Materials

- **`self_update` crate**: https://docs.rs/self_update/latest/self_update/
- **GitHub Releases API**: https://docs.github.com/en/rest/releases/releases
- **tokio-cron-scheduler**: https://docs.rs/tokio-cron-scheduler/latest/tokio_cron_scheduler/
- **notify-rust**: https://docs.rs/notify-rust/latest/notify_rust/
- **systemd timers**: https://www.freedesktop.org/software/systemd/man/systemd.timer.html
- **launchd**: https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html

### Code Snippets

**Current check-update command** (terraphim-agent:851-863):
```rust
Command::CheckUpdate => {
    println!("🔍 Checking for terraphim-agent updates...");
    match check_for_updates("terraphim-agent").await {
        Ok(status) => {
            println!("{}", status);
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Failed to check for updates: {}", e);
            std::process::exit(1);
        }
    }
}
```

**Current update command** (terraphim-agent:864-876):
```rust
Command::Update => {
    println!("🚀 Updating terraphim-agent...");
    match update_binary("terraphim-agent").await {
        Ok(status) => {
            println!("{}", status);
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Update failed: {}", e);
            std::process::exit(1);
        }
    }
}
```

**UpdaterConfig structure** (terraphim_update:93-131):
```rust
pub struct UpdaterConfig {
    pub bin_name: String,
    pub repo_owner: String,
    pub repo_name: String,
    pub current_version: String,
    pub show_progress: bool,
}
```

**DeviceSettings structure** (terraphim_settings:64-76):
```rust
pub struct DeviceSettings {
    pub server_hostname: String,
    pub api_endpoint: String,
    pub initialized: bool,
    pub default_data_path: String,
    pub profiles: HashMap<String, HashMap<String, String>>,
}
```
