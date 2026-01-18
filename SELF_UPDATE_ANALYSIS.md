# Self-Update Feature Analysis & Implementation Plan

## Executive Summary

After analyzing the `self_update` crate from crates.io, we can significantly reduce implementation complexity by leveraging existing functionality instead of reinventing the wheel. The crate provides ~70% of what we need, allowing us to focus on value-add features.

**Estimated Effort Reduction**: 40-60 hours (down from 60-80 hours)

---

## Feature Matrix

### ✅ Provided by self_update (Use Directly)

| Feature | Implementation | Notes |
|---------|---------------|-------|
| **Update Checking** | `self_update::backends::github::Update::configure()` | Supports GitHub, releases API |
| **Version Comparison** | Internal semver (via `self_update::should_update()` - deprecated but logic exists) | Uses `semver` crate internally |
| **Download** | `self_update::Download::from_url().download_to()` | Handles retries, chunked downloads |
| **Archive Extraction** | `self_update::Extract::from_source().archive().extract_file()` | Supports .tar.gz, .zip |
| **Binary Replacement** | `self_replace::self_replace()` (re-exported) | Cross-platform, atomic |
| **Progress Display** | `indicatif` crate integration | Built-in progress bars |
| **Signature Verification** | `signatures` feature with `zipsign` crate | **Key finding: PGP already supported!** |
| **Release Info** | `Release` struct with assets, tags, notes | GitHub API integration |
| **Target Detection** | `get_target()` helper | Linux/macOS/Windows detection |

### 🔨 Build Custom (Value-Add Features)

| Feature | Purpose | Complexity |
|---------|---------|------------|
| **Configuration Management** | `UpdateConfig` struct, validation, defaults | Medium |
| **Scheduling** | Tokio-based interval scheduler, cron-like triggers | High |
| **State Persistence** | Save/load `UpdateHistory` to disk | Medium |
| **In-App Notifications** | Format and display update messages | Medium |
| **Binary Backup/Rollback** | Backup before update, restore on failure | High |
| **Interactive Prompts** | User confirmation dialogs (CLI/GUI) | Medium |
| **Platform-Specific Paths** | Linux/macOS binary locations, permissions | Low |
| **Permission Handling** | Check write permissions, elevate if needed | High |
| **Update Policies** | Auto-update, notify-only, manual modes | Medium |
| **Telemetry/Logging** | Update success/failure tracking | Low |

---

## Updated Implementation Plan

### Step 1: Setup & Configuration (4 hours)

#### Use self_update:
```toml
# terraphim_update/Cargo.toml
[dependencies]
self_update = { version = "0.41", features = ["signatures"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
anyhow = "1"
```

#### Build custom:
- `UpdateConfig` struct with update policy, schedule, backup settings
- Validation logic (URLs, version formats, intervals)
- Default configuration for development/production

```rust
pub struct UpdateConfig {
    pub repo_owner: String,
    pub repo_name: String,
    pub bin_name: String,
    pub current_version: String,
    pub update_policy: UpdatePolicy,
    pub check_interval: Duration,
    pub backup_enabled: bool,
    pub signature_verification: bool,
}
```

---

### Step 2: Update Checker (2 hours - significantly reduced)

#### Use self_update:
```rust
use self_update::backends::github::{Update, Release};

let updater = Update::configure()
    .repo_owner("terraphim")
    .repo_name("terraphim-ai")
    .bin_name("terraphim")
    .current_version("0.1.0")
    .build()?;

let releases = updater.get_latest_release()?;
let should_update = self_update::version_bump_semver(
    &current_version,
    &latest_version
);
```

