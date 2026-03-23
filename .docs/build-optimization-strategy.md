# Build Optimization Strategy: Preventing 200+ GB Storage Consumption

## Executive Summary

The Terraphim AI project is experiencing excessive disk usage during compilation, consuming over 200 GB of storage. This document provides a comprehensive optimization strategy to address this issue while maintaining build performance, debuggability, and CI/CD compatibility.

### Key Findings

1. **Multi-crate workspace**: 40+ crates in the workspace with complex dependency graphs
2. **Multiple build targets**: x86_64, aarch64, musl, and WASM targets
3. **Feature flag proliferation**: Multiple feature combinations creating exponential compilation units
4. **CI/CD inefficiencies**: Suboptimal caching strategies and redundant builds
5. **Missing optimization configurations**: Default Cargo profiles not optimized for disk usage

### Expected Impact

| Optimization Area | Expected Savings | Priority |
|-------------------|------------------|----------|
| Intermediate Artifact Cleanup | 30-50 GB | High |
| Incremental Compilation Tuning | 20-30 GB | High |
| Dependency Caching | 40-60 GB | High |
| Profile Optimizations | 15-25 GB | Medium |
| Workspace Consolidation | 10-20 GB | Medium |
| **Total Potential Savings** | **115-185 GB** | - |

---

## 1. Intermediate Artifact Bloat Reduction

### Current State Analysis

The `target/` directory structure shows significant bloat from:
- Multiple target architectures (x86_64, aarch64, armv7, musl variants)
- Debug and release artifacts coexisting
- Feature flag combinations creating redundant builds
- No automated cleanup of old artifacts

### Recommendations

#### 1.1 Implement Target Directory Cleanup Automation

**Priority: HIGH**

Create a cleanup script and CI integration:

```bash
#!/bin/bash
# scripts/cleanup-target.sh

# Keep only the last 3 builds per target
find target -name "*.rlib" -type f -mtime +7 -delete
find target -name "*.rmeta" -type f -mtime +7 -delete

# Remove old incremental compilation data
find target -path "*/incremental/*" -type d -mtime +3 -exec rm -rf {} +

# Clean up dead code artifacts
find target -name "*.o" -type f -delete
find target -name "*.d" -type f -mtime +1 -delete
```

**Implementation:**
- Add to `.github/workflows/ci-main.yml` as a scheduled job
- Run nightly on self-hosted runners
- Expected savings: 20-30 GB per runner

#### 1.2 Configure Target Directory Sharing

**Priority: HIGH**

Modify `.cargo/config.toml`:

```toml
[build]
# Use a shared target directory for all workspace members
target-dir = "target"

# Enable sparse registry for faster dependency resolution
[registries.crates-io]
protocol = "sparse"
```

#### 1.3 Implement Artifact Retention Policies

**Priority: MEDIUM**

Update CI workflows to implement retention:

```yaml
# .github/workflows/ci-main.yml
- name: Cleanup old artifacts
  if: github.event_name == 'schedule'
  run: |
    # Keep only artifacts from last 5 runs
    gh run list --limit 20 --json databaseId | \
      jq -r '.[5:].databaseId' | \
      xargs -I {} gh run delete {}
```

---

## 2. Incremental Compilation Configuration

### Current State Analysis

The workspace currently has:
- `CARGO_INCREMENTAL=0` in CI (disables incremental compilation entirely)
- No `split-debuginfo` configuration
- Default `codegen-units` settings

### Recommendations

#### 2.1 Optimize Incremental Compilation

**Priority: HIGH**

Update workspace `Cargo.toml`:

```toml
[profile.dev]
# Enable incremental with optimized settings
incremental = true
codegen-units = 256
# Split debug info to reduce object file sizes
split-debuginfo = "unpacked"

[profile.test]
incremental = true
codegen-units = 256
split-debuginfo = "unpacked"

[profile.release]
# Already optimized but add split-debuginfo
lto = false
codegen-units = 1
opt-level = 3
split-debuginfo = "packed"
```

