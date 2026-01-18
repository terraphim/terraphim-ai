# Feature Request: Advanced Auto-Update Functionality (Phase 2)

**Labels:** `enhancement`, `auto-update`, `phase-2`, `good first issue`

---

## Problem Statement

Phase 1 auto-update implementation established the core update mechanism but intentionally deferred several advanced features to maintain shipping velocity. Users now require enhanced update experiences including desktop notifications, background updates, telemetry for reliability monitoring, and multi-platform support including Windows.

## Overview

This issue tracks advanced auto-update features that build upon the Phase 1 foundation. These enhancements focus on user experience, reliability, and platform coverage while maintaining the project's privacy-first principles.

## Features

### 1. Desktop Notifications (Priority: 3)

**Description:** Display native system tray notifications for update availability, download progress, and completion status across all platforms.

**Platform Implementations:**
- **Linux:** libnotify / freedesktop notifications via `notify-rust` crate
- **macOS:** NSUserNotification framework via `notify-rust` crate
- **Windows:** Windows Toast notifications via `winrt-notifications` or `notify-rust`

**Acceptance Criteria:**
- [ ] Users receive notification when update is available
- [ ] Users receive notification when update download completes
- [ ] Users receive notification when update installation succeeds/fails
- [ ] Notifications respect system Do Not Disturb settings
- [ ] Notifications include relevant action buttons (Update Now, Remind Later)
- [ ] Notification timeout and persistence configurable
- [ ] Tested on Linux, macOS, and Windows

**User Stories:**
- As a user, I want to receive a popup notification when a new version is available so I don't miss important updates
- As a user, I want to be notified when an update completes successfully so I know I'm running the latest version
- As a user, I want notification action buttons to quickly start updates without opening the application

**Implementation Complexity:** Medium
- Requires native platform notification libraries
- Cross-platform testing needed
- Handle platform-specific permissions and user preferences

**Technical Considerations:**
- Use `notify-rust` crate for cross-platform notifications (already in ecosystem)
- Handle notification service unavailability gracefully
- Store notification preferences in user config
- Consider accessibility features for notifications
- Bundle notification assets (icons, sounds) appropriately

**Dependencies:** None (can be implemented independently)

---

### 2. Background Daemon (Priority: 2)

**Description:** System service/agent that performs scheduled update checks even when the main application is not running.

**Platform Implementations:**
- **Linux:** systemd service and timer unit files installed to `/etc/systemd/system/`
- **macOS:** launchd agent plist file installed to `~/Library/LaunchAgents/` or `/Library/LaunchDaemons/`
- **Windows:** Task Scheduler task configured for periodic execution

**Acceptance Criteria:**
- [ ] System service installs/uninstalls correctly on all platforms
- [ ] Service checks for updates according to configurable schedule (default: daily)
- [ ] Service respects user update preferences (auto-update enabled/disabled)
- [ ] Service logs activity to standard system log locations
- [ ] Service wakes from sleep if scheduled check was missed
- [ ] Service runs with minimal resource usage
- [ ] Service handles authentication/credentials securely
- [ ] Users can enable/disable background checks in configuration

**User Stories:**
- As a user, I want updates to be checked automatically even when I'm not using the application so I always have the latest version
- As a user, I want to configure how often background checks happen so I can balance freshness and battery usage
- As a user, I want to disable background updates if I prefer manual control

**Implementation Complexity:** High
- Requires platform-specific service installation and management
- Security and permission considerations
- Service lifecycle management (start, stop, restart, status)
- Handling user sessions (user-level vs system-level services)

**Technical Considerations:**
- **Linux:** Create systemd service unit (`terraphim-updater.service`) and timer unit (`terraphim-updater.timer`)
- **macOS:** Create launchd agent with `StartInterval` or calendar-based schedule
- **Windows:** Use Task Scheduler API via `win32` crate or `taskschd` COM interface
- Service should run a lightweight binary (separate from main app or shared library)
- Use existing Phase 1 update checking logic from service
- Handle service permissions appropriately (user-level service sufficient)
- Service should not require root/admin privileges for user-level updates
- Implement service status checking and debugging tools

**Dependencies:** Phase 1 complete (reuses update checking logic)

---

### 3. Update Telemetry (Priority: 5)

**Description:** Collect anonymous statistics about update operations to improve reliability and understand user behavior.

