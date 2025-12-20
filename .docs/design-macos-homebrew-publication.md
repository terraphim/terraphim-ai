# Design & Implementation Plan: macOS Release Artifacts and Homebrew Publication

## 1. Summary of Target Behavior

After implementation, the system will:

1. **Build universal macOS binaries** combining arm64 and x86_64 architectures using `lipo`
2. **Sign binaries** with Apple Developer ID certificate for Gatekeeper approval
3. **Notarize binaries** with Apple for malware scanning verification
4. **Publish to Homebrew tap** at `terraphim/homebrew-terraphim`
5. **Auto-update formulas** with correct SHA256 checksums on each release

**User experience after implementation:**
```bash
# One-time setup
brew tap terraphim/terraphim

# Install any tool
brew install terraphim/terraphim/terraphim-server
brew install terraphim/terraphim/terraphim-agent

# No Gatekeeper warnings - binaries are signed and notarized
terraphim_server --version
```

## 2. Key Invariants and Acceptance Criteria

### Invariants

| Invariant | Guarantee |
|-----------|-----------|
| Binary universality | Every macOS binary contains both arm64 and x86_64 slices |
| Signature validity | All binaries pass `codesign --verify --deep --strict` |
| Notarization status | All binaries pass `spctl --assess --type execute` |
| Formula correctness | SHA256 checksums match downloaded artifacts exactly |
| Version consistency | Formula version matches GitHub release tag |

### Acceptance Criteria

| ID | Criterion | Verification Method |
|----|-----------|---------------------|
| AC1 | `brew install terraphim/terraphim/terraphim-server` succeeds on Intel Mac | Manual test on Intel Mac |
| AC2 | `brew install terraphim/terraphim/terraphim-server` succeeds on Apple Silicon Mac | Manual test on M1/M2/M3 Mac |
| AC3 | Installed binary runs without Gatekeeper warning | Launch binary, no security dialog |
| AC4 | `file $(which terraphim_server)` shows "universal binary" | Command output verification |
| AC5 | Release workflow completes without manual intervention | GitHub Actions log review |
| AC6 | Formula SHA256 matches release artifact | `shasum -a 256` comparison |
| AC7 | `brew upgrade terraphim-server` pulls new version after release | Version comparison after upgrade |

## 3. High-Level Design and Boundaries

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                    release-comprehensive.yml                         │
├─────────────────────────────────────────────────────────────────────┤
│  ┌────────────────────────┐  ┌────────────────────────┐             │
│  │ build-binaries         │  │ build-binaries         │             │
│  │ x86_64-apple-darwin    │  │ aarch64-apple-darwin   │             │
│  │ [self-hosted,macOS,X64]│  │ [self-hosted,macOS,ARM]│             │
│  └──────────┬─────────────┘  └──────────┬─────────────┘             │
│             │                           │                            │
│             └─────────┬─────────────────┘                            │
│                       ▼                                              │
│  ┌───────────────────────────────────────┐                          │
│  │   create-universal-macos              │  NEW JOB                 │
│  │   runs-on: [self-hosted, macOS, ARM64]│  (M3 Pro)                │
│  │   - Download both artifacts           │                          │
│  │   - lipo -create universal            │                          │
│  │   - Upload universal artifact         │                          │
│  └──────────────────┬────────────────────┘                          │
│                     ▼                                                │
│  ┌───────────────────────────────────────┐                          │
│  │   sign-and-notarize-macos             │  NEW JOB                 │
│  │   runs-on: [self-hosted, macOS, ARM64]│  (M3 Pro)                │
│  │   - Import certificate from 1Password │                          │
│  │   - codesign --sign "Developer ID"    │                          │
│  │   - xcrun notarytool submit           │                          │
│  │   - Upload signed artifacts           │                          │
│  └──────────────────┬────────────────────┘                          │
│                     ▼                                                │
│  ┌───────────────────────────────────────┐                          │
│  │   create-release (existing)           │  MODIFIED                │
│  │   - Include signed macOS binaries     │                          │
│  │   - All platforms in one release      │                          │
│  └──────────────────┬────────────────────┘                          │
│                     ▼                                                │
│  ┌───────────────────────────────────────┐                          │
│  │   update-homebrew-tap                 │  NEW JOB                 │
│  │   runs-on: ubuntu-latest              │                          │
│  │   - Clone homebrew-terraphim          │                          │
│  │   - Update formula versions           │                          │
│  │   - Update SHA256 checksums           │                          │
│  │   - Commit and push                   │                          │
│  └───────────────────────────────────────┘                          │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│              terraphim/homebrew-terraphim (NEW REPO)                 │
├─────────────────────────────────────────────────────────────────────┤
│  Formula/                                                            │
│  ├── terraphim-server.rb    # Server formula with universal binary  │
│  ├── terraphim-agent.rb     # TUI formula with universal binary     │
│  └── terraphim.rb           # Meta-formula (optional, installs all) │
└─────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Changes |
|-----------|---------------|---------|
| `release-comprehensive.yml` | Orchestrates full release pipeline | Add 3 new jobs |
| `create-universal-macos` job | Combines arch-specific binaries | New |
| `sign-and-notarize-macos` job | Apple code signing and notarization | New |
| `update-homebrew-tap` job | Updates formulas in tap repository | New |
| `homebrew-terraphim` repo | Hosts Homebrew formulas | New repository |
| `scripts/sign-macos-binary.sh` | Reusable signing script | New |
| `scripts/update-homebrew-formula.sh` | Formula update script | Modify existing |