**Rationale:**
- `split-debuginfo = "unpacked"` for dev/test reduces intermediate object sizes by 30-40%
- `codegen-units = 256` for dev builds enables better parallelism without excessive disk usage
- `split-debuginfo = "packed"` for release creates `.dwp` files, reducing binary sizes

#### 2.2 Configure CI-Specific Profiles

**Priority: HIGH**

Create CI-optimized profiles in `Cargo.toml`:

```toml
[profile.ci]
inherits = "dev"
incremental = false  # Disable in CI for reproducibility
codegen-units = 16   # Balance between speed and disk usage
split-debuginfo = "off"  # No debug info needed in CI

[profile.ci-release]
inherits = "release"
lto = "thin"         # Faster than full LTO, smaller than none
codegen-units = 8    # Better optimization than 1, less disk than default
```

Update CI workflows:
```yaml
env:
  CARGO_PROFILE_DEV_DEBUG: 0  # Disable debug info in CI
  CARGO_PROFILE_TEST_DEBUG: 0
```

---

## 3. Dependency Caching Optimization

### Current State Analysis

Current CI caching strategy:
- Caches `~/.cargo/registry` and `~/.cargo/git`
- Caches `target` directory per matrix job
- No sccache or distributed caching
- Multiple redundant dependency downloads across jobs

### Recommendations

#### 3.1 Implement sccache for Distributed Caching

**Priority: HIGH**

Add sccache configuration to CI:

```yaml
# .github/workflows/ci-main.yml
env:
  SCCACHE_GHA_ENABLED: "true"
  RUSTC_WRAPPER: "sccache"

jobs:
  rust-build:
    steps:
      - name: Run sccache-cache
        uses: mozilla-actions/sccache-action@v0.0.3
        
      - name: Build with sccache
        run: |
          sccache --start-server
          cargo build --release --workspace
          sccache --stop-server
```

**Expected Impact:**
- 40-60% reduction in compilation time
- 30-50% reduction in disk usage from shared cache hits
- Better cache utilization across matrix jobs

#### 3.2 Optimize Cargo Registry Caching

**Priority: HIGH**

Update cache configuration:

```yaml
- name: Cache Cargo registry
  uses: actions/cache@v4
  with:
    path: |
      ~/.cargo/registry/cache
      ~/.cargo/registry/index
      ~/.cargo/git/db
    key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
    restore-keys: |
      ${{ runner.os }}-cargo-registry-

- name: Cache Cargo build
  uses: actions/cache@v4
  with:
    path: target
    key: ${{ runner.os }}-cargo-build-${{ hashFiles('**/Cargo.lock') }}-${{ github.sha }}
    restore-keys: |
      ${{ runner.os }}-cargo-build-${{ hashFiles('**/Cargo.lock') }}-
      ${{ runner.os }}-cargo-build-
```

#### 3.3 Implement Dependency Deduplication

**Priority: MEDIUM**

Analyze `Cargo.lock` for duplicate dependencies:

```bash
# Find duplicate dependencies
cargo tree --duplicates
```

Key findings from analysis:
- Multiple versions of `reqwest` (0.11.27 and 0.12.28)
- Multiple versions of `hyper` (0.14.32 and 1.8.1)
- Multiple versions of `http` (0.2.12 and 1.4.0)

**Recommendation:**
Standardize on latest versions and update `Cargo.toml` files:

```toml
[workspace.dependencies]
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
hyper = "1.8"
http = "1.4"
```

---

## 4. Build Configuration Changes

### Current State Analysis

Current release profile:
```toml
[profile.release]
panic = "unwind"
lto = false
codegen-units = 1
opt-level = 3
```

### Recommendations

#### 4.1 Optimize Release Profile for Disk Usage

**Priority: MEDIUM**

Update `Cargo.toml`:

```toml
[profile.release]
panic = "unwind"
lto = "thin"          # Changed from false - better size/perf balance
codegen-units = 1
opt-level = 3
strip = "debuginfo"   # Strip debug symbols from binaries

[profile.release-lto]
inherits = "release"
lto = true            # Full LTO for maximum size reduction
codegen-units = 1
opt-level = "z"       # Optimize for size
strip = true          # Strip all symbols
```

**Expected Impact:**
- 15-25% reduction in binary sizes
- Faster CI artifact uploads/downloads
- Reduced Docker image sizes

