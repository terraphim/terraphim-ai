# Terraphim AI Test Scenarios

## Overview

This document outlines comprehensive test scenarios for validating Terraphim AI releases across all platforms, installation methods, and use cases. These scenarios cover download, installation, update functionality, platform-specific behavior, and network/environment conditions.

## Download & Installation Testing

### Binary Download Tests

#### GitHub Release Artifact Verification
```bash
# Test Case: Verify all artifacts are downloadable
Test ID: DOWNLOAD-001
Description: Verify all release artifacts are accessible from GitHub releases
Preconditions: Release is published on GitHub
Steps:
  1. Navigate to GitHub releases page
  2. For each platform/architecture combination:
     - Attempt to download server binary
     - Attempt to download TUI binary
     - Attempt to download desktop application
  3. Verify SHA256 checksums match published values
Expected Results:
  - All artifacts download successfully
  - Checksum verification passes
  - File sizes match expectations
Priority: Critical
```

#### Platform-Specific Binary Tests
```bash
# Test Case: Binary execution verification
Test ID: DOWNLOAD-002
Description: Verify downloaded binaries execute correctly on target platforms
Preconditions: Binaries downloaded for target platform
Steps:
  1. Make binary executable (Unix systems)
  2. Run `--version` flag on server binary
  3. Run `--version` flag on TUI binary
  4. Verify version matches release tag
  5. Check for immediate runtime errors
Expected Results:
  - Version information displays correctly
  - No immediate crash or error messages
  - Binary exits cleanly
Priority: Critical
```

### Package Manager Installation Tests

#### Debian Package Installation
```bash
# Test Case: DEB package installation and removal
Test ID: PKG-DEB-001
Description: Test Debian package installation, configuration, and removal
Preconditions: Clean Debian/Ubuntu system
Steps:
  1. Download .deb package for target architecture
  2. Install using `sudo dpkg -i package.deb`
  3. Fix dependencies with `sudo apt-get install -f`
  4. Verify binary is in PATH
  5. Test basic functionality
  6. Remove package with `sudo apt-get remove package`
  7. Purge configuration with `sudo apt-get purge package`
Expected Results:
  - Package installs without dependency conflicts
  - Binary available in PATH
  - Service starts automatically (if applicable)
  - Clean removal without leftover files
Priority: High
```

#### RPM Package Installation
```bash
# Test Case: RPM package installation and removal
Test ID: PKG-RPM-001
Description: Test RPM package installation and removal on RHEL-based systems
Preconditions: Clean RHEL/CentOS/Fedora system
Steps:
  1. Download .rpm package
  2. Install using `sudo rpm -i package.rpm`
  3. Verify package registration with `rpm -q package`
  4. Test basic functionality
  5. Remove package with `sudo rpm -e package`
Expected Results:
  - Clean installation without dependency conflicts
  - Package registered correctly
  - Functionality works as expected
  - Clean removal
Priority: High
```

#### Homebrew Installation
```bash
# Test Case: Homebrew formula installation
Test ID: PKG-HOMEBREW-001
Description: Test Homebrew installation on macOS and Linux
Preconditions: Homebrew installed, tap added
Steps:
  1. Install with `brew install terraphim-ai`
  2. Verify installation with `brew list terraphim-ai`
  3. Test basic functionality
  4. Update with `brew upgrade terraphim-ai`
  5. Uninstall with `brew uninstall terraphim-ai`
Expected Results:
  - Clean installation from formula
  - All components installed correctly
  - Update process works
  - Complete removal
Priority: High
```

### Docker Image Tests

#### Multi-Architecture Docker Tests
```bash
# Test Case: Docker image pull and execution
Test ID: DOCKER-001
Description: Test Docker images across all supported architectures
Preconditions: Docker environment available
Steps:
  1. Pull latest image: `docker pull ghcr.io/terraphim/terraphim-server:latest`
  2. Pull versioned image: `docker pull ghcr.io/terraphim/terraphim-server:v{version}`
  3. Run container: `docker run -d --name terraphim-test ghcr.io/terraphim/terraphim-server:latest`
  4. Verify container starts: `docker ps`
  5. Test API endpoint: `curl http://localhost:8080/health`
  6. Stop and remove container
