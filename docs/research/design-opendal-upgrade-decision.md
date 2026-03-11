# Implementation Plan: OpenDAL Upgrade Decision

**Status**: Review
**Research Doc**: [.docs/research-opendal-upgrade.md](research-opendal-upgrade.md)
**Author**: Terraphim AI
**Date**: 2026-03-11
**Estimated Effort**: N/A (documentation only)

---

## Overview

### Summary
Document the decision to accept transitive security advisory risk from OpenDAL dependencies (instant, fxhash) rather than upgrade to 0.55.

### Approach
1. Document risk acceptance with rationale
2. Create monitoring plan for upstream fixes
3. Prepare upgrade path documentation for future use

### Scope

**In Scope:**
- Document decision rationale
- Create monitoring checklist
- Document future upgrade path
- Update security policy

**Out of Scope:**
- Upgrading OpenDAL (not beneficial for security)
- Replacing backends (unnecessary)
- Forking/patching dependencies

**Avoid At All Cost:**
- Upgrading for security theater
- Breaking existing functionality
- Adding complexity without benefit

---

## Architecture

### Simplicity Check

**What if this could be easy?**
The simplest approach is to accept that:
1. The advisories are in transitive dependencies we don't use directly
2. OpenDAL 0.55 still has the same issue
3. Our code uses std::time::Instant, not instant::Instant
4. No action needed beyond documentation

---

## Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Accept transitive risk | Advisories don't affect our code path | Upgrading (same issue remains) |
| Document, don't upgrade | Upgrade has breaking changes, no security benefit | Forced upgrade for appearances |
| Monitor upstream | Sled/parking_lot may update eventually | Patching dependencies ourselves |

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Upgrade to OpenDAL 0.55 | Still has sled → fxhash/instant | Breaking changes for no benefit |
| Use [patch] to override | Complex, may break other things | Dependency hell |
| Disable sled feature | Not possible (core dependency) | Wasted effort |
| Switch to redb backend | Unnecessary - current backends work fine | Scope creep |

---

## Decision Rationale

### Why Not Upgrade?

**OpenDAL 0.54.1 → 0.55.0 Analysis:**
```
# Current chain
opendal 0.54.1 → sled 0.34.7 → fxhash/instant

# After upgrade
opendal 0.55.0 → sled 0.34.7 → fxhash/instant (unchanged!)
```

**Breaking Changes in 0.55:**
- Scheme enum → string-based (API change)
- Removed deprecated `from_map` / `via_map`
- chrono → jiff migration
- MSRV 1.85, edition 2024

**Effort**: 2-4 hours
**Benefit**: Zero (same security posture)

### Current Security Posture

| Advisory | Crate | How We Use It | Risk Level |
|----------|-------|---------------|------------|
| RUSTSEC-2024-0384 | instant | Not used directly | None |
| RUSTSEC-2025-0057 | fxhash | Not used directly | None |

Our code uses `std::time::Instant`, not `instant::Instant`.
Our code doesn't use fxhash at all.

---

## Monitoring Plan

### Monthly Checks
```bash
# Check for sled updates
cargo search sled

# Check for parking_lot updates
cargo search parking_lot

# Check cargo audit
cargo audit
```

### Triggers for Re-evaluation
1. **Sled releases new version** that updates dependencies
2. **OpenDAL removes sled** from core dependencies
3. **New security advisory** with actual impact on our code
4. **We need OpenDAL 0.55+ features**

### Upstream Issues to Watch
- https://github.com/spacejam/sled/issues (for dependency updates)
- https://github.com/apache/opendal/releases (for sled removal)

---

## Future Upgrade Path

### If OpenDAL Upgrade Becomes Necessary

**Step 1: Update Cargo.toml files**
```toml
# In 4 crates: terraphim_persistence, terraphim_service, terraphim_config, terraphim_middleware
opendal = { version = "0.55" }
```

**Step 2: Fix API changes**
```rust
// Before
use opendal::Scheme;
let op = Operator::via_map(Scheme::Memory, map)?;

// After
let op = Operator::via_map("memory", map)?;
```

**Step 3: Test all backends**
- memory
- sqlite
- dashmap
- s3
- redis
- redb
- ipfs

**Estimated Effort**: 2-4 hours

---

## Alternative Backends

### If Sled Becomes Problematic

| Backend | Pros | Cons | Use Case |
|---------|------|------|----------|
| **sqlite** | Reliable, fast | SQL overhead | Default for persistence |
| **redb** | Pure Rust, fast | Newer crate | Modern alternative |
| **dashmap** | In-memory, fast | Not persistent | Cache layer |
| **memory** | Fastest | Not persistent | Testing |

**Current default**: sqlite + dashmap + memory

---

## Documentation Updates

### Files to Update

| File | Change |
|------|--------|
| `SECURITY.md` | Document accepted transitive risks |
| `CLAUDE.md` | Add note about OpenDAL/sled advisories |
| `.docs/research-opendal-upgrade.md` | Link to this decision |

### Security Policy Addition
```markdown
## Accepted Transitive Risks

The following security advisories are present in transitive dependencies
but do not affect our code:

- RUSTSEC-2024-0384 (instant) - via parking_lot, not used directly
- RUSTSEC-2025-0057 (fxhash) - via sled, not used directly

Rationale: OpenDAL 0.55 still includes sled. We use std::time::Instant,
not instant::Instant. No upgrade path available that eliminates these.
```

---

## Rollback Plan

If we need to reverse this decision:
1. Remove documentation about accepting risk
2. Proceed with OpenDAL 0.55 upgrade
3. Follow "Future Upgrade Path" steps above

---

## Approval

- [ ] Technical review complete
- [ ] Security implications understood
- [ ] Decision documented
- [ ] Monitoring plan in place

---

## Quick Reference

### Commands for Monitoring
```bash
# Check for sled updates
cargo search sled

# Check for parking_lot updates
cargo search parking_lot

# Run security audit
cargo audit

# Check opendal dependencies
cargo tree -i sled
cargo tree -i fxhash
cargo tree -i instant
```

### Related Issues
- #659 - fxhash (closed, transitive)
- #660 - instant (closed, transitive)
- #662 - This research