#### 4.2 Add Size-Optimized Profile for CI Artifacts

**Priority: MEDIUM**

```toml
[profile.size-optimized]
inherits = "release"
opt-level = "z"       # Optimize for size
lto = true
codegen-units = 1
strip = true
panic = "abort"       # Smaller than unwind
```

Use for release builds:
```bash
cargo build --profile size-optimized --package terraphim_server
```

#### 4.3 Configure Target-Specific Optimizations

**Priority: LOW**

Add to `.cargo/config.toml`:

```toml
[target.x86_64-unknown-linux-musl]
# Use musl's built-in allocator to reduce binary size
rustflags = ["-C", "target-feature=+crt-static", "-C", "link-arg=-s"]

[target.aarch64-unknown-linux-musl]
rustflags = ["-C", "target-feature=+crt-static", "-C", "link-arg=-s"]
```

---

## 5. Workspace-Specific Optimizations

### Current State Analysis

- 40+ crates in workspace create significant overhead
- Each crate generates its own set of artifacts
- Feature flags create exponential compilation combinations
- No workspace-level artifact sharing

### Recommendations

#### 5.1 Implement Workspace-Level Target Sharing

**Priority: HIGH**

Ensure all crates use shared target directory:

```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = ["crates/*", "terraphim_server", "terraphim_firecracker", "desktop/src-tauri", "terraphim_ai_nodejs"]
exclude = ["crates/terraphim_agent_application", "crates/terraphim_truthforge", "crates/terraphim_automata_py", "crates/terraphim_validation"]

# Set shared target directory
[build]
target-dir = "target"
```

#### 5.2 Consolidate Feature Flags

**Priority: MEDIUM**

Analyze and consolidate feature flags across crates:

**Current Issues:**
- `terraphim_persistence` has 10+ feature flags
- `terraphim_service` has multiple backend features
- Feature combinations create 20+ different build configurations

**Recommendation:**

```toml
# terraphim_persistence/Cargo.toml - Simplified features
[features]
default = ["sqlite", "memory"]
memory = []
sqlite = ["opendal/services-sqlite", "rusqlite"]
dashmap = ["opendal/services-dashmap"]

# Cloud backends - server only
server = ["s3"]
s3 = ["opendal/services-s3"]
redis = ["opendal/services-redis"]
redb = ["opendal/services-redb"]

# Remove unused features:
# - rocksdb (already commented out)
# - atomicserver (removed in opendal 0.54)
# - ipfs (rarely used)
```

#### 5.3 Create Workspace Build Script

**Priority: MEDIUM**

Create `scripts/build-workspace.sh`:

```bash
#!/bin/bash
set -e

# Build script with optimized settings for workspace

# Clean up old artifacts first
./scripts/cleanup-target.sh

# Build with optimized settings
export CARGO_INCREMENTAL=1
export CARGO_PROFILE_DEV_CODEGEN_UNITS=256
export CARGO_PROFILE_DEV_SPLIT_DEBUGINFO=unpacked

# Build only default features for most crates
cargo build --workspace --lib

# Build specific binaries with required features
cargo build --package terraphim_server --features "sqlite,redis"
cargo build --package terraphim-ai-desktop --features "custom-protocol"

# Run tests with minimal features
cargo test --workspace --lib --features "sqlite,memory"
```

---

## 6. CI/CD Pipeline Optimizations

### Current State Analysis

- Multiple redundant builds across workflow files
- Inefficient caching with large target directories
- No artifact sharing between jobs
- Self-hosted runners accumulating artifacts

### Recommendations

#### 6.1 Implement Build Job Deduplication

**Priority: HIGH**

Consolidate build jobs in `.github/workflows/ci-main.yml`:

