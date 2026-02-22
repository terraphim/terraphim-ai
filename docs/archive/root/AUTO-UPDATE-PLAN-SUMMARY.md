# Auto-Update Feature - Implementation Plan Summary

## Executive Summary

**Problem:** Users currently need to manually check for updates and upgrade Terraphim binaries, leading to missed security patches, bug fixes, and new features. Manual updates are error-prone and time-consuming.

**Solution:** Implement automatic update checking and installation for both `terraphim-agent` and `terraphim-cli` binaries on Linux and macOS platforms.

**Key Decisions:**
- Daily silent checks that never interrupt active sessions
- Interactive prompts for updates (unless disabled in config)
- PGP signature verification for security
- Automatic backups before installation
- Rollback capability for quick recovery
- Full parity between CLI and agent implementations
- DeviceSettings for user configuration

**Expected Outcomes:**
- 95%+ of users stay on latest version
- Reduced support burden from outdated versions
- Faster security patch deployment
- Improved user experience with zero friction updates

## Implementation Plan Overview

### Phase 1: Core Implementation (60-80 hours)
- Update checking system with silent background runs
- PGP signature verification
- Binary installation with backup/rollback
- Interactive prompts and notifications
- DeviceSettings configuration
- Linux and macOS support
- Comprehensive testing suite

**Status:** Ready for implementation

### Phase 2: Enhanced Features (Tracked in GitHub Issue)
- Automatic updates without confirmation (opt-in)
- Update scheduling (custom intervals, specific times)
- Delta updates for faster downloads
- Update history and rollback UI
- Telemetry for update success rates
- Staged rollouts

**Status:** Future enhancement

### Platforms
- **Linux:** Native binary installation, systemd integration
- **macOS:** Native binary installation, launchd integration

### Binaries
- **terraphim-agent:** Background updates during idle periods
- **terraphim-cli:** Updates on command execution

## Key Features

### 1. Automatic Update Checks
- Daily silent checks in background
- Never interrupts active sessions
- Configurable check frequency

### 2. In-App Notifications
- Desktop notifications when updates available
- Clear version information and changelog
- Non-intrusive UI design

### 3. Interactive Prompts
- Ask before downloading (unless disabled)
- Ask before installing (unless auto-update enabled)
- Progress indicators during download/install

### 4. Auto-Install with Confirmation
- Seamless installation process
- Shows progress to user
- Provides success/failure feedback

### 5. PGP Signature Verification
- Verify binary authenticity before installation
- Prevent malicious updates
- PGP key management system

### 6. Binary Backup & Rollback
- Keep 3 previous versions as backups
- One-command rollback to previous version
- Safe update failure recovery

### 7. Full CLI/Agent Parity
- Identical update experience in both tools
- Shared update library crate
- Consistent user interface

### 8. Silent Background Checks
- Checks during idle periods
- No user notification until update available
- Respects system resources

### 9. Never Interrupt Sessions
- Checks only when safe
- Updates wait for session completion
- No forced reboots or restarts

### 10. DeviceSettings Configuration
- `auto_check_enabled`: Enable/disable automatic checks
- `auto_install_enabled`: Enable/disable automatic installation
- `check_frequency_hours`: How often to check (default 24)
- `backup_count`: Number of backup versions to keep (default 3)

## Architecture Overview

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                         User Interface                        │
│  (Desktop Notifications + Interactive Prompts)               │
└────────────────────┬────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────┐
│                    Update Manager                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ UpdateChecker│  │ Updater      │  │ RollbackMgr  │       │
│  │              │  │              │  │              │       │
│  │ - check()    │  │ - download() │  │ - backup()   │       │
│  │ - compare()  │  │ - verify()   │  │ - restore()  │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└────────────────────┬────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────┐
│                    Core Services                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ ConfigMgr    │  │ SignatureMgr  │  │ NetworkMgr   │       │
│  │              │  │              │  │              │       │
│  │ - settings   │  │ - verify()   │  │ - fetch()    │       │
│  │ - persistence│  │ - keys()     │  │ - retry()    │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└────────────────────┬────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────┐
│                    File System                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ Binary Dir   │  │ Backup Dir   │  │ Config Dir   │       │
│  │              │  │              │  │              │       │
│  │ terraphim-*  │  │ backups/v1/  │  │ settings.json│       │
│  │              │  │ backups/v2/  │  │              │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

```
1. Check Update
   ├─ Schedule: Daily (configurable)
   ├─ Action: Fetch version info from GitHub Releases
   ├─ Compare: Current version vs latest
   └─ Decision: Update available? → Yes/No

2. Notify User
   ├─ If update available: Show desktop notification
   ├─ Display: Version info, changelog, size
   └─ Prompt: "Update now?" or "Skip"

3. Download Update
   ├─ Action: Download binary + signature
   ├─ Progress: Show download progress
   └─ Verify: Check PGP signature

4. Backup Current
   ├─ Action: Move current binary to backups/
   ├─ Rotation: Keep last 3 versions
   └─ Metadata: Store version and timestamp

5. Install Update
   ├─ Action: Replace current binary with new version
   ├─ Verify: Check executable permissions
   └─ Notify: Show success message

6. Rollback (if needed)
   ├─ Action: Restore from backup
   ├─ Verify: Check binary integrity
   └─ Notify: Report rollback status
```

