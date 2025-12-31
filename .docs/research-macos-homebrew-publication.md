# Research Document: macOS Release Artifacts and Homebrew Publication

## 1. Problem Restatement and Scope

### Problem Statement
Terraphim AI currently lacks a complete macOS release pipeline. While CI/CD workflows exist for building macOS binaries, the following gaps exist:
- **No pre-built macOS binaries** in Homebrew formulas (macOS users must build from source)
- **No Homebrew tap repository** for distributing formulas
- **No code signing or notarization** for macOS binaries (Gatekeeper will block execution)
- **No universal binaries** for CLI tools (separate x86_64 and arm64 builds exist but aren't combined)
- **Placeholder Homebrew update step** in release workflow (non-functional)

### IN Scope
- macOS CLI binaries: `terraphim_server`, `terraphim-agent` (TUI), `terraphim-cli`, `terraphim-repl`
- Universal binary creation (arm64 + x86_64)
- Code signing with Developer ID certificate
- Apple notarization for Gatekeeper approval
- Homebrew tap repository creation (`homebrew-terraphim`)
- Automated formula updates on release
- Integration with existing GitHub Actions workflows

### OUT of Scope
- Tauri desktop app (.dmg) - already has separate workflow with signing
- Windows and Linux releases - already functional
- npm/PyPI package distribution - separate workflows exist
- Mac App Store distribution - not required for CLI tools

## 2. User & Business Outcomes

### User-Visible Changes
1. **One-command installation**: `brew install terraphim/tap/terraphim-server`
2. **Native M1/M2/M3 support**: Universal binaries work on all Macs without Rosetta
3. **No Gatekeeper warnings**: Signed and notarized binaries launch without security prompts
4. **Automatic updates**: `brew upgrade` keeps tools current
5. **SHA256 verification**: Checksums automatically verified by Homebrew

### Business Outcomes
1. **Lower support burden**: Fewer "app won't open" tickets
2. **Professional image**: Signed apps demonstrate enterprise-grade quality
3. **macOS market access**: Required for enterprise macOS deployments
4. **Faster onboarding**: Single command vs. manual Rust compilation

## 3. System Elements and Dependencies

### Components Involved

| Component | Location | Role | Dependencies |
|-----------|----------|------|--------------|
| `release-comprehensive.yml` | `.github/workflows/` | Builds macOS binaries | Self-hosted macOS runner |
| `publish-tauri.yml` | `.github/workflows/` | Desktop app release | 1Password for signing keys |
| `terraphim-ai.rb` | `./` (root) | Main Homebrew formula | Pre-built binaries |
| `terraphim-cli.rb` | `homebrew-formulas/` | CLI formula (Linux only) | GitHub releases |
| `terraphim-repl.rb` | `homebrew-formulas/` | REPL formula (Linux only) | GitHub releases |
| `build-macos-bundles.sh` | `scripts/` | Creates .app bundles | Rust binaries |
| `update-homebrew-checksums.sh` | `scripts/` | Updates SHA256 in formulas | Linux binaries |
| `tauri.conf.json` | `desktop/src-tauri/` | Tauri signing config | minisign key |

### Key Binaries to Publish

| Binary | Package | Description | Current Status |
|--------|---------|-------------|----------------|
| `terraphim_server` | `terraphim_server` | HTTP API server | Built in release-comprehensive.yml |
| `terraphim-agent` | `terraphim_agent` | TUI with REPL | Built in release-comprehensive.yml |
| `terraphim-cli` | N/A | CLI tool | Formula exists (Linux only) |
| `terraphim-repl` | N/A | Interactive REPL | Formula exists (Linux only) |

### External Dependencies
- **Apple Developer Account**: Required for Developer ID certificate and notarization
- **1Password**: Already used for Tauri signing keys
- **Self-hosted macOS Runner**: Currently `[self-hosted, macOS, X64]`
- **GitHub Secrets**: Will need `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_TEAM_ID`, `APPLE_APP_SPECIFIC_PASSWORD`

## 4. Constraints and Their Implications

### Business Constraints

| Constraint | Implication |
|------------|-------------|
| Apple Developer Program ($99/year) | Required for notarization; likely already have for Tauri |
| Self-hosted runner requirement | Cannot use GitHub-hosted macOS runners (cost/availability) |

### Technical Constraints

| Constraint | Implication |
|------------|-------------|
| Universal binary requirement | Must `lipo` combine arm64 + x86_64 binaries |
| Notarization requires internet | CI must have outbound access to Apple servers |
| Stapling required | Binaries must have notarization ticket stapled |
| Homebrew tap naming | Must be `homebrew-terraphim` for `brew tap terraphim/terraphim` |

### Security Constraints

| Constraint | Implication |
|------------|-------------|
| Certificate in secure storage | Must use 1Password like Tauri workflow |
| No hardcoded credentials | All secrets via GitHub Secrets + 1Password |
| Notarization audit trail | Apple records all notarized binaries |

### Operational Constraints

| Constraint | Implication |
|------------|-------------|
| Formula update automation | Must auto-commit to homebrew-terraphim repo |
| Version synchronization | Formula version must match release tag |
| SHA256 must be exact | Checksums computed from release artifacts |

## 5. Risks, Unknowns, and Assumptions

### Unknowns

| Unknown | Impact | De-risking Action |
|---------|--------|-------------------|
| Apple Developer account credentials | Critical | Confirm with owner; check 1Password |
| Self-hosted runner architecture | High | Verify if ARM runner available for native arm64 builds |
| Current Tauri signing setup | Medium | Check if Developer ID cert exists or only ad-hoc |
| Homebrew formula acceptance criteria | Low | Review Homebrew documentation |

### Assumptions

1. **ASSUMPTION**: Apple Developer Program membership is active
2. **ASSUMPTION**: Self-hosted macOS runner has Xcode command-line tools
3. **ASSUMPTION**: Cross-compilation to aarch64 works from x86_64 runner
4. **ASSUMPTION**: 1Password service account has access to signing credentials
5. **ASSUMPTION**: GitHub Actions can create commits to homebrew-terraphim repo

### Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Notarization fails for Rust binaries | Medium | High | Test with simple binary first; check entitlements |
| Self-hosted runner unavailable | Low | High | Document fallback to manual release |
| Cross-compilation fails for arm64 | Medium | Medium | Use `cargo build --target aarch64-apple-darwin` with proper SDK |
| Homebrew PR rejected | Low | Low | Follow tap conventions; don't submit to core |
| Certificate expiration | Low | High | Set calendar reminder; monitor in 1Password |

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity

1. **Multiple release workflows**: `release-comprehensive.yml`, `publish-tauri.yml`, `package-release.yml` have overlapping responsibilities
2. **Self-hosted runner constraint**: Limits parallelism and adds maintenance burden
3. **Cross-compilation matrix**: x86_64 and aarch64 builds require different configurations
4. **Signing infrastructure**: Keychain management on CI is error-prone
5. **Multiple formulas**: Separate formulas for server, TUI, CLI, REPL fragments the experience

### Simplification Opportunities

1. **Single formula with multiple binaries**: Create `terraphim` formula that installs all CLI tools
2. **Unified release workflow**: Consolidate macOS release logic into one workflow
3. **Dedicated signing job**: Create reusable signing action/job
4. **Pre-configured runner**: Ensure runner has signing tools pre-installed
5. **GitHub-hosted fallback**: Use `macos-latest` for non-signing builds, sign on self-hosted

## 7. Questions for Human Reviewer

1. **Apple Developer credentials**: Are Developer ID certificates already configured in 1Password? What is the exact vault/item path?

2. **Self-hosted runner capabilities**: Does the `[self-hosted, macOS, X64]` runner have an ARM counterpart? Can it cross-compile to aarch64?

3. **Formula organization**: Should we have one `terraphim` formula with all binaries, or separate formulas per binary?

4. **Homebrew tap repository**: Should we create `terraphim/homebrew-terraphim` now, or use an existing org structure?

5. **Signing scope**: Should we sign only binaries distributed via Homebrew, or also binaries in GitHub Releases?

6. **Notarization tolerance**: Is it acceptable to release unsigned binaries initially while signing pipeline is developed?

7. **Binary naming**: Current formulas reference `terraphim-cli` and `terraphim-repl` but release workflow builds `terraphim_server` and `terraphim-agent`. What are the canonical names?

8. **Tauri integration**: Should the Tauri desktop app be included in the Homebrew Cask, or remain download-only?

---

## Current State Summary

### What Works
- macOS binary builds (x86_64 and aarch64 separately)
- Self-hosted macOS runner infrastructure
- Tauri app signing with minisign (for auto-update)
- Linux Homebrew formulas with pre-built binaries
- Release workflow uploads binaries to GitHub Releases

### What's Missing
- Universal binary creation for CLI tools
- Code signing with Developer ID
- Apple notarization
- Homebrew tap repository
- Automated formula updates
- macOS pre-built binary URLs in formulas

### Workflow Integration Points
```
release-comprehensive.yml (existing)
  └── build-binaries job
      ├── x86_64-apple-darwin ─┐
      └── aarch64-apple-darwin ─┼── NEW: create-universal-macos job
                                └── NEW: sign-and-notarize job
                                    └── NEW: update-homebrew job
                                        └── Commits to homebrew-terraphim repo
```
