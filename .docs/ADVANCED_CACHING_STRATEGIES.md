# Advanced Caching Strategies for Terraphim AI

## Overview

This document outlines advanced caching strategies to further reduce build times and disk usage beyond the Phase 1 and Phase 2 optimizations.

## Current State

### Existing Caching (Phase 1)
- ✅ sccache integration in CI
- ✅ Cargo registry caching
- ✅ Target directory caching
- ✅ Docker layer caching

### Gaps Identified
1. No distributed caching across self-hosted runners
2. No build artifact sharing between jobs
3. Limited incremental compilation in CI
4. No specialized caching for WASM builds

## Advanced Caching Strategies

### 1. Distributed sccache with S3 Backend

**Purpose:** Share compilation cache across all CI runners

**Implementation:**

```yaml
# .github/workflows/ci-main.yml
env:
  SCCACHE_BUCKET: "terraphim-sccache"
  SCCACHE_REGION: "us-east-1"
  SCCACHE_S3_KEY_PREFIX: "ci-cache"
  AWS_ACCESS_KEY_ID: ${{ secrets.SCCACHE_AWS_ACCESS_KEY_ID }}
  AWS_SECRET_ACCESS_KEY: ${{ secrets.SCCACHE_AWS_SECRET_ACCESS_KEY }}
  RUSTC_WRAPPER: "sccache"
```

**Benefits:**
- Cache hits across all runners (not just single runner)
- Survives runner restarts
- Shared cache between PR builds

**Expected Savings:**
- 40-60% faster builds
- 20-30 GB less per-runner storage

### 2. Build Artifacts Sharing

**Purpose:** Avoid rebuilding same crates in different jobs

**Implementation:**

```yaml
# .github/workflows/ci-main.yml
jobs:
  build-deps:
    name: Build Dependencies
    steps:
      - name: Build workspace dependencies
        run: cargo build --profile ci --workspace --lib
      
      - name: Upload dependency artifacts
        uses: actions/upload-artifact@v4
        with:
          name: deps-cache-${{ hashFiles('**/Cargo.lock') }}
          path: |
            target/*/ci/deps/*.rlib
            target/*/ci/.fingerprint
          retention-days: 1

  build-binaries:
    name: Build Binaries
    needs: build-deps
    steps:
      - name: Download dependency artifacts
        uses: actions/download-artifact@v4
        with:
          name: deps-cache-${{ hashFiles('**/Cargo.lock') }}
          path: target/
      
      - name: Build binaries
        run: cargo build --profile ci-release --bins
```

**Benefits:**
- Parallel job optimization
- Reduced redundant compilation

**Expected Savings:**
- 15-25% faster CI pipelines
- 10-15 GB per workflow run

### 3. Feature-Gated Caching

**Purpose:** Cache feature-specific builds separately

**Implementation:**

```yaml
# .github/workflows/ci-main.yml
strategy:
  matrix:
    include:
      - features: "sqlite,memory"
        cache-key: "core"
      - features: "sqlite,redis,s3"
        cache-key: "server"
      - features: "all-backends"
        cache-key: "full"

cache:
  key: ${{ matrix.cache-key }}-${{ hashFiles('**/Cargo.lock') }}
```

**Benefits:**
- More targeted cache hits
- Reduced cache thrashing

### 4. WASM-Specific Caching

**Purpose:** Optimize WASM build caching

**Implementation:**

```yaml
# .github/workflows/ci-main.yml
- name: Cache WASM build
  uses: actions/cache@v4
  with:
    path: |
      crates/terraphim_automata/wasm-test/pkg
      crates/terraphim_automata/wasm-test/target
    key: wasm-${{ hashFiles('crates/terraphim_automata/**') }}

- name: Cache wasm-pack
  uses: actions/cache@v4
  with:
    path: ~/.cargo/bin/wasm-pack
    key: wasm-pack-0.12.1
```

**Expected Savings:**
- 50% faster WASM builds
- 2-3 GB per WASM build

### 5. Incremental Compilation Cache

**Purpose:** Enable incremental compilation in CI with proper caching

**Implementation:**

```yaml
# .github/workflows/ci-main.yml
env:
  CARGO_INCREMENTAL: 1

- name: Cache incremental compilation
  uses: actions/cache@v4
  with:
    path: target/**/incremental
    key: incremental-${{ github.ref }}-${{ github.sha }}
    restore-keys: |
      incremental-${{ github.ref }}-
      incremental-main-
```

**Note:** Only enable for development branches, not release builds.

**Expected Savings:**
- 30-50% faster incremental builds
- Best for PR builds

### 6. Docker BuildKit Cache Mounts

**Purpose:** Optimize Docker builds with persistent caching

