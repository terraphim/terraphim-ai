# Handover Document: macOS Release Pipeline & Homebrew Publication

**Date:** 2024-12-20
**Session Focus:** Implementing macOS release artifacts and Homebrew publication
**Branch:** `main`

---

## 1. Progress Summary

### Completed This Session

| Task | Status | Commit/Resource |
|------|--------|-----------------|
| Phase 1: Disciplined Research | ✅ Complete | `.docs/research-macos-homebrew-publication.md` |
| Phase 2: Disciplined Design | ✅ Complete | `.docs/design-macos-homebrew-publication.md` |
| Apple Developer Setup Guide | ✅ Complete | `.docs/guide-apple-developer-setup.md` |
| Create `homebrew-terraphim` tap | ✅ Complete | https://github.com/terraphim/homebrew-terraphim |
| `terraphim-server.rb` formula | ✅ Complete | Builds from source |
| `terraphim-agent.rb` formula | ✅ Complete | Builds from source |
| `create-universal-macos` job | ✅ Complete | `696bdb4a` |
| Native ARM64 runner config | ✅ Complete | `[self-hosted, macOS, ARM64]` |
| `update-homebrew` job | ✅ Complete | Uses 1Password |
| Homebrew tap token validation | ✅ Complete | `34358a3a` |
| GitHub tracking issue | ✅ Complete | #375 |

### Current Implementation State

**What's Working:**
- Homebrew tap is live: `brew tap terraphim/terraphim && brew install terraphim-server`
- Workflow will create universal binaries (arm64 + x86_64) using `lipo`
- ARM64 builds run natively on M3 Pro runner
- Automated Homebrew formula updates via 1Password token

**What's Not Yet Implemented (Phase B):**
- Apple Developer enrollment not started
- Code signing not configured
- Notarization not configured
- Formulas currently build from source (no pre-built binaries until next release)

---

## 2. Technical Context

### Repository State

```
Branch: main
Latest commits:
  34358a3a feat(ci): use 1Password for Homebrew tap token
  696bdb4a feat(ci): add macOS universal binary and Homebrew automation

Untracked files (not committed):
  .claude/hooks/
  .docs/summary-*.md (init command summaries)
```

### Key Files Modified

| File | Change |
|------|--------|
| `.github/workflows/release-comprehensive.yml` | Added universal binary job, ARM64 runner, Homebrew automation |
| `.docs/research-macos-homebrew-publication.md` | Phase 1 research document |
| `.docs/design-macos-homebrew-publication.md` | Phase 2 design plan |
| `.docs/guide-apple-developer-setup.md` | Apple enrollment instructions |

### External Resources Created

| Resource | URL |
|----------|-----|
| Homebrew Tap | https://github.com/terraphim/homebrew-terraphim |
| Tracking Issue | https://github.com/terraphim/terraphim-ai/issues/375 |

### Credentials Configured

| Credential | 1Password Path | Status |
|------------|----------------|--------|
| Homebrew Tap Token | `op://TerraphimPlatform/homebrew-tap-token/token` | ✅ Validated |
| Apple Developer Cert | `op://TerraphimPlatform/apple.developer.certificate` | ❌ Not yet created |
| Apple Credentials | `op://TerraphimPlatform/apple.developer.credentials` | ❌ Not yet created |

---

## 3. Next Steps

### Immediate (Phase B - Code Signing)

1. **Enroll in Apple Developer Program**
   - URL: https://developer.apple.com/programs/enroll/
   - Cost: $99/year
   - Time: 24-48 hours for verification
   - Follow: `.docs/guide-apple-developer-setup.md`

2. **After Enrollment - Create Certificate**
   ```bash
   # On Mac, generate CSR in Keychain Access
   # Upload to developer.apple.com
   # Download and install certificate
   # Export as .p12
   ```

3. **Store Credentials in 1Password**
   - `apple.developer.certificate` with base64 + password fields
   - `apple.developer.credentials` with APPLE_TEAM_ID + APPLE_APP_SPECIFIC_PASSWORD

4. **Add `sign-and-notarize-macos` Job**
   - Template in design document
   - Uses `codesign --sign "Developer ID Application"`
   - Uses `xcrun notarytool submit`

### After Signing Pipeline Complete (Phase C)

5. **Test Full Release**
   ```bash
   git tag v1.3.0
   git push origin v1.3.0
   ```
   - Verify universal binaries created
   - Verify binaries are signed
   - Verify Homebrew formulas updated

### Cleanup (Phase D)

6. Archive old `homebrew-formulas/` directory
7. Add Homebrew badge to README
8. Document release process

---

## 4. Blockers & Risks

| Blocker | Impact | Resolution |
|---------|--------|------------|
| Apple Developer enrollment required | Cannot sign binaries | User must enroll ($99/year, 24-48h) |
| No pre-built macOS binaries in releases | Homebrew builds from source | Next release will include them |

| Risk | Mitigation |
|------|------------|
| Notarization may fail for Rust binaries | Test with `--options runtime` flag |
| Certificate expires annually | Set calendar reminder |

---

## 5. Architecture Summary

```
release-comprehensive.yml
├── build-binaries (x86_64-apple-darwin) → [self-hosted, macOS, X64]
├── build-binaries (aarch64-apple-darwin) → [self-hosted, macOS, ARM64]
├── create-universal-macos → lipo combine → [self-hosted, macOS, ARM64]
├── sign-and-notarize-macos → (NOT YET IMPLEMENTED)
├── create-release → includes universal binaries
└── update-homebrew → push to terraphim/homebrew-terraphim
```

---

## 6. Quick Reference

### Test Homebrew Tap (Current)
```bash
brew tap terraphim/terraphim
brew install terraphim-server  # Builds from source
brew install terraphim-agent   # Builds from source
```

### Trigger Release Pipeline
```bash
git tag v1.3.0
git push origin v1.3.0
```

### Verify Signing (After Phase B)
```bash
codesign --verify --deep --strict $(which terraphim_server)
spctl --assess --type execute $(which terraphim_server)
```

---

**Next Session:** Complete Apple Developer enrollment, then implement Phase B (code signing pipeline).