### Boundaries

**Inside this change:**
- `release-comprehensive.yml` workflow modifications
- New shell scripts for signing
- New Homebrew tap repository
- New formula files

**Outside this change (no modifications):**
- `publish-tauri.yml` - Desktop app has separate signing
- `package-release.yml` - Linux/Arch packages unchanged
- Existing Linux Homebrew formulas in `homebrew-formulas/`
- Rust source code

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|--------|-------|--------------|
| `.github/workflows/release-comprehensive.yml` | Modify | Builds separate arch binaries, placeholder Homebrew step | Adds universal binary, signing, notarization, and Homebrew update jobs | Self-hosted macOS runner, 1Password |
| `scripts/sign-macos-binary.sh` | Create | N/A | Signs and notarizes a macOS binary | Xcode CLI tools, Apple credentials |
| `scripts/update-homebrew-formula.sh` | Modify | Updates Linux checksums only | Updates macOS universal binary URL and checksum | GitHub CLI |
| `terraphim/homebrew-terraphim` (repo) | Create | N/A | Homebrew tap repository with formulas | GitHub organization access |
| `homebrew-terraphim/Formula/terraphim-server.rb` | Create | N/A | Formula for server binary | Release artifacts |
| `homebrew-terraphim/Formula/terraphim-agent.rb` | Create | N/A | Formula for TUI binary | Release artifacts |
| `1Password vault` | Modify | Tauri signing keys only | Add Apple Developer ID cert + credentials | Apple Developer account |

### New 1Password Items Required

| Item | Type | Contents |
|------|------|----------|
| `apple.developer.certificate` | Document | Developer ID Application certificate (.p12) |
| `apple.developer.certificate.password` | Password | Certificate import password |
| `apple.developer.credentials` | Login | APPLE_ID, APPLE_TEAM_ID, APPLE_APP_SPECIFIC_PASSWORD |

## 5. Step-by-Step Implementation Sequence

### Phase A: Infrastructure Setup (No Code Signing)

| Step | Action | Purpose | Deployable? |
|------|--------|---------|-------------|
| A1 | Create `terraphim/homebrew-terraphim` repository on GitHub | Establish tap location | Yes |
| A2 | Add initial `Formula/terraphim-server.rb` with source build | Basic formula structure | Yes, but builds from source |
| A3 | Add initial `Formula/terraphim-agent.rb` with source build | Basic formula structure | Yes, but builds from source |
| A4 | Test `brew tap terraphim/terraphim && brew install terraphim-server` | Verify tap works | Yes |
| A5 | Add `create-universal-macos` job to `release-comprehensive.yml` | Create universal binaries | Yes, produces unsigned universals |
| A6 | Update formulas to use pre-built universal binaries (unsigned) | Faster installation | Yes, Gatekeeper warnings expected |

### Phase B: Code Signing Pipeline

| Step | Action | Purpose | Deployable? |
|------|--------|---------|-------------|
| B1 | Store Apple Developer ID certificate in 1Password | Secure credential storage | N/A |
| B2 | Store Apple credentials (ID, Team ID, App Password) in 1Password | Notarization auth | N/A |
| B3 | Create `scripts/sign-macos-binary.sh` | Reusable signing logic | N/A (script only) |
| B4 | Add `sign-and-notarize-macos` job to workflow | Integrate signing into CI | Yes |
| B5 | Test signing with manual workflow dispatch | Verify signing works | Yes, test release only |
| B6 | Verify notarization status with `spctl` | Confirm Gatekeeper approval | Yes |

### Phase C: Homebrew Automation

| Step | Action | Purpose | Deployable? |
|------|--------|---------|-------------|
| C1 | Add GitHub PAT for homebrew-terraphim repo access | Cross-repo commits | N/A |
| C2 | Create `update-homebrew-tap` job in workflow | Automate formula updates | Yes |
| C3 | Modify `scripts/update-homebrew-formula.sh` for macOS | Handle universal binary URLs | Yes |
| C4 | Test full release cycle with tag push | End-to-end verification | Yes |
| C5 | Document installation in README | User documentation | Yes |

### Phase D: Cleanup and Polish

