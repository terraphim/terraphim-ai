# CI/CD Migration Plan: Earthly to Dagger

## Executive Summary

This document outlines the migration strategy for Terraphim AI's CI/CD pipeline from Earthly to Dagger, necessitated by Earthly's announcement to shut down their cloud services and end active maintenance by July 16, 2025.

## Current State Assessment

### Earthly Infrastructure
- **4 Active Earthfiles:**
  - Root: Main orchestrator with cross-compilation support
  - Desktop: Frontend build with Node.js/Yarn
  - Terraphim Server: Rust backend builds
  - Atomic Server Infrastructure: Firecracker deployment

- **Key Features in Use:**
  - Multi-platform builds (linux/amd64, linux/arm/v7, linux/arm64)
  - Earthly Cloud satellites for remote builds
  - Cross-compilation for multiple targets
  - Integrated testing, linting, and formatting
  - Docker image generation with musl for minimal containers
  - GitHub Actions integration via earthly/actions-setup

### Build Complexity Analysis

#### Root Earthfile
- **Targets:** 15+ build targets
- **Cross-compilation:** 4 platforms (x86_64, armv7, aarch64)
- **Dependencies:** Rust 1.82.0, Node.js, Yarn, ripgrep, cross, orogene
- **Caching:** Vendor directory persistence, cargo caching
- **Artifacts:** Binary outputs for multiple architectures

#### Desktop Earthfile
- **Framework:** Svelte with TypeScript
- **Testing:** Vitest unit tests, Playwright E2E tests
- **Build Process:** Node 20, Yarn frozen lockfile
- **Output:** Static dist folder for embedding in server

#### Server Earthfile
- **Language:** Rust with async/concurrent systems
- **Features:** Embedded frontend dist, cross-platform support
- **Testing:** Cargo test suite
- **Linting:** rustfmt, clippy

## Migration Rationale

### Why Migration is Mandatory
1. **Earthly Shutdown:** Services ending July 16, 2025
2. **No Commercial Support:** Active maintenance ending
3. **Remote Runners Disappearing:** Loss of performance benefits
4. **Community Fork Uncertainty:** No guaranteed long-term viability

### Why Dagger is Recommended
1. **Migration Support:** Free 1-year Team plan for Earthly users
2. **Technical Alignment:** Both use Buildkit underneath
3. **Language Support:** TypeScript SDK matches frontend stack
4. **Professional Support:** Hands-on migration workshop available
5. **Modern Architecture:** Programmable CI/CD as code

## Migration Strategy

### Phase 1: Foundation (Weeks 1-2)

#### Week 1: Setup and Learning
- [ ] Accept Dagger's free migration offer
- [ ] Install Dagger CLI and SDKs
- [ ] Set up Dagger Cloud account
- [ ] Attend Dagger migration workshop
- [ ] Review Dagger documentation for Rust and TypeScript

#### Week 2: Proof of Concept
- [ ] Create `dagger/` directory structure
- [ ] Implement simple "Hello World" Dagger function
- [ ] Convert Desktop build to Dagger TypeScript
- [ ] Validate local execution
- [ ] Test Dagger Cloud caching

### Phase 2: Core Migration (Weeks 3-6)

#### Week 3-4: Frontend Migration
```typescript
// dagger/src/desktop.ts
import { dag, Container, Directory } from "@dagger.io/dagger"

export class Desktop {
  async build(source: Directory): Promise<Directory> {
    return dag
      .container()
      .from("node:20")
      .withDirectory("/app", source)
      .withWorkdir("/app/desktop")
      .withExec(["yarn", "install", "--frozen-lockfile"])
      .withExec(["yarn", "build"])
      .directory("/app/desktop/dist")
  }

  async test(source: Directory): Promise<string> {
    // Implement test pipeline
  }
}
```

#### Week 5-6: Backend Migration
```typescript
// dagger/src/terraphim-server.ts
export class TerraphimServer {
  async buildRelease(source: Directory, platform: string): Promise<File> {
    return dag
      .container()
      .from("rust:1.82")
      .withDirectory("/code", source)
      .withExec(["cargo", "build", "--release", "--target", platform])
      .file(`/code/target/${platform}/release/terraphim_server`)
  }

  async crossCompile(source: Directory): Promise<Container> {
    // Implement cross-compilation logic
  }
}
```