**Implementation:**

```dockerfile
# docker/Dockerfile.base
RUN --mount=type=cache,target=/var/cache/sccache \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --profile ci-release --workspace
```

**Benefits:**
- Cache persists between builds
- No layer bloat from build artifacts

**Expected Savings:**
- 60-80% faster Docker builds
- 5-10 GB smaller images

### 7. GitHub Actions Cache Optimization

**Purpose:** Optimize cache key strategy

**Implementation:**

```yaml
# .github/workflows/ci-main.yml
- name: Cache Cargo
  uses: actions/cache@v4
  with:
    path: |
      ~/.cargo/registry/index
      ~/.cargo/registry/cache
      ~/.cargo/git/db
    key: cargo-registry-${{ hashFiles('**/Cargo.lock') }}-v1
    restore-keys: |
      cargo-registry-${{ hashFiles('**/Cargo.lock') }}-
      cargo-registry-

- name: Cache Build
  uses: actions/cache@v4
  with:
    path: target
    key: build-${{ runner.os }}-${{ matrix.target }}-${{ hashFiles('**/Cargo.lock') }}-${{ github.sha }}
    restore-keys: |
      build-${{ runner.os }}-${{ matrix.target }}-${{ hashFiles('**/Cargo.lock') }}-
      build-${{ runner.os }}-${{ matrix.target }}-
```

**Key Improvements:**
- Separate registry and build caches
- Versioned cache keys for invalidation
- Target-specific caches

### 8. Pre-built Docker Images for CI

**Purpose:** Avoid rebuilding base image dependencies

**Implementation:**

```yaml
# .github/workflows/docker-base.yml
name: Build Base Image
on:
  push:
    paths:
      - 'docker/Dockerfile.base'
      - '.github/rust-toolchain.toml'

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - name: Build and push base image
        uses: docker/build-push-action@v5
        with:
          context: .
          file: ./docker/Dockerfile.base
          target: base-builder
          push: true
          tags: ghcr.io/terraphim/terraphim-ai:base-builder
          cache-from: type=gha
          cache-to: type=gha,mode=max
```

```dockerfile
# docker/Dockerfile
FROM ghcr.io/terraphim/terraphim-ai:base-builder as builder
# ... rest of build
```

**Expected Savings:**
- 5-10 minutes per build
- Consistent build environment

## Implementation Priority

### High Priority (Immediate)

1. **S3-backed sccache** - Highest impact for distributed builds
2. **Docker BuildKit cache mounts** - Immediate Docker improvements
3. **Cache key optimization** - Better cache hit rates

### Medium Priority (Week 2)

4. **WASM-specific caching** - Faster frontend builds
5. **Build artifact sharing** - Parallel job optimization
6. **Pre-built base images** - Consistent CI environment

### Low Priority (Week 3)

7. **Feature-gated caching** - Fine-tuned optimization
8. **Incremental compilation** - Development branch optimization

## Monitoring and Metrics

### Cache Hit Rate Tracking

```yaml
- name: Report cache metrics
  run: |
    echo "## Cache Metrics" >> $GITHUB_STEP_SUMMARY
    echo "sccache hits: $(sccache -s | grep 'Cache hits' | head -1)" >> $GITHUB_STEP_SUMMARY
    echo "sccache size: $(sccache -s | grep 'Cache size' | head -1)" >> $GITHUB_STEP_SUMMARY
```

### Build Time Tracking

```yaml
- name: Track build time
  uses: benchmark-action/github-action-benchmark@v1
  with:
    tool: 'cargo'
    output-file-path: target/criterion/report/index.html
    github-token: ${{ secrets.GITHUB_TOKEN }}
    auto-push: true
```

## Cost Analysis

### S3 Storage Costs (Estimated)

| Item | Size | Cost/Month |
|------|------|------------|
| sccache storage | 50 GB | ~$1.15 |
| Data transfer | 100 GB | ~$9.00 |
| **Total** | | **~$10/month** |

### Time Savings Value

| Metric | Before | After | Savings |
|--------|--------|-------|---------|
| Average build time | 25 min | 12 min | 13 min |
| Daily builds | 20 | 20 | - |
| **Daily time saved** | | | **260 min** |
| **Monthly time saved** | | | **~87 hours** |

## Rollback Plan

1. **S3 Cache Issues:** Fall back to local sccache or disable
2. **Cache Poisoning:** Version cache keys to invalidate
3. **Performance Regression:** Revert to previous configuration

## Conclusion

These advanced caching strategies can provide:
- **50-70% faster builds**
- **30-50 GB storage savings**
- **Improved developer productivity**
- **Reduced CI costs**

Start with S3-backed sccache and Docker BuildKit mounts for immediate impact.