Expected Results:
  - Images pull successfully for all architectures
  - Container starts without errors
  - API responds correctly
  - Clean container lifecycle management
Priority: Critical
```

#### Docker Compose Integration
```bash
# Test Case: Docker Compose deployment
Test ID: DOCKER-002
Description: Test multi-container setup using docker-compose
Preconditions: docker-compose available
Steps:
  1. Create docker-compose.yml with terraphim services
  2. Run `docker-compose up -d`
  3. Verify all services start
  4. Test inter-service communication
  5. Check persistent data volumes
  6. Stop with `docker-compose down`
Expected Results:
  - All services start correctly
  - Network connectivity established
  - Data persistence works
  - Clean shutdown
Priority: Medium
```

### Source Build Installation Tests

#### Cargo Build Tests
```bash
# Test Case: Source compilation from git tag
Test ID: BUILD-001
Description: Test building from source code for each platform
Preconditions: Rust toolchain installed
Steps:
  1. Clone repository: `git clone https://github.com/terraphim/terraphim-ai.git`
  2. Checkout release tag: `git checkout v{version}`
  3. Build workspace: `cargo build --release`
  4. Verify binaries in target/release/
  5. Test basic functionality
Expected Results:
  - Compilation completes without errors
  - All binaries generated
  - Binaries execute correctly
  - Performance comparable to pre-built binaries
Priority: Medium
```

#### Feature Flag Compilation
```bash
# Test Case: Build with different feature combinations
Test ID: BUILD-002
Description: Test compilation with various feature flags
Preconditions: Source code available
Steps:
  1. Build with default features: `cargo build --release`
  2. Build with minimal features: `cargo build --release --no-default-features`
  3. Build with specific features: `cargo build --release --features "openrouter,mcp-rust-sdk"`
  4. Test each build variant functionality
Expected Results:
  - All feature combinations compile successfully
  - Feature-specific functionality works
  - No unused features cause issues
Priority: Medium
```

### Installation Script Validation

#### One-Line Installation Script
```bash
# Test Case: Automated installation script
Test ID: SCRIPT-001
Description: Test the one-line installation script on clean systems
Preconditions: Clean system with curl/sh
Steps:
  1. Run: `curl -fsSL https://github.com/terraphim/terraphim-ai/releases/latest/download/install.sh | sh`
  2. Verify installation completes
  3. Check binary locations and permissions
  4. Test basic functionality
  5. Verify no system conflicts
Expected Results:
  - Script completes without errors
  - All components installed correctly
  - Permissions set appropriately
  - No interference with existing software
Priority: High
```

## Update Functionality Testing

### Auto-Updater Tests (Tauri Desktop)

#### Automatic Update Detection
```bash
# Test Case: Update notification and download
Test ID: UPDATE-AUTO-001
Description: Test automatic update detection and download for desktop app
Preconditions: Previous version installed, newer version available
Steps:
  1. Launch desktop application
  2. Wait for update check interval or trigger manually
  3. Verify update notification appears
  4. Confirm update download starts
  5. Monitor download progress
  6. Verify update applies correctly
  7. Test application functionality post-update
Expected Results:
  - Update detected promptly
  - Clear user notification
  - Smooth download and installation
  - Application restarts successfully
  - User data preserved
Priority: Critical
```

#### Manual Update Workflow
```bash
# Test Case: Manual update initiation
Test ID: UPDATE-MANUAL-001
Description: Test user-initiated update process
Preconditions: Application with update available
Steps:
  1. Open application settings/check for updates
  2. Manually trigger update check
  3. Download and install update
  4. Verify application restarts
  5. Test all functionality
Expected Results:
  - Manual check works reliably
  - Update process completes successfully
  - No user data loss
  - Application functions correctly
Priority: High
```

### Version Compatibility Tests

#### Backward Compatibility
```bash
# Test Case: Configuration file compatibility
Test ID: COMPAT-CONFIG-001
Description: Test new version with old configuration files
Preconditions: Configuration files from previous version
Steps:
  1. Install new version
  2. Copy configuration from previous version
  3. Start application/server
  4. Verify configuration is read correctly
  5. Test all configured features
Expected Results:
  - Configuration migrates successfully
  - No data loss or corruption
  - All features work as expected
  - Deprecation warnings if applicable