### Phase 3: Advanced Features (Weeks 7-9)

#### Week 7: Cross-Platform Builds
- [ ] Implement multi-platform build matrix
- [ ] Set up QEMU for ARM builds
- [ ] Configure platform-specific optimizations
- [ ] Test binary outputs on target platforms

#### Week 8: Testing and Quality
- [ ] Migrate test suites to Dagger
- [ ] Implement parallel test execution
- [ ] Set up coverage reporting
- [ ] Configure linting pipelines

#### Week 9: Docker and Deployment
- [ ] Convert Docker image generation
- [ ] Implement multi-stage builds
- [ ] Set up registry pushes
- [ ] Configure deployment pipelines

### Phase 4: Transition (Weeks 10-12)

#### Week 10: GitHub Actions Integration
```yaml
# .github/workflows/ci.yml
name: CI with Dagger
on:
  push:
    branches: [main]
  pull_request:

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dagger/dagger-for-github@v5
        with:
          version: "0.9.5"
          cmds: |
            call build --source .
            call test --source .
```

#### Week 11: Parallel Running
- [ ] Run Earthly and Dagger in parallel
- [ ] Compare build times and outputs
- [ ] Validate artifact consistency
- [ ] Monitor for issues

#### Week 12: Cutover
- [ ] Disable Earthly pipelines
- [ ] Update documentation
- [ ] Train team on Dagger
- [ ] Remove Earthly configuration

## Technical Migration Details

### Earthly to Dagger Concept Mapping

| Earthly Concept | Dagger Equivalent | Migration Notes |
|-----------------|-------------------|-----------------|
| Earthfile | Dagger Module (TypeScript/Go/Python) | Complete rewrite required |
| Targets | Functions | More flexible, programmable |
| FROM | Container.from() | Direct mapping |
| COPY | Container.withDirectory() | Similar semantics |
| RUN | Container.withExec() | Array format for commands |
| SAVE ARTIFACT | Directory/File return | Explicit return types |
| BUILD | Function calls | Composable functions |
| ARG | Function parameters | Type-safe parameters |
| CACHE | Dagger Cloud caching | Automatic with Cloud |
| WITH DOCKER | Nested containers | Service binding pattern |

### Key Migration Patterns

#### Pattern 1: Multi-Stage Builds
```typescript
// Earthly: Multi-stage with dependencies
// FROM +install
// COPY +build/artifact /app

// Dagger: Composable functions
async buildWithDeps() {
  const deps = await this.install()
  const artifact = await this.build(deps)
  return artifact
}
```

#### Pattern 2: Cross-Compilation
```typescript
// Earthly: Platform-specific targets
// BUILD +cross-build --TARGET=aarch64-unknown-linux-musl

// Dagger: Parameterized functions
async crossBuild(target: string): Promise<File> {
  return this.buildWithTarget(target)
}
```

#### Pattern 3: Caching
```typescript
// Earthly: Explicit cache mounts
// CACHE --persist /root/.cargo

// Dagger: Automatic with Cloud + explicit mounts
.withMountedCache("/root/.cargo", dag.cacheVolume("cargo"))
```

## Risk Management

### Identified Risks

1. **Learning Curve**
   - **Mitigation:** Workshop attendance, gradual migration
   - **Contingency:** Extend timeline, get Dagger support

2. **Build Compatibility**
   - **Mitigation:** Parallel running, extensive testing
   - **Contingency:** Maintain Earthly backup until July

3. **Performance Regression**
   - **Mitigation:** Benchmark comparisons, optimization
   - **Contingency:** Invest in Dagger Cloud resources

4. **Team Resistance**
   - **Mitigation:** Training, documentation, gradual transition
   - **Contingency:** Assign champions, provide incentives

### Success Criteria

- [ ] All builds complete successfully in Dagger
- [ ] Build times within 20% of Earthly baseline
- [ ] All tests passing with same coverage
- [ ] Team trained and comfortable with Dagger
- [ ] Documentation updated and complete
- [ ] Zero production incidents during migration

## Resource Requirements

### Personnel
- **Lead Developer:** 50% time for 12 weeks
- **DevOps Engineer:** 75% time for 12 weeks
- **Team Training:** 4 hours per developer