**Data Points:**
- Update success/failure rates
- Platform distribution (OS, architecture)
- Version distribution (current, target)
- Update timing metrics (check frequency, download time, install time)
- Channel distribution (stable, beta)

**Privacy Guarantees:**
- Opt-out by default (user must explicitly enable)
- No personal data (no IP addresses, user IDs, machine identifiers)
- Anonymous aggregate reporting
- Transparent about what data is collected
- User can disable at any time

**Acceptance Criteria:**
- [ ] Telemetry disabled by default
- [ ] Users can enable/disable via configuration
- [ ] Telemetry data is anonymized before transmission
- [ ] Telemetry endpoint is configurable
- [ ] Failed telemetry collection does not block updates
- [ ] Documentation clearly explains what data is collected and why
- [ ] Privacy policy updated to include telemetry details

**User Stories:**
- As a user, I want the option to share anonymous update statistics to help improve the project
- As a user, I want telemetry disabled by default to protect my privacy
- As a user, I want to know exactly what data is being collected before enabling telemetry

**Implementation Complexity:** Medium
- Design telemetry data structure and schema
- Implement privacy-preserving collection and transmission
- Handle network failures gracefully
- Provide user controls and documentation

**Technical Considerations:**
- Design simple JSON schema for telemetry payloads
- Use existing HTTP client for transmission to configurable endpoint
- Implement exponential backoff for failed transmissions
- Cache telemetry locally and send in batches
- Consider using a privacy-focused analytics service (e.g., Plausible, self-hosted)
- Document telemetry format for transparency
- Add telemetry configuration to `config.toml`:

```toml
[telemetry]
enabled = false
endpoint = "https://telemetry.terraphim.ai/v1/update"
```

**Dependencies:** None (can be implemented independently)

---

### 4. Multiple Release Channels (Priority: 4)

**Description:** Support for multiple release channels (stable, beta, custom) allowing users to choose between stability and latest features.

**Channel Types:**
- **Stable:** Official releases only (default)
- **Beta:** Prereleases and release candidates
- **Custom:** Specific branch or commit hash for development/testing

**Acceptance Criteria:**
- [ ] Users can configure channel in configuration file
- [ ] Application respects channel setting when checking for updates
- [ ] Beta channel correctly identifies prereleases
- [ ] Custom channel supports branch names and commit hashes
- [ ] Channel selection persists across updates
- [ ] UI/configuration validation for channel names
- [ ] Documentation explains channel risks and benefits

**User Stories:**
- As an early adopter, I want to use the beta channel to get access to new features before they're stable
- As a developer, I want to configure custom channels to test specific branches or commits
- As a regular user, I want to stay on the stable channel for maximum reliability

**Implementation Complexity:** Medium
- Modify update checking logic to filter by channel/semver constraints
- Add channel configuration to config system
- Handle invalid channel names gracefully
- Test channel switching scenarios

**Technical Considerations:**
- Extend GitHub releases API filtering to include `prerelease=true` for beta channel
- For custom channels, fetch specific branches/commits via GitHub API
- Add channel to `config.toml`:

```toml
[update]
channel = "stable"  # stable, beta, or custom branch name
auto_update = true
check_interval_hours = 24
```

- Validate channel configuration on startup
- Consider allowing different channels per workspace/profile
- Handle cases where channel has no updates (e.g., inactive custom branch)

**Dependencies:** Phase 1 complete (extends existing release checking)

---

### 5. Windows Support (Priority: 1)

**Description:** Full Windows platform support including platform-specific paths, service installation, permissions, and UAC elevation handling.

**Platform-Specific Requirements:**
- **Paths:** Use Windows-appropriate directories (Program Files, AppData, Registry)
- **Service Installer:** Windows Service API or Task Scheduler integration
- **Permissions:** Handle Windows security descriptors and ACLs
- **UAC:** Graceful elevation for update operations requiring admin privileges

**Acceptance Criteria:**
- [ ] Application installs correctly to Windows-appropriate directories
- [ ] Updates work on Windows 10/11
- [ ] Service/daemon equivalent works on Windows (Task Scheduler or Windows Service)
- [ ] Permissions handled correctly for user-level and system-level installs
- [ ] UAC prompts only when necessary and are clearly explained
- [ ] Windows-specific configuration files use appropriate locations
- [ ] Installer/uninstaller works on Windows (MSI or NSIS)
- [ ] Testing coverage for Windows scenarios