```yaml
jobs:
  # Single build job with matrix
  build:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            features: "sqlite,redis"
          - target: aarch64-unknown-linux-gnu
            features: "sqlite"
    steps:
      - uses: actions/checkout@v6
      
      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      
      - name: Cache with sccache
        uses: mozilla-actions/sccache-action@v0.0.3
      
      - name: Build
        run: |
          cargo build --release \
            --target ${{ matrix.target }} \
            --features "${{ matrix.features }}" \
            --package terraphim_server \
            --package terraphim_mcp_server
      
      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: binaries-${{ matrix.target }}
          path: |
            target/${{ matrix.target }}/release/terraphim_server
            target/${{ matrix.target }}/release/terraphim_mcp_server
          retention-days: 7  # Reduced from 30/90
```

#### 6.2 Implement Nightly Cleanup

**Priority: HIGH**

Add to `.github/workflows/cleanup.yml`:

```yaml
name: Nightly Cleanup
on:
  schedule:
    - cron: '0 2 * * *'  # 2 AM daily
  workflow_dispatch:

jobs:
  cleanup:
    runs-on: [self-hosted, Linux, X64]
    steps:
      - name: Cleanup target directory
        run: |
          # Remove artifacts older than 3 days
          find /opt/cargo-cache/target -type f -mtime +3 -delete
          find /opt/cargo-cache/target -type d -empty -delete
          
          # Cleanup cargo registry cache
          cargo cache --autoclean
          
      - name: Report disk usage
        run: |
          df -h
          du -sh /opt/cargo-cache/target 2>/dev/null || true
          du -sh ~/.cargo/registry 2>/dev/null || true
```

#### 6.3 Optimize Docker Builds

**Priority: MEDIUM**

Update `docker/Dockerfile.base`:

```dockerfile
# Use multi-stage build with cache mounts
FROM rust:1.92.0-slim as builder

# Install sccache
RUN cargo install sccache --locked
ENV SCCACHE_DIR=/var/cache/sccache
ENV RUSTC_WRAPPER=sccache

# Build with cache mount
RUN --mount=type=cache,target=/var/cache/sccache \
    --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release --package terraphim_server

# Final stage - minimal image
FROM debian:12-slim
COPY --from=builder /app/target/release/terraphim_server /usr/local/bin/
```

---

## 7. Implementation Roadmap

### Phase 1: Immediate Actions (Week 1)

| Task | Owner | Expected Savings |
|------|-------|------------------|
| Implement target directory cleanup script | DevOps | 20-30 GB |
| Configure sccache in CI | DevOps | 30-50 GB |
| Update Cargo profiles | Backend | 15-25 GB |
| Add retention policies to artifacts | DevOps | 10-15 GB |

### Phase 2: Short-term (Weeks 2-3)

| Task | Owner | Expected Savings |
|------|-------|------------------|
| Consolidate feature flags | Backend | 10-20 GB |
| Optimize CI workflow deduplication | DevOps | 15-25 GB |
| Implement nightly cleanup job | DevOps | 20-30 GB |
| Update Docker builds | DevOps | 10-15 GB |

### Phase 3: Long-term (Weeks 4-6)

| Task | Owner | Expected Savings |
|------|-------|------------------|
| Dependency deduplication | Backend | 10-15 GB |
| Workspace restructuring | Architecture | 15-25 GB |
| Advanced caching strategies | DevOps | 10-20 GB |

---

## 8. Risk Assessment

| Optimization | Risk Level | Mitigation |
|--------------|------------|------------|
| Incremental compilation changes | Low | Test builds in CI before deployment |
| sccache integration | Low | Fallback to local compilation if cache fails |
| Feature flag consolidation | Medium | Comprehensive testing of all feature combinations |
| Profile optimizations | Low | Benchmark performance before/after changes |
| Artifact cleanup automation | Medium | Implement dry-run mode first, gradual rollout |
| LTO changes | Low | Test binary sizes and performance |

---

## 9. Monitoring and Metrics

### Key Metrics to Track

1. **Target directory size**: Daily measurement on CI runners
2. **Build cache hit rate**: sccache and Cargo cache metrics
3. **Build duration**: Track improvements from optimizations
4. **Binary sizes**: Monitor release binary sizes
5. **Disk usage trends**: Weekly reports on storage consumption

### Monitoring Implementation