### Infrastructure
- **Dagger Cloud:** Free for first year (Team plan)
- **Development Environment:** Local Dagger installation
- **CI Runners:** GitHub Actions (existing)

### Budget
- **Year 1:** $0 (free migration offer)
- **Year 2:** ~$200/month for Dagger Cloud Team
- **Training:** Workshop included in migration offer

## Alternative Approaches Considered

### Option: Community Earthly Fork
- **Pros:** No code changes, familiar tooling
- **Cons:** Uncertain support, self-hosted infrastructure
- **Decision:** Too risky for production system

### Option: Direct Docker + Make
- **Pros:** Simple, no vendor lock-in
- **Cons:** Loss of features, more maintenance
- **Decision:** Insufficient for complex builds

### Option: Bazel/Nix
- **Pros:** Powerful, reproducible
- **Cons:** Steep learning curve, major rewrite
- **Decision:** Too disruptive for timeline

## Implementation Checklist

### Pre-Migration
- [ ] Backup all Earthfiles
- [ ] Document current build process
- [ ] Inventory all build artifacts
- [ ] Set up Dagger development environment
- [ ] Complete team training

### During Migration
- [ ] Weekly progress reviews
- [ ] Parallel build validation
- [ ] Performance benchmarking
- [ ] Issue tracking and resolution
- [ ] Documentation updates

### Post-Migration
- [ ] Remove Earthly dependencies
- [ ] Archive Earthly configuration
- [ ] Update all documentation
- [ ] Conduct retrospective
- [ ] Plan optimization phase

## Monitoring and Validation

### Metrics to Track
- Build success rate
- Build duration by type
- Cache hit rates
- Artifact sizes
- Test coverage
- Deployment frequency

### Validation Steps
1. Binary compatibility testing
2. Performance benchmarking
3. Security scanning
4. Dependency validation
5. Multi-platform testing

## Rollback Plan

If critical issues arise:
1. **Immediate:** Continue using Earthly (until July 2025)
2. **Week 1-6:** Revert to Earthly, reassess approach
3. **Week 7-12:** Parallel run both systems
4. **Post-July 2025:** Must complete migration or use alternative

## Communication Plan

### Stakeholders
- Development team
- DevOps team
- Management
- External contributors

### Communication Schedule
- **Weekly:** Progress updates to team
- **Bi-weekly:** Status report to management
- **Monthly:** Blog post for community
- **Milestone:** Announcements at phase completion

## Conclusion

The migration from Earthly to Dagger is mandatory due to Earthly's shutdown. This plan provides a structured approach to complete the migration within 12 weeks, leveraging Dagger's free migration offer and support. The phased approach minimizes risk while ensuring all functionality is preserved and potentially enhanced through Dagger's programmable pipeline approach.

## Appendix A: Useful Resources

- [Dagger Documentation](https://docs.dagger.io/)
- [Earthly to Dagger Migration Guide](https://dagger.io/blog/earthly-to-dagger-migration)
- [Dagger TypeScript SDK](https://docs.dagger.io/sdk/typescript)
- [Dagger Cloud Documentation](https://docs.dagger.io/cloud)
- [Buildkit Documentation](https://github.com/moby/buildkit)

## Appendix B: Current Earthfile Analysis

### Complexity Metrics
- **Total Earthfiles:** 4 active
- **Total Targets:** 40+
- **Lines of Code:** ~800
- **Platform Support:** 3 architectures
- **External Dependencies:** 15+

### Migration Effort Estimate
- **Simple Targets:** 15 targets × 2 hours = 30 hours
- **Complex Targets:** 10 targets × 4 hours = 40 hours
- **Cross-compilation:** 4 platforms × 6 hours = 24 hours
- **Testing/Validation:** 40 hours
- **Documentation:** 20 hours
- **Total Estimate:** 154 hours (~4 weeks full-time)

## Appendix C: Sample Dagger Module Structure

```
dagger/
├── dagger.json
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts
│   ├── desktop.ts
│   ├── server.ts
│   ├── cross-compile.ts
│   ├── docker.ts
│   └── tests.ts
└── tests/
    ├── desktop.test.ts
    ├── server.test.ts
    └── integration.test.ts
```

---

*Document Version: 1.0*
*Date: 2025-01-03*
*Author: Terraphim AI Team*
*Status: Draft - Pending Review*