**User Stories:**
- As a Windows user, I want the application to install and update smoothly on my operating system
- As a Windows user, I want clear permission prompts when admin access is needed
- As a Windows admin, I want to deploy the application across my organization

**Implementation Complexity:** High
- Significant platform-specific code required
- Windows security model complexity
- Testing across Windows versions
- Installer creation and maintenance

**Technical Considerations:**
- Use `dirs` crate for Windows paths (`%LOCALAPPDATA%`, `%PROGRAMDATA%`, etc.)
- Implement Windows Service using `windows-service` crate or Task Scheduler via `win32`
- Handle UAC elevation using `windows-rs` or `winapi` for `ShellExecuteW` with `runas` verb
- Registry operations for install location tracking (optional, can use config file instead)
- Code signing certificate needed for UAC prompts to be user-friendly
- Consider NSIS or WiX for installer creation
- Windows-specific error messages and troubleshooting steps
- Test on both user-level and system-level installations

**Dependencies:** Phase 1 complete (requires update mechanism, adds Windows support)

---

### 6. Delta Updates (Priority: 6)

**Description:** Implement patch-based updates that only download changed portions of the binary, reducing bandwidth and update time.

**Implementation Approaches:**
- **Binary Patching:** Use tools like `bsdiff` or `rsync` algorithm for patch generation
- **File-Level:** Download only changed files if using multi-file distribution
- **Hybrid:** Combine binary patching with file-level updates

**Acceptance Criteria:**
- [ ] Delta updates reduce download size by at least 50% for typical updates
- [ ] Patch generation is automated in release pipeline
- [ ] Patch application is reliable and rollback-safe
- [ ] Fallback to full download if delta patch fails
- [ ] Delta updates are transparent to users
- [ ] Cross-platform patching works consistently
- [ ] Performance metrics show speed improvement over full downloads

**User Stories:**
- As a user with limited bandwidth, I want updates to download faster so I can stay current without excessive data usage
- As a user, I want updates to complete quickly so I can get back to work
- As a maintainer, I want to reduce bandwidth costs for update distribution

**Implementation Complexity:** Very High
- Complex patch generation algorithm
- Binary patch application reliability
- Patch server infrastructure
- Extensive testing across update scenarios
- Rollback safety guarantees

**Technical Considerations:**
- Use `bsdiff` or similar algorithm for binary patching
- Patch generation:
  - Run as part of CI/CD release pipeline
  - Generate patches between consecutive versions
  - Store patches alongside releases (e.g., `terraphim-v1.0.0-to-v1.0.1.patch`)
- Patch application:
  - Download patch file instead of full binary
  - Apply patch to current binary
  - Verify patch integrity (checksum)
  - Fallback to full download if patch fails
- Patch server:
  - Serve patch files via GitHub releases or custom server
  - Metadata file listing available patches
- Configuration option to disable delta updates for users who prefer full downloads
- Consider using existing tools like `cargo-binstall`'s patching mechanism as reference

**Dependencies:** Phase 1 complete, Windows support (ideally)

---

### 7. GUI Update Prompts (Priority: 4)

**Description:** Desktop application UI for update confirmation, progress visualization, release notes display, and deferred update options.

**UI Components:**
- Update available notification dialog
- Update progress bar with status text
- Release notes display window
- Update confirmation dialog (Update Now, Remind Later, Skip This Version)
- Post-update summary

**Acceptance Criteria:**
- [ ] Users see a dialog when update is available
- [ ] Users can view release notes before updating
- [ ] Progress bar shows download and installation progress
- [ ] Users can defer updates (remind later or skip version)
- [ ] UI is responsive and non-blocking
- [ ] UI integrates with existing desktop application (if any)
- [ ] Keyboard shortcuts and accessibility support
- [ ] Tested on all platforms (Linux, macOS, Windows)

**User Stories:**
- As a user, I want to see what's new in an update before installing it
- As a user, I want to postpone updates if I'm in the middle of important work
- As a user, I want visual feedback during the update process

**Implementation Complexity:** Medium-High
- Depends on having a desktop application UI (Svelte frontend)
- Cross-platform UI consistency
- Integration with existing application architecture
- Responsive design and accessibility

**Technical Considerations:**
- Use existing Svelte frontend for desktop UI (from `/desktop` directory)
- Implement update dialog as a modal or separate window
- Backend API endpoints for:
  - Update status (checking, available, downloading, installing)
  - Download progress
  - Release notes content
  - Update actions (start, defer, skip)