Priority: High
```

#### API Compatibility
```bash
# Test Case: Client-server API compatibility
Test ID: COMPAT-API-001
Description: Test API compatibility between versions
Preconditions: Mixed version components
Steps:
  1. Run server with different version than client
  2. Test all API endpoints
  3. Verify error handling for incompatibilities
  4. Document supported version ranges
Expected Results:
  - Compatible versions work seamlessly
  - Clear error messages for incompatibilities
  - Graceful degradation where possible
  - Comprehensive compatibility documentation
Priority: High
```

### Rollback Scenarios

#### Update Failure Recovery
```bash
# Test Case: Failed update rollback
Test ID: ROLLBACK-001
Description: Test rollback when update fails mid-process
Preconditions: Application update interrupted
Steps:
  1. Start update process
  2. Simulate failure (network loss, power off, etc.)
  3. Restart system/application
  4. Verify previous version still functional
  5. Test data integrity
Expected Results:
  - Previous version remains functional
  - No data corruption
  - User can retry update
  - Clear status indicators
Priority: Critical
```

## Platform-Specific Testing

### Linux Platform Testing

#### Distribution Compatibility
```bash
# Test Case: Multiple Linux distributions
Test ID: LINUX-DISTRO-001
Description: Test across major Linux distributions
Preconditions: Various Linux environments
Steps:
  1. Test on Ubuntu 20.04, 22.04 LTS
  2. Test on Debian 11, 12
  3. Test on Fedora 37, 38
  4. Test on CentOS/RHEL 8, 9
  5. Test on Arch Linux
  6. Test each distribution's package manager
Expected Results:
  - Installation works on all distributions
  - Package manager integration correct
  - Service management works
  - Consistent behavior across distributions
Priority: High
```

#### Library Dependency Testing
```bash
# Test Case: System library compatibility
Test ID: LINUX-DEPS-001
Description: Test with various system library versions
Preconditions: Different library environments
Steps:
  1. Test with minimal library versions
  2. Test with latest stable libraries
  3. Test with mixed library versions
  4. Verify dynamic linking works correctly
  5. Test static linking where applicable
Expected Results:
  - No library conflicts
  - Graceful handling of version differences
  - Clear error messages for missing dependencies
  - Robust dependency resolution
Priority: Medium
```

### macOS Platform Testing

#### Intel vs Apple Silicon
```bash
# Test Case: Universal binary functionality
Test ID: MACOS-ARCH-001
Description: Test on both Intel and Apple Silicon Macs
Preconditions: Access to both architectures
Steps:
  1. Install on Intel Mac
  2. Install on Apple Silicon Mac
  3. Test universal binary compatibility
  4. Verify Rosetta 2 functionality if needed
  5. Compare performance between architectures
Expected Results:
  - Native execution on both architectures
  - Universal binary works correctly
  - Consistent behavior across platforms
  - Performance optimized for each architecture
Priority: Critical
```

#### Gatekeeper and Notarization
```bash
# Test Case: macOS security features
Test ID: MACOS-SECURITY-001
Description: Test Gatekeeper, notarization, and code signing
Preconditions: macOS with default security settings
Steps:
  1. Download and run application
  2. Verify Gatekeeper allows execution
  3. Check notarization status
  4. Test code signature verification
  5. Test with modified security settings
Expected Results:
  - Application runs without warnings
  - Code signature validates
  - Notarization passes
  - No security blockages
Priority: Critical
```

### Windows Platform Testing

#### Installer Types
```bash
# Test Case: Windows installer functionality
Test ID: WINDOWS-INSTALLER-001
Description: Test both MSI and NSIS installers
Preconditions: Clean Windows environment
Steps:
  1. Test MSI installer
  2. Test NSIS installer
  3. Verify Windows registry entries
  4. Test uninstall process
  5. Verify no leftover files/registry entries
Expected Results:
  - Both installers work correctly
  - Clean installation and uninstallation
  - Proper Windows integration
  - No system contamination
Priority: High
```

#### Antivirus and Security Software
```bash
# Test Case: Third-party security software compatibility
Test ID: WINDOWS-SECURITY-001
Description: Test with various antivirus/security software
Preconditions: Windows with security software
Steps:
  1. Test with Windows Defender
  2. Test with common third-party antivirus
  3. Verify no false positives
  4. Test application functionality with real-time protection
  5. Test network communication through security software