#### Build custom:
- Caching logic (don't check GitHub too frequently)
- Schedule integration (check only when due)

---

### Step 3: Version Comparison (0 hours - eliminated)

**Use self_update directly**. The crate uses `semver` internally and provides:
- `version_bump_semver()` for semantic version comparison
- `version_bump_patch()`, `version_bump_minor()`, etc.

**Action**: Remove custom semver code from plan.

---

### Step 4: Downloader (1 hour - simplified)

#### Use self_update:
```rust
use self_update::Download;

Download::from_url(&release_url)
    .download_to(&temp_path)
    .show_progress(true)?;
```

#### Build custom:
- Retry logic (already in self_update but may need custom backoff)
- Resume support (if needed for large binaries)

---

### Step 5: Extractor (0 hours - eliminated)

**Use self_update directly**. Supports:
- `.tar.gz` archives
- `.zip` archives
- Automatic binary detection

```rust
use self_update::Extract;

Extract::from_source(&archive_path)
    .archive(&target_file_name)
    .extract_file(&temp_path)?;
```

**Action**: Remove custom extraction code from plan.

---

### Step 6: Signature Verification (2 hours - simplified)

#### Use self_update (with `signatures` feature):
```toml
self_update = { version = "0.41", features = ["signatures"] }
```

```rust
// self_update uses zipsign internally for signature verification
// No custom PGP implementation needed!
use self_update::backends::github::Update;

let updater = Update::configure()
    .repo_owner("terraphim")
    .repo_name("terraphim-ai")
    .build()?;

// Self_update verifies signatures automatically when signature file is present
updater.download_and_replace()?;
```

#### Build custom:
- Public key management (where to store, how to distribute)
- Signature key rotation handling
- Fallback behavior when verification fails

**Action**: Remove custom PGP/GPG implementation plan entirely.

---

### Step 7: Installer (2 hours - simplified)

#### Use self_update:
```rust
use self_replace;

self_replace::self_replace(path)?;
```

#### Build custom:
- Binary backup before replacement:
  ```rust
  fn backup_binary(path: &Path) -> Result<PathBuf> {
      let backup_path = path.with_extension("bak");
      fs::copy(path, &backup_path)?;
      Ok(backup_path)
  }
  ```
- Rollback on failure
- Permission checking

---

### Step 8: Configuration Manager (4 hours)

**Build custom** (not in self_update):
- `UpdateConfig` struct (from Step 1)
- `UpdateHistory` tracking:
  ```rust
  pub struct UpdateHistory {
      pub last_check: DateTime<Utc>,
      pub last_update: DateTime<Utc>,
      pub last_version: String,
      pub update_count: u32,
      pub failed_updates: Vec<FailedUpdate>,
  }
  ```
- Save/load to `~/.config/terraphim/update_config.toml`
- Schema validation
- Migration support

---

### Step 9: Scheduler (6 hours)

**Build custom** (not in self_update):
- Tokio-based interval scheduler:
  ```rust
  pub struct UpdateScheduler {
      config: Arc<UpdateConfig>,
      handle: Option<JoinHandle<()>>,
  }

  impl UpdateScheduler {
      pub async fn start(&mut self) -> Result<()> {
          let interval = tokio::time::interval(self.config.check_interval);
          loop {
              interval.tick().await;
              if let Err(e) = self.check_for_updates().await {
                  error!("Update check failed: {}", e);
              }
          }
      }
  }
  ```
- Graceful shutdown
- Skip if update already checked recently
- Integration with UpdateConfig

---

### Step 10: Notifier (4 hours)

**Build custom** (not in self_update):
- Format update messages:
  ```rust
  pub struct UpdateMessage {
      pub version: String,
      pub release_notes: String,
      pub action: UpdateAction,
  }

  pub enum UpdateAction {
      UpdateNow,
      UpdateLater,
      Skip,
  }
  ```
- CLI prompts:
  ```rust
  use dialoguer::Confirm;
  let confirmed = Confirm::new()
      .with_prompt("Update to v1.2.3?")
      .interact()?;
  ```
- GUI notifications (future work)

---

### Step 11: Backup & Rollback (6 hours)

**Build custom** (not in self_update):
- Pre-update backup:
  ```rust
  pub fn create_backup(binary_path: &Path) -> Result<Backup> {
      let backup_path = backup_path(binary_path)?;
      fs::copy(binary_path, &backup_path)?;
      Ok(Backup { path: backup_path })
  }
  ```
- Post-update verification (run binary, check version)
- Rollback on failure:
  ```rust
  pub fn rollback(backup: Backup, target: &Path) -> Result<()> {
      fs::copy(&backup.path, target)?;
      Ok(())
  }
  ```
- Cleanup old backups (keep last N)

---

### Step 12: Platform Paths (2 hours)

**Build custom** (not in self_update):
```rust
pub fn get_binary_path() -> Result<PathBuf> {
    if cfg!(target_os = "macos") {
        Ok(PathBuf::from("/usr/local/bin/terraphim"))
    } else if cfg!(target_os = "linux") {
        Ok(PathBuf::from("/usr/local/bin/terraphim"))
    } else if cfg!(windows) {
        // Use Windows registry or Program Files
        Ok(PathBuf::from("C:\\Program Files\\Terraphim\\terraphim.exe"))
    } else {
        bail!("Unsupported platform")
    }
}
```
- Write permission checks
- Fallback to `~/.local/bin` if no sudo access

---

### Step 13: Tests (8 hours)

**Test self_update integration:**
- Mock GitHub releases for testing
- Test signature verification with test keys
- Test rollback scenarios

**Test custom components:**
- Configuration serialization/deserialization
- Scheduler interval logic
- Backup/rollback flows
- Permission handling

---

## Recommendations: Use vs. Custom

### Use self_update for:

1. **Update checking** - GitHub API integration is robust
2. **Version comparison** - semver implementation is battle-tested
3. **Downloading** - Handles retries, progress bars
4. **Extraction** - Supports multiple archive formats
5. **Binary replacement** - Cross-platform atomic replacement
6. **Progress display** - Built-in `indicatif` integration
7. **Signature verification** - `zipsign` integration via `signatures` feature
8. **Release info** - Parsed from GitHub API

### Build custom for:

1. **Configuration** - Our specific policies, schedules, defaults
2. **Scheduling** - Tokio integration, cron-like triggers
3. **State persistence** - Update history, config files
4. **Notifications** - In-app formatting, user prompts
5. **Backup/rollback** - Safety net for failed updates
6. **Platform paths** - Our specific installation locations
7. **Permission handling** - Check and elevate privileges
8. **Update policies** - Auto-update, notify-only, manual modes

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| **self_update dependency version** | Pin to specific version, test upgrades |
| **Signature key management** | Document key rotation, embed in release |
| **Backup conflicts** | Timestamp backup files, limit count |
| **Permission issues** | Graceful fallback to user directory |
| **GitHub API rate limits** | Cache results, respect headers |
| **Rollback failures** | Verify backup integrity before update |

---

## Complexity Breakdown by Step

| Step | Use self_update | Custom Code | Hours |
|------|-----------------|-------------|-------|
| 1. Setup | ✅ Feature flags | Config struct | 4 |
| 2. Update Checker | ✅ `Update::configure()` | Caching, scheduling | 2 |
| 3. Version Comparison | ✅ Eliminated | - | 0 |
| 4. Downloader | ✅ `Download::from_url()` | Retry logic | 1 |
| 5. Extractor | ✅ Eliminated | - | 0 |
| 6. Signature Verification | ✅ `signatures` feature | Key management | 2 |
| 7. Installer | ✅ `self_replace()` | Backup, rollback | 2 |
| 8. Config Manager | ❌ | Full implementation | 4 |
| 9. Scheduler | ❌ | Full implementation | 6 |
| 10. Notifier | ❌ | Full implementation | 4 |
| 11. Backup & Rollback | ❌ | Full implementation | 6 |
| 12. Platform Paths | ❌ | Full implementation | 2 |
| 13. Tests | Mixed | Full test suite | 8 |
| **Total** | **7 steps leveraged** | **8 steps custom** | **41** |

**Effort reduction**: 33-49 hours (45-61% reduction)

---

## Implementation Order (Optimized)

1. **Phase 1: Core (8 hours)**
   - Step 1: Setup (config struct, feature flags)
   - Step 2: Update Checker (integrate self_update)
   - Step 7: Installer (self_replace, basic backup)

2. **Phase 2: Safety (8 hours)**
   - Step 11: Backup & Rollback (robust backup system)
   - Step 6: Signature Verification (key management)
   - Step 4: Downloader (custom retry logic)

3. **Phase 3: Automation (10 hours)**
   - Step 8: Config Manager (persistence)
   - Step 9: Scheduler (Tokio integration)
   - Step 10: Notifier (user prompts)

4. **Phase 4: Polish (7 hours)**
   - Step 12: Platform Paths (macOS/Linux)
   - Step 13: Tests (comprehensive suite)

5. **Phase 5: Integration (8 hours)**
   - Wire into terraphim-ai main binary
   - CLI flags for update commands
   - Documentation

**Total: 41 hours**

---

## Key Takeaways

1. **Eliminate redundant code**: Don't rebuild version comparison, downloading, extraction
2. **Leverage signature verification**: self_update's `signatures` feature uses zipsign
3. **Focus on value-add**: Build only what's missing (config, scheduling, backup, rollback)
4. **Reduce complexity**: 45-61% effort reduction
5. **Maintain safety**: Backup/rollback is critical, even with atomic replacement
6. **Test thoroughly**: Mock self_update calls for isolated testing

---

## Next Steps

1. ✅ Add `signatures` feature to `terraphim_update/Cargo.toml`
2. ✅ Create `UpdateConfig` struct
3. ✅ Implement update checker using self_update
4. ✅ Build backup/rollback system
5. ✅ Add scheduler with Tokio
6. ✅ Integrate into main binary
7. ✅ Write tests for custom components
8. ✅ Document usage and configuration