| Step | Action | Purpose | Deployable? |
|------|--------|---------|-------------|
| D1 | Remove placeholder `update-homebrew` step from workflow | Clean up dead code | Yes |
| D2 | Archive old `homebrew-formulas/` directory | Consolidate to tap | Yes |
| D3 | Add Homebrew badge to README | Discoverability | Yes |
| D4 | Create release checklist documentation | Operational runbook | Yes |

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location/Method |
|---------------------|-----------|---------------------|
| AC1: Intel Mac install | Manual E2E | Run on Intel Mac hardware |
| AC2: Apple Silicon install | Manual E2E | Run on M1/M2/M3 Mac hardware |
| AC3: No Gatekeeper warning | Manual E2E | First launch after install |
| AC4: Universal binary | Integration | `file` command in workflow |
| AC5: Workflow completion | Integration | GitHub Actions status |
| AC6: SHA256 match | Integration | Workflow checksum step |
| AC7: Upgrade works | Manual E2E | Version bump and upgrade test |

### Automated Verification Steps (in workflow)

```yaml
# Verify universal binary
- name: Verify universal binary
  run: |
    file artifacts/terraphim_server-universal-apple-darwin | grep -q "universal binary"

# Verify signature
- name: Verify code signature
  run: |
    codesign --verify --deep --strict artifacts/terraphim_server-universal-apple-darwin

# Verify notarization
- name: Verify notarization
  run: |
    spctl --assess --type execute artifacts/terraphim_server-universal-apple-darwin
```

## 7. Risk & Complexity Review

| Risk (from Phase 1) | Mitigation in Design | Residual Risk |
|---------------------|---------------------|---------------|
| Notarization fails for Rust binaries | Test with simple binary in Phase B5; check entitlements | May need `--options runtime` or entitlements.plist |
| Self-hosted runner unavailable | Document manual release procedure; alert on runner offline | Manual intervention required if runner down |
| Cross-compilation fails for arm64 | Existing workflow already builds aarch64 successfully | Low - already working |
| Certificate expiration | Add 1Password expiry monitoring; document renewal | Requires annual renewal attention |
| Homebrew tap push fails | Use dedicated GitHub PAT with repo scope; test in Phase C4 | May need org admin for initial setup |

### New Risks Identified

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Apple notarization service unavailable | Low | Medium | Add retry logic with exponential backoff |
| 1Password CLI rate limiting | Low | Low | Cache credentials within job |
| Formula syntax errors | Medium | Low | Test formula locally before push |
| Universal binary size too large | Low | Low | Acceptable tradeoff for compatibility |

## 8. Confirmed Decisions

### Decisions Made (2024-12-20)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Homebrew tap repository | `terraphim/homebrew-terraphim` | Follows Homebrew conventions |
| Formula organization | Separate formulas per binary | User preference for granularity |
| Signing scope | All GitHub Release binaries | Consistency across distribution channels |
| ARM runner availability | `[self-hosted, macOS, ARM64]` M3 Pro | Native arm64 builds, no cross-compilation needed |

### Runner Configuration

**Available self-hosted macOS runners:**

| Runner Label | Architecture | Use Case |
|--------------|--------------|----------|
| `[self-hosted, macOS, X64]` | Intel x86_64 | Build x86_64 binaries natively |
| `[self-hosted, macOS, ARM64]` | Apple Silicon M3 Pro | Build arm64 binaries natively |

**Updated build strategy:** Build each architecture on native hardware (no cross-compilation), then combine with `lipo` on either runner.

### Remaining Setup Required

1. **Apple Developer Program enrollment** - See `.docs/guide-apple-developer-setup.md`
2. **1Password credential storage** - After enrollment, store in `TerraphimPlatform` vault
3. **GitHub PAT for tap repo** - Create token with `repo` scope after tap creation

---

## Appendix: Formula Template

```ruby
# Formula/terraphim-server.rb
class TerraphimServer < Formula
  desc "Privacy-first AI assistant HTTP server with semantic search"
  homepage "https://github.com/terraphim/terraphim-ai"
  version "VERSION_PLACEHOLDER"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/terraphim/terraphim-ai/releases/download/vVERSION_PLACEHOLDER/terraphim_server-universal-apple-darwin"
    else
      url "https://github.com/terraphim/terraphim-ai/releases/download/vVERSION_PLACEHOLDER/terraphim_server-universal-apple-darwin"
    end
    sha256 "SHA256_PLACEHOLDER"
  end

  on_linux do
    url "https://github.com/terraphim/terraphim-ai/releases/download/vVERSION_PLACEHOLDER/terraphim_server-x86_64-unknown-linux-gnu"
    sha256 "LINUX_SHA256_PLACEHOLDER"
  end

  def install
    bin.install "terraphim_server-universal-apple-darwin" => "terraphim_server" if OS.mac?
    bin.install "terraphim_server-x86_64-unknown-linux-gnu" => "terraphim_server" if OS.linux?
  end

  service do
    run opt_bin/"terraphim_server"
    keep_alive true
    log_path var/"log/terraphim-server.log"
    error_log_path var/"log/terraphim-server-error.log"
  end

  test do
    assert_match "terraphim", shell_output("#{bin}/terraphim_server --version")
  end
end
```

---

**Do you approve this plan as-is, or would you like to adjust any part?**