### Configuration Model

```rust
pub struct UpdateSettings {
    pub auto_check_enabled: bool,        // Default: true
    pub auto_install_enabled: bool,      // Default: false (Phase 2)
    pub check_frequency_hours: u32,      // Default: 24
    pub backup_count: u32,               // Default: 3
    pub check_url: String,              // Default: GitHub Releases API
    pub pgp_public_key_path: String,     // Path to trusted key
}

// Stored in DeviceSettings:
{
  "update": {
    "auto_check_enabled": true,
    "auto_install_enabled": false,
    "check_frequency_hours": 24,
    "backup_count": 3
  }
}
```

## Security & Safety

### PGP Signature Verification
- Every binary signed with project PGP key
- Verification before installation
- Prevents tampering during distribution
- Key distribution via secure channels

### Binary Backups
- Keep 3 previous versions by default
- Configurable backup count
- Automatic rotation (oldest deleted)
- Quick rollback capability

### Graceful Failure Handling
- Network failures: Silent retry later
- Download failures: Keep current binary
- Verification failures: Reject update, alert user
- Installation failures: Restore from backup

### No Session Interruption
- Checks only during idle periods
- Updates wait for active sessions to complete
- No forced restarts or reboots
- User controls timing

### Permission Checks
- Verify write permissions before starting
- Check disk space availability
- Validate binary ownership
- Prevent privilege escalation

## Testing Strategy

### Unit Tests
- UpdateChecker: Version comparison, scheduling logic
- Updater: Download progress, installation steps
- SignatureMgr: PGP verification, key handling
- RollbackMgr: Backup/restore operations
- ConfigMgr: Settings persistence, validation

### Integration Tests
- Full update flow: Check → Download → Verify → Install
- Rollback flow: Install → Rollback → Verify
- Network failures: Timeout, retry, recovery
- Permission failures: Error handling, cleanup

### Security Tests
- PGP signature tampering detection
- Malicious binary rejection
- Key rotation scenarios
- Man-in-the-middle attack prevention

### Platform-Specific Tests
- Linux: Binary installation, permissions, systemd integration
- macOS: Binary installation, permissions, launchd integration
- File paths: Platform-specific directory structures

### Rollback Tests
- Successful rollback to previous version
- Multiple backup version selection
- Rollback after failed update
- Rollback with missing backups (graceful failure)

## Risk Mitigation

### Backup Before Update
- Always backup current binary before replacement
- Keep multiple backup versions
- Automatic rotation prevents disk bloat

### Silent Network Failure Handling
- Fail gracefully without user notification
- Retry on next scheduled check
- Never leave system in broken state

### Manual Fallback Instructions
- Document manual update process
- Provide troubleshooting guide
- Include rollback commands
- Support contact information

### Rollback Capability
- One-command rollback to any backup
- Automatic fallback on installation failure
- User can try update again later

### Feature Flag for Quick Disable
- Global feature flag: `auto_updates_enabled`
- Instant disable without code deployment
- Quick rollback to manual updates

### Gradual Rollout Strategy
- Phase 1: Opt-in beta testers
- Phase 2: Gradual enablement (10%, 25%, 50%, 100%)
- Monitor metrics: Success rate, failure patterns
- Quick rollback if issues detected

## Deliverables

### Code Changes
1. **terraphim-agent** with auto-update integration
2. **terraphim-cli** with auto-update integration
3. **terraphim-update-lib** (new shared crate)
   - Update checking logic
   - Download and verification
   - Installation and rollback
   - Configuration management

### DeviceSettings Changes
- Update configuration schema
- Default values for auto-update
- Migration path from existing config

### PGP Key Management
- Key generation and distribution
- Key rotation process
- Documentation for key handling

### Rollback Support
- Backup directory structure
- Rollback commands (CLI)
- Rollback API (agent)

### Test Suite
- Unit tests (90%+ coverage)
- Integration tests
- Security tests
- Platform-specific tests

### Documentation
- User guide: Auto-update configuration
- Administrator guide: PGP key management
- Troubleshooting guide: Manual updates, rollback
- Architecture documentation

## Approval Checklist

- [ ] **Scope approved** - Core auto-update features for Phase 1
- [ ] **Architecture approved** - Component design and data flow
- [ ] **Security approach approved** - PGP verification, backups, rollback
- [ ] **Test strategy approved** - Comprehensive test coverage
- [ ] **Risk mitigation approved** - Backup, rollback, feature flags
- [ ] **Timeline approved** - 60-80 hours for Phase 1 implementation
- [ ] **Platform support approved** - Linux and macOS
- [ ] **Ready for Phase 3 implementation** - Proceed to disciplined implementation

## Next Steps

Upon approval:
1. Create detailed implementation tasks in GitHub issues
2. Set up development branch for auto-update feature
3. Begin Phase 3: Disciplined Implementation
4. Track progress with daily status updates
5. Conduct code reviews at each milestone

---

**Document Version:** 1.0
**Last Updated:** 2025-01-09
**Status:** Awaiting Approval
