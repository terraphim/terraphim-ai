# Research Document: Install Zig for Cross-Platform CI Builds

**Status**: Draft
**Author**: AI Agent (Claude Code)
**Date**: 2026-05-27
**Reviewers**: TBD

## Executive Summary

The `zlob v1.3.3` crate (a dependency of `fff-search` → `terraphim_file_search` → `terraphim_agent`/`terraphim_server`) requires `zig` to compile native C/Zig code during its build.rs. Zig is missing from all CI runners except the self-hosted macOS runner, causing all cross-compilation Linux builds and the Windows build to fail. This prevents creating complete releases.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Blocks all non-macOS releases |
| Leverages strengths? | Yes | We already use cross for Linux builds |
| Meets real need? | Yes | v1.20.2 release blocked by missing builds |

**Proceed**: Yes - 3/3 YES

## Problem Statement

### Description
The `zlob` crate's build.rs invokes `zig` to compile C/Zig sources into a static library:
```rust
let zig = env::var("ZIG").unwrap_or_else(|_| "zig".to_string());
let zig_version = Command::new(&zig)
    .arg("version")
    .output()
    .expect("Failed to find zig. Please install zig or set the ZIG environment variable.");
```

This fails on:
- Linux cross-compilation via `cross` (Docker containers don't have zig)
- Windows GitHub-hosted runners (no zig installed)
- Docker image builds (no zig in Dockerfile)

### Impact
- All Linux musl builds fail (armv7, aarch64, x86_64)
- Windows build fails
- Docker image builds fail (separate issue - workspace path)
- Release creation blocked due to strict conditions

### Success Criteria
1. All platform builds in build-matrix succeed
2. No manual intervention required for zig installation
3. Release workflow completes and creates GitHub release

## Current State Analysis

### zlob Dependency Chain
```
zlob v1.3.3
├── fff-query-parser v0.5.1
│   └── fff-search v0.5.1
│       ├── terraphim_file_search
│       │   └── terraphim_middleware
│       │       ├── terraphim_agent
│       │       ├── terraphim_server
│       │       └── terraphim_service
```

### CI Build Matrix

| Target | Runner | use_cross | Status | Failure |
|--------|--------|-----------|--------|---------|
| x86_64-unknown-linux-gnu | [self-hosted, bigbox] | false | ✅ | - |
| x86_64-unknown-linux-musl | [self-hosted, bigbox] | true | ❌ | Missing zig in cross container |
| aarch64-unknown-linux-musl | [self-hosted, bigbox] | true | ❌ | Missing zig in cross container |
| armv7-unknown-linux-musleabihf | [self-hosted, bigbox] | true | ❌ | Missing zig in cross container |
| x86_64-apple-darwin | [self-hosted, macOS] | false | ✅ | Already has zig |
| aarch64-apple-darwin | [self-hosted, macOS] | false | ✅ | Already has zig |
| x86_64-pc-windows-msvc | windows-latest | false | ❌ | Missing zig on runner |

### How cross Works

`cross` uses Docker containers for cross-compilation. The container images are maintained by the cross-rs project and don't include zig by default.

Key configuration options from cross docs:
- `pre-build`: Run commands before building (e.g., install packages)
- `target.TARGET.pre-build`: Target-specific pre-build commands
- `CROSS_CONTAINER_OPTS`: Additional docker run arguments
- Environment passthrough: Variables starting with `CARGO_` or `CROSS_` are passed through

### Docker Build Issue (Separate)

Docker builds fail with:
```
error: failed to load manifest for workspace member `/code/infrastructure/*`
  No such file or directory (os error 2)
```

The Dockerfile doesn't copy the `infrastructure/` directory but it's listed in workspace members.

## Constraints

### Technical Constraints
1. **cross container immutability**: Can't modify published cross images directly
2. **self-hosted runner access**: bigbox and macOS runners are under our control
3. **zig installation methods vary by platform**:
   - Linux: apt, snap, or direct download
   - macOS: brew (already installed)
   - Windows: chocolatey, scoop, or direct download
4. **cross pre-build runs inside container**: Must use container's package manager

### Business Constraints
- Must not significantly increase build times
- Must be maintainable (don't fork cross images)
- Must work on both GitHub-hosted and self-hosted runners

## Vital Few (Essentialism)

### Essential Constraints
1. **zig must be available in cross containers**: Without this, Linux musl builds fail
2. **zig must be available on Windows runner**: Without this, Windows build fails
3. **Docker workspace issue must be fixed**: Without this, Docker images fail

### Eliminated from Scope
- Replacing zlob with a pure-Rust alternative (too much work, external dependency)
- Forking cross Docker images (maintenance burden)
- Building custom cross images (complexity)

## Dependencies

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| cross | 0.2.x | Low | Could use cargo-zigbuild directly |
| zlob | 1.3.3 | High | No pure-Rust alternative |
| zig | 0.16.0 | Low | Can download specific version |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| pre-build increases build time | High | Low | Cache dependencies |
| zig download fails in container | Low | High | Use apt if available |
| Windows zig install fails | Low | High | Use chocolatey as fallback |
| Docker workspace fix breaks other things | Medium | Medium | Test locally |

### Open Questions
1. Does the cross container have internet access for downloading zig? - Yes, for apt
2. What's the best way to install zig in Debian-based cross containers? - apt or direct download
3. Does the Windows runner have chocolatey? - Yes, GitHub-hosted runners do

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| cross containers are Debian-based | cross uses Ubuntu images | Medium - would need different install | Partially |
| Windows runner has chocolatey | GitHub-hosted runner defaults | Low | Yes |
| bigbox runner can install zig | Self-hosted, we control it | Low | Yes |
| zlob will always need zig | build.rs requires it | Medium - could be changed upstream | Yes |

## Research Findings

### Key Insights

1. **cross has a `pre-build` feature perfect for this**
   ```toml
   [target.aarch64-unknown-linux-musl]
   pre-build = [
       "apt-get update && apt-get install -y zig",
   ]
   ```
   This runs inside the container before cargo build.

2. **cross containers are Ubuntu/Debian based**
   The official cross images use Debian/Ubuntu, so `apt-get` is available.

3. **Windows can install zig via multiple methods**
   - Chocolatey: `choco install zig`
   - Scoop: `scoop install zig`
   - Direct download from ziglang.org

4. **Docker build fails due to workspace member path**
   The `infrastructure/*` workspace member doesn't exist in the Docker context.

5. **The ZIG environment variable can override zig path**
   zlob's build.rs checks `ZIG` env var before defaulting to `zig` command.

### Relevant Prior Art
- cross-rs documentation on pre-build: https://github.com/cross-rs/cross/wiki/Configuration
- zig installation methods: https://ziglang.org/learn/getting-started/

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Test pre-build zig install | Verify cross container can install zig | 10 minutes |
| Test Windows zig install | Verify chocolatey install works | 10 minutes |

## Recommendations

### Proceed/No-Proceed
**PROCEED** - This is blocking the v1.20.2 release.

### Scope Recommendations
1. Add `Cross.toml` with `pre-build` commands to install zig for musl targets
2. Add Windows zig installation step in workflow
3. Fix Docker workspace member issue
4. Keep the self-hosted macOS runner as-is (already works)

### Risk Mitigation Recommendations
1. Use apt-get for Debian-based cross containers (most reliable)
2. Use chocolatey for Windows (standard on GitHub runners)
3. Add fallback to direct download if apt fails

## Next Steps

If approved:
1. Create `Cross.toml` with pre-build zig installation
2. Add Windows zig install step to workflow
3. Fix Docker workspace issue
4. Test workflow on next tag

## Appendix

### zlob build.rs key section
```rust
let zig = env::var("ZIG").unwrap_or_else(|_| "zig".to_string());
let zig_version = Command::new(&zig)
    .arg("version")
    .output()
    .expect("Failed to find zig. Please install zig or set the ZIG environment variable.");
```

### Cross.toml example for zig installation
```toml
[target.aarch64-unknown-linux-musl]
pre-build = [
    "apt-get update && apt-get install -y curl",
    "curl -L https://ziglang.org/download/0.16.0/zig-linux-x86_64-0.16.0.tar.xz | tar -xJ -C /usr/local --strip-components=1",
]
```

### Windows workflow step
```yaml
- name: Install zig
  run: choco install zig
```