- Real-time progress updates via WebSocket or polling
- Store update preferences (remind me later time, skipped versions)
- Use existing Bulma CSS framework for UI styling
- Ensure UI works on all supported desktop platforms
- Consider system tray integration for update status indicator

**Dependencies:** Phase 1 complete, Desktop Notifications (for integration)

---

## Implementation Priority

1. **Windows Support** (Priority 1) - Critical for platform coverage
2. **Background Daemon** (Priority 2) - Major user experience improvement
3. **Desktop Notifications** (Priority 3) - User visibility and convenience
4. **Multiple Release Channels** (Priority 4) - Developer/early adopter value
5. **GUI Update Prompts** (Priority 4) - Better user control and experience
6. **Update Telemetry** (Priority 5) - Reliability insights (nice to have)
7. **Delta Updates** (Priority 6) - Performance optimization (complex)

## Out of Scope

- Automatic application restart after update (requires explicit user action)
- Rollback to previous version (users can manually reinstall older version)
- Update dependency management (updates only the main binary, not dependencies)
- Signed updates (code signing is separate concern, though recommended for Windows)
- Peer-to-peer update distribution
- Update scheduling based on time windows (e.g., "only update at night")
- A/B testing of update mechanisms
- Mobile platform support (iOS, Android)

## Dependencies on Phase 1

All features depend on the Phase 1 auto-update foundation being complete:
- GitHub releases API integration
- Version comparison logic
- Download and verification mechanisms
- Self-replacement mechanism
- Configuration system for update settings

Phase 1 provides the core update lifecycle that these features enhance and extend.

## Questions for Discussion

1. **Telemetry Provider:** Should we host our own telemetry endpoint or use a third-party service? If third-party, which one (Plausible, Matomo, etc.)?

2. **Update Verification:** Should Windows code signing be required before enabling auto-update on Windows? How will we manage signing certificates?

3. **Background Service Frequency:** What should be the default check interval for the background daemon? Daily? Weekly? Configurable?

4. **Delta vs. Full Updates:** Given the complexity, should we pursue delta updates or invest that effort in other features first?

5. **Windows Service Type:** Should the Windows background component be a Windows Service or a Task Scheduler task? Task Scheduler is easier but less robust.

6. **Channel Permissions:** Should certain channels (e.g., beta) require explicit user acknowledgment of risks?

7. **GUI Integration:** If there's no desktop GUI (CLI-only usage), should GUI prompts still be implemented as optional feature?

8. **Rollback Safety:** What is the acceptable risk level for update failures? Should we implement backup/restore as safety mechanism?

9. **Telemetry Opt-In:** Should we prompt users to enable telemetry on first run or keep it strictly opt-in via configuration?

10. **Testing Infrastructure:** What level of automated testing is required for each platform? Can we use GitHub Actions for Windows/macOS testing?

## Related Issues

- Phase 1 implementation: (link to Phase 1 issue)
- Windows support: (create separate issue if needed)
- Desktop application UI: (link to related UI issues)

## Tasks Breakdown

- [ ] Design and document telemetry schema
- [ ] Implement Windows-specific path handling
- [ ] Create systemd service unit files for Linux
- [ ] Create launchd agent plist for macOS
- [ ] Create Windows Task Scheduler task or Windows Service
- [ ] Integrate `notify-rust` for desktop notifications
- [ ] Implement channel filtering in update checking logic
- [ ] Add channel configuration to config.toml
- [ ] Implement GUI update dialogs in Svelte frontend
- [ ] Create backend API endpoints for GUI update management
- [ ] Design delta patch generation and application system
- [ ] Implement telemetry collection and transmission
- [ ] Add telemetry opt-in/opt-out controls
- [ ] Create Windows installer (MSI or NSIS)
- [ ] Implement UAC elevation handling
- [ ] Add comprehensive cross-platform testing
- [ ] Update documentation with new features
- [ ] Write user guide for advanced update features

## Success Metrics

- Windows users can successfully install and update the application
- Background daemon reduces update latency by >50% compared to manual checks
- Telemetry adoption rate (if users opt in)
- Channel switching works correctly with no update conflicts
- GUI prompts improve user satisfaction (measured via feedback)
- Delta updates (if implemented) reduce download size by >50% and time by >30%