Expected Results:
  - No detection as malware
  - Clear explanation if flagged
  - Functionality not impacted
  - Network access works correctly
Priority: High
```

## Network & Environment Testing

### Offline Installation Scenarios

#### Complete Offline Installation
```bash
# Test Case: Installation without internet connectivity
Test ID: OFFLINE-001
Description: Test installation when system has no internet access
Preconditions: No internet connection, installation media available
Steps:
  1. Download all required packages/files beforehand
  2. Disconnect from internet
  3. Attempt installation
  4. Verify all components work
  5. Test functionality without external dependencies
Expected Results:
  - Installation completes successfully
  - All features work offline where applicable
  - Clear indication of network-dependent features
  - No unexpected failures
Priority: Medium
```

### Proxy and Firewall Testing

#### Corporate Proxy Environment
```bash
# Test Case: Installation through corporate proxy
Test ID: PROXY-001
Description: Test installation and updates through HTTP/HTTPS proxies
Preconditions: Corporate proxy environment
Steps:
  1. Configure system proxy settings
  2. Attempt download and installation
  3. Test update process through proxy
  4. Test authentication with proxy
  5. Verify SSL certificate handling
Expected Results:
  - Installation works through proxy
  - Update process functions correctly
  - Authentication works as expected
  - No certificate errors
Priority: Medium
```

#### Firewall Restrictions
```bash
# Test Case: Installation with restrictive firewalls
Test ID: FIREWALL-001
Description: Test with various firewall configurations
Preconditions: Configurable firewall environment
Steps:
  1. Test with default firewall settings
  2. Test with restrictive outbound rules
  3. Test required ports for functionality
  4. Test fallback mechanisms
  5. Verify clear error messages for blocked connections
Expected Results:
  - Installation succeeds with default settings
  - Clear guidance for firewall configuration
  - Graceful handling of blocked connections
  - Informative error messages
Priority: Medium
```

### Clean vs Upgrade Installation

#### Fresh Installation
```bash
# Test Case: Installation on clean system
Test ID: FRESH-INSTALL-001
Description: Test installation on system without previous versions
Preconditions: Clean system without Terraphim AI
Steps:
  1. Verify no previous installation exists
  2. Perform new installation
  3. Test all default configurations
  4. Verify default file locations
  5. Test first-run experience
Expected Results:
  - Clean installation without conflicts
  - Sensible default configurations
  - Intuitive first-run experience
  - No leftover files from previous versions
Priority: High
```

#### Upgrade Installation
```bash
# Test Case: Upgrade from previous version
Test ID: UPGRADE-INSTALL-001
Description: Test upgrade installation from previous major/minor versions
Preconditions: Previous version installed with user data
Steps:
  1. Install previous version with configuration/data
  2. Perform upgrade to new version
  3. Verify configuration migration
  4. Test data preservation
  5. Verify all functionality works
Expected Results:
  - Seamless upgrade process
  - All user data preserved
  - Configuration migrates correctly
  - No functionality regression
Priority: Critical
```

## Test Execution Framework

### Automation Strategy

#### Continuous Integration Tests
- Automated binary download and verification
- Package installation testing in containerized environments
- Docker image testing across architectures
- Basic functionality smoke tests

#### Manual Testing Requirements
- Desktop application UI testing
- Platform-specific installation verification
- Real-world network environment testing
- User experience validation

### Test Environment Setup

#### Virtual Machine Templates
- Standardized VM images for each platform
- Automated VM provisioning for testing
- Snapshot management for test isolation
- Automated cleanup between test runs

#### Container Testing
- Multi-architecture Docker testing
- Package installation in containers
- Network simulation capabilities
- Resource constraint testing

### Reporting and Tracking

#### Test Result Categories
- Pass: All expected results achieved
- Fail: Critical functionality broken
- Warn: Minor issues or non-critical problems
- Skip: Test not applicable to environment

#### Bug Triage Priority
- Blocker: Prevents release
- Critical: Major functionality broken
- High: Significant impact on users
- Medium: Workaround available
- Low: Minor cosmetic or documentation issues

This comprehensive test scenario document provides the foundation for validating Terraphim AI releases across all platforms, installation methods, and use cases, ensuring reliable and high-quality releases for all users.