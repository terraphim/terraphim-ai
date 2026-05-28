# Implementation Plan: Install Zig for Cross-Platform CI Builds

**Status**: Draft
**Research Doc**: `.docs/research-zig-ci-installation.md`
**Author**: AI Agent (Claude Code)
**Date**: 2026-05-27
**Estimated Effort**: 2 hours

## Overview

### Summary
Enable zig for all CI build targets by:
1. Adding `Cross.toml` with pre-build commands to install zig in cross containers
2. Adding zig installation step for Windows builds
3. Fixing Docker workspace member path issue

### Approach
Use cross's built-in `pre-build` feature to install zig in Docker containers, and standard package managers for Windows.

### Scope

**In Scope:**
- Create `Cross.toml` with zig pre-build for musl targets
- Add Windows zig install step in workflow
- Fix Docker workspace member issue
- Test the complete workflow

**Out of Scope:**
- Replacing zlob with pure-Rust alternative
- Forking cross Docker images
- Custom Docker image builds
- Fixing crates.io publishing issue (separate problem)

**Avoid At All Cost:**
- Hardcoding specific zig versions (use latest or env var)
- Complex custom Dockerfiles
- Breaking existing macOS builds

## Architecture

### Component Diagram

```
GitHub Actions Workflow
├── Linux musl builds (cross)
│   └── Cross.toml → pre-build: apt-get install zig
│       └── Container now has zig → build succeeds
├── Windows build
│   └── Workflow step: choco install zig
│       └── Runner now has zig → build succeeds
├── macOS builds
│   └── Already has zig → no changes
└── Docker builds
    └── Fix workspace member path → build succeeds
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Use `pre-build` in Cross.toml | Official cross feature, minimal complexity | Custom Docker images |
| Use apt-get for Linux containers | Debian-based containers, most reliable | Download from ziglang.org |
| Use chocolatey for Windows | Standard on GitHub-hosted runners | Scoop, manual download |
| Fix Docker workspace issue | Simple path fix in Dockerfile | Restructure workspace |

### Simplicity Check

> "What if this could be easy?"

The simplest solution:
1. One `Cross.toml` file with 3 pre-build lines
2. One workflow step for Windows
3. One Dockerfile edit

**Senior Engineer Test**: Is this overcomplicated? No - it's using built-in features.

## File Changes

### New Files
| File | Purpose |
|------|---------|
| `Cross.toml` | cross configuration with zig pre-build |

### Modified Files
| File | Changes |
|------|---------|
| `.github/workflows/release-comprehensive.yml` | Add Windows zig install, fix Docker issue |
| `Dockerfile` or `.github/workflows/docker-multiarch.yml` | Fix workspace member path |

## Implementation Steps

### Step 1: Create Cross.toml
**File:** `Cross.toml`
**Description:** Configure cross to install zig before building musl targets

```toml
[target.aarch64-unknown-linux-musl]
pre-build = [
    "apt-get update && apt-get install -y snapd",
    "snap install zig --classic --beta",
]

[target.x86_64-unknown-linux-musl]
pre-build = [
    "apt-get update && apt-get install -y snapd",
    "snap install zig --classic --beta",
]

[target.armv7-unknown-linux-musleabihf]
pre-build = [
    "apt-get update && apt-get install -y snapd",
    "snap install zig --classic --beta",
]
```

Wait - snap may not work in Docker containers. Let me use a different approach.

Better approach - download zig directly:
```toml
[target.aarch64-unknown-linux-musl]
pre-build = [
    "apt-get update && apt-get install -y curl xz-utils",
    "curl -L https://ziglang.org/builds/zig-linux-x86_64-0.16.0-dev.1234+abcdef.tar.xz -o /tmp/zig.tar.xz",
    "tar -xf /tmp/zig.tar.xz -C /usr/local --strip-components=1",
]
```

Actually, the simplest and most reliable approach is to use the official zig download:

```toml
[target.aarch64-unknown-linux-musl]
pre-build = [
    "apt-get update && apt-get install -y wget tar xz-utils",
    "wget -q https://ziglang.org/download/0.13.0/zig-linux-x86_64-0.13.0.tar.xz -O /tmp/zig.tar.xz",
    "tar -xf /tmp/zig.tar.xz -C /tmp",
    "cp /tmp/zig-linux-x86_64-0.13.0/zig /usr/local/bin/zig",
    "chmod +x /usr/local/bin/zig",
]
```

But we want to avoid hardcoding versions. Let me check if we can use the latest version or apt.

Actually, for Ubuntu/Debian, there's a simpler way - use the official zig PPA or just download the latest stable version.

Let me use a more robust approach that works with any version:

```toml
[target.aarch64-unknown-linux-musl]
pre-build = [
    "apt-get update && apt-get install -y curl tar xz-utils",
    "curl -s https://ziglang.org/download/index.json | grep -o 'https://.*zig-linux-x86_64-[^\"]*\\.tar\\.xz' | head -1 | xargs -I {} curl -L {} -o /tmp/zig.tar.xz",
    "tar -xf /tmp/zig.tar.xz -C /tmp",
    "cp /tmp/zig-*/zig /usr/local/bin/zig",
    "chmod +x /usr/local/bin/zig",
    "zig version",
]
```

This dynamically fetches the latest zig version.

**Tests:** Build locally with cross
**Estimated:** 30 minutes

### Step 2: Add Windows zig installation
**File:** `.github/workflows/release-comprehensive.yml`
**Description:** Add step to install zig on Windows runner

```yaml
- name: Install zig (required by zlob dependency)
  if: contains(matrix.target, 'windows')
  run: |
    choco install zig
    zig version
```

**Tests:** Verify Windows build succeeds
**Estimated:** 15 minutes

### Step 3: Fix Docker workspace issue
**File:** `.github/workflows/docker-multiarch.yml` or Dockerfile
**Description:** The Docker build fails because it doesn't copy `infrastructure/` but workspace expects it.

Options:
1. Add `infrastructure/` to Docker COPY commands
2. Remove `infrastructure/*` from workspace members and make it a separate workspace
3. Use `--exclude infrastructure` in cargo build

The simplest fix is option 1 - copy infrastructure into the Docker context.

**Estimated:** 30 minutes

### Step 4: Test workflow
**Description:**
1. Commit changes
2. Push to main
3. Create test tag or re-run workflow
4. Verify all builds succeed

**Estimated:** 30 minutes

## Rollback Plan

If issues discovered:
1. Revert Cross.toml
2. Revert workflow changes
3. macOS builds will still work (unaffected)

## Dependencies

### New Dependencies
None

### External Tools
- curl (in cross containers)
- tar (in cross containers)
- xz-utils (in cross containers)
- chocolatey (on Windows runners)

## Performance Considerations

### Expected Impact
- Pre-build adds ~30-60 seconds per musl build
- Windows zig install adds ~30 seconds
- Total workflow time increase: ~2-3 minutes

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify cross container has internet | Pending | Test in CI |
| Verify chocolatey works on Windows | Pending | Test in CI |
| Determine correct zig version | Pending | Use latest stable |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Human approval received