```yaml
# Add to CI workflows
- name: Report metrics
  run: |
    echo "## Build Metrics" >> $GITHUB_STEP_SUMMARY
    echo "Target dir size: $(du -sh target | cut -f1)" >> $GITHUB_STEP_SUMMARY
    echo "Cache hit rate: $(sccache -s | grep 'Cache hits' | head -1)" >> $GITHUB_STEP_SUMMARY
    echo "Binary sizes:" >> $GITHUB_STEP_SUMMARY
    ls -lh target/release/terraphim* | awk '{print $9, $5}' >> $GITHUB_STEP_SUMMARY
```

---

## 10. Conclusion

This optimization strategy addresses the 200+ GB storage consumption issue through a multi-faceted approach:

1. **Immediate relief** through cleanup automation and caching improvements
2. **Structural improvements** through profile optimizations and feature consolidation
3. **Long-term sustainability** through monitoring and CI/CD enhancements

**Total Expected Savings: 115-185 GB** (57-92% reduction from current 200 GB)

The recommended implementation follows a phased approach to minimize risk while delivering incremental benefits. Each phase builds upon the previous, ensuring stable and measurable improvements to the build system's resource utilization.

---

## Appendix A: Quick Reference Commands

```bash
# Manual target cleanup
cargo clean -p <package-name>  # Clean specific package
cargo clean --target-dir target  # Clean entire target

# Cache statistics
cargo cache --info
sccache -s

# Dependency analysis
cargo tree --duplicates
cargo tree -e features

# Build size analysis
cargo bloat --release
cargo size --release

# Profile-guided optimization
cargo build --profile release-lto
```

## Appendix B: Configuration Files

### Updated Cargo.toml (Root)

```toml
[workspace]
resolver = "2"
members = ["crates/*", "terraphim_server", "terraphim_firecracker", "desktop/src-tauri", "terraphim_ai_nodejs"]
exclude = ["crates/terraphim_agent_application", "crates/terraphim_truthforge", "crates/terraphim_automata_py", "crates/terraphim_validation"]
default-members = ["terraphim_server"]

[workspace.package]
version = "1.6.0"
edition = "2024"

[workspace.dependencies]
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.19", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
async-trait = "0.1"
thiserror = "1.0"
anyhow = "1.0"
log = "0.4"

[patch.crates-io]
genai = { git = "https://github.com/terraphim/rust-genai.git", branch = "merge-upstream-20251103" }
self_update = { git = "https://github.com/AlexMikhalev/self_update.git", branch = "update-zipsign-api-v0.2" }

# Optimized profiles for disk usage
[profile.dev]
incremental = true
codegen-units = 256
split-debuginfo = "unpacked"

[profile.test]
incremental = true
codegen-units = 256
split-debuginfo = "unpacked"

[profile.release]
panic = "unwind"
lto = "thin"
codegen-units = 1
opt-level = 3
strip = "debuginfo"

[profile.release-lto]
inherits = "release"
lto = true
codegen-units = 1
opt-level = 3
strip = true

[profile.size-optimized]
inherits = "release"
opt-level = "z"
lto = true
codegen-units = 1
strip = true
panic = "abort"

[profile.ci]
inherits = "dev"
incremental = false
codegen-units = 16
split-debuginfo = "off"
debug = false

[profile.ci-release]
inherits = "release"
lto = "thin"
codegen-units = 8
strip = "debuginfo"
```

### Updated .cargo/config.toml

```toml
# Cargo configuration for Terraphim AI project

# Target-specific configurations for cross-compilation
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"

[target.aarch64-unknown-linux-musl]
linker = "aarch64-linux-musl-gcc"
rustflags = ["-C", "target-feature=+crt-static", "-C", "link-arg=-s"]

[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"

[target.armv7-unknown-linux-musleabihf]
linker = "arm-linux-musleabihf-gcc"

[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"
rustflags = ["-C", "target-feature=+crt-static", "-C", "link-arg=-s"]

# Build configuration
[build]
target-dir = "target"

# Registry configuration
[registries.crates-io]
protocol = "sparse"

# Network configuration
[http]
check-revoke = false

[net]
git-fetch-with-cli = true

# Doc configuration
[doc]
browser = ["firefox", "google-chrome", "safari"]

# Testing configuration
[term]
color = "auto"
quiet = false
verbose = false
```
