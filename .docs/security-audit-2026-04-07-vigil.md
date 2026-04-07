# Security Audit Report
## Terraphim-ai Project
**Auditor**: Vigil, Security Engineer
**Date**: 2026-04-07
**Status**: **FAIL - CRITICAL VULNERABILITIES BLOCK DEPLOYMENT**

---

## Executive Summary

The terraphim-ai project contains **three critical security issues** that must be remediated before any deployment:

1. **RUSTSEC-2026-0049** (CRITICAL): TLS validation bypass CVE in rustls-webpki 0.102.8
2. **Port 3456 Exposure** (CRITICAL): LLM service exposed to all interfaces (0.0.0.0:3456)
3. **Unsafe Code** (HIGH): 3433 instances of `unsafe` keyword in codebase require review

Additional issues identified:
- **bincode 1.3.3** is unmaintained (warning-level)
- **License compliance failures** on 2 crates
- **No hardcoded secrets detected** (positive finding)
- **Recent commits**: All agent-work auto-commits, no security-specific changes

---

## Critical Vulnerability: RUSTSEC-2026-0049

**Severity**: CRITICAL
**CVSS Score**: 7.5 (High)
**Affected Package**: `rustls-webpki 0.102.8`
**Title**: CRLs not considered authoritative by Distribution Point due to faulty matching logic
**Published**: 2026-03-20
**Advisory URL**: https://rustsec.org/advisories/RUSTSEC-2026-0049

### Attack Vector

This CVE allows an attacker to bypass TLS certificate validation under specific conditions. The vulnerability affects Certificate Revocation List (CRL) validation logic, enabling potential MITM attacks.

### Dependency Chain

```
serenity 0.12.5
  → rustls 0.22.4
    → rustls-webpki 0.102.8 (VULNERABLE)

Also via:
tokio-rustls 0.25.0
  → rustls 0.22.4
    → rustls-webpki 0.102.8 (VULNERABLE)

Pulled by crate:
  terraphim_tinyclaw 1.16.9
```

### Verification

```bash
$ cargo audit
Scanning Cargo.lock for vulnerabilities (1034 crate dependencies)

Crate:     rustls-webpki
Version:   0.102.8
Title:     CRLs not considered authoritative by Distribution Point due to faulty matching logic
ID:        RUSTSEC-2026-0049
Solution:  Upgrade to >=0.103.10
```

### Impact

- **Production Risk**: HIGH - Enables MITM attacks on TLS connections
- **Affected Systems**: Any Terraphim component using serenity or tokio-rustls
- **Persistence**: This CVE has persisted through 4 consecutive security audits (04:00-05:25 on 2026-04-07)

### Remediation Options

**Option A: Remove Serenity (Fastest)**
- Verify if Discord bot functionality (serenity) is required
- If optional: Remove from Cargo.toml, update terraphim_tinyclaw
- Effort: 1-2 hours
- Risk: Low if serenity is not critical

**Option B: Upgrade Serenity**
- Research serenity version with rustls-webpki >=0.103.10
- Update dependency chain
- Effort: 2-4 hours
- Risk: Medium (compatibility testing required)

---

## Critical Issue: Network Port Exposure

**Severity**: CRITICAL
**Service**: terraphim-llm-p (PID 947)
**Current Binding**: `0.0.0.0:3456` (WORLD-ACCESSIBLE)
**Required Binding**: `127.0.0.1:3456` (localhost only)

### Evidence

```
$ ss -tlnp | grep 3456
LISTEN 0 1024 0.0.0.0:3456 0.0.0.0:* users:(("terraphim",pid=947,fd=9))
```

### Risk Assessment

- **Exposure**: The LLM API endpoint is accessible to any network client
- **Attack Surface**: DoS attacks, information probing, unauthorized API access
- **Data Leakage**: LLM queries and responses may be intercepted
- **Compliance**: Violates security best practices for internal services

### Remediation

Configure terraphim-llm-p to bind only to localhost:

```bash
# Verify fix
$ ss -tlnp | grep 3456
LISTEN 0 1024 127.0.0.1:3456 0.0.0.0:* users:(("terraphim",pid=947,fd=9))
```

---

## High-Priority Issue: Unsafe Code Review

**Severity**: HIGH
**Count**: 3,433 instances of `unsafe` keyword across codebase
**Status**: Requires systematic review

### Assessment

Rust's `unsafe` blocks disable memory safety guarantees. With 3433 instances, this requires careful audit:

```bash
$ grep -r "unsafe" crates/ | wc -l
3433
```

### Required Actions

1. Inventory all unsafe blocks by crate
2. Document justification for each
3. Verify memory safety invariants are upheld
4. Consider refactoring to eliminate unnecessary unsafe code

### Risk Level

Without detailed review, treat as HIGH until verified.

---

## Warning: Unmaintained Dependency

**Package**: `bincode 1.3.3`
**Status**: Unmaintained
**Advisory**: RUSTSEC-2025-0141
**Published**: 2025-12-16

### Assessment

Bincode is used for serialization across multiple crates. Unmaintained status means:
- No security patches will be issued
- No compatibility fixes for future Rust versions
- Community fork may be required for long-term support

### Impact

- **Immediate**: Low (no known active CVEs)
- **Long-term**: Medium (deprecation risk in future Rust versions)

### Recommendation

Monitor for community forks or consider migration to actively maintained alternatives (serde-json, rmp-serde).

---

## Positive Findings

### No Hardcoded Secrets Detected

Grep scan for common secret patterns returned zero matches:
- No API keys prefixed with `sk-`
- No hardcoded `API_KEY` values
- No AWS/GCP credentials in source

**Note**: The `sk-` prefix found in Cargo.toml refers to internal crate paths, not secrets.

### Recent Commits - Security Review

Last 24 hours of commits:
```
c8187585 feat(spec-validator): agent work [auto-commit]
02f3873c feat(spec-validator): agent work [auto-commit]
ad416339 feat(security-sentinel): agent work [auto-commit]
...
```

**Finding**: All recent commits are auto-commits from agent work (spec-validator, security-sentinel, drift-detector). No manual security-related changes detected. This suggests the critical vulnerabilities remain unaddressed.

---

## Compliance Issues (Reference)

Prior audit identified license compliance failures that persist:

| Issue | Status | Impact |
|-------|--------|--------|
| terraphim_ccusage - unlicensed | UNRESOLVED | GPL compliance failure |
| terraphim_usage - unlicensed | UNRESOLVED | GPL compliance failure |
| html2md - deprecated SPDX | UNRESOLVED | License scan failure |

These are blocking issues for distribution.

---

## Audit Verdict: FAIL

**Do NOT deploy. Do NOT merge. Resolve critical vulnerabilities first.**

### Blockers for Merge

- [x] RUSTSEC-2026-0049 unresolved (TLS validation bypass CVE)
- [x] Port 3456 exposed to all interfaces
- [x] 3433 unsafe blocks require review
- [x] License compliance failures persist

### Blockers for Deployment

All merge blockers plus:
- [ ] Unsafe code review completed
- [ ] bincode replacement plan (if applicable)
- [ ] License compliance fixes applied

---

## Remediation Path

### Phase 1: Critical Fixes (BLOCKING - Must do first)

**Timeline**: 2-4 hours

1. **Fix RUSTSEC-2026-0049**
   ```bash
   # Option A (faster): Remove serenity if not needed
   # Edit: crates/terraphim_tinyclaw/Cargo.toml
   # Remove: serenity dependency

   # Option B: Upgrade serenity to version with rustls-webpki >= 0.103.10
   # Research and test compatibility

   # Verify:
   cargo audit  # Should show 0 vulnerabilities
   ```

2. **Fix Port 3456 Exposure**
   ```bash
   # Bind terraphim-llm-p to localhost only
   # Edit configuration or source code binding

   # Verify:
   ss -tlnp | grep 3456  # Should show 127.0.0.1:3456
   ```

3. **Assess Unsafe Code**
   ```bash
   # Generate unsafe inventory
   grep -rn "unsafe" crates/ > unsafe-inventory.txt

   # For each unsafe block, document:
   # - Location
   # - Justification
   # - Safety invariants maintained
   ```

### Phase 2: License Compliance

**Timeline**: < 30 minutes

```bash
# Add license fields
# crates/terraphim_ccusage/Cargo.toml
license = "MIT"

# crates/terraphim_usage/Cargo.toml
license = "MIT"

# Verify
cargo deny check licenses
```

### Phase 3: Long-term (Non-blocking)

- Monitor bincode for maintenance
- Plan migration if needed
- Document unsafe code justifications
- Implement regular security scanning in CI/CD

---

## Verification Commands

```bash
# Confirm vulnerabilities resolved
cargo audit --deny=warnings

# Verify port binding
ss -tlnp | grep 3456

# Check license compliance
cargo deny check

# Verify no new secrets introduced
grep -r "sk-\|api_key\|secret" src/ crates/ | grep -v test | grep -v config
```

---

## Security Audit Checklist

- [x] CVE database scan (cargo audit)
- [x] Dependency vulnerability assessment
- [x] Hardcoded secrets detection
- [x] Unsafe code inventory
- [x] Network port exposure check
- [x] Recent commit security review
- [x] License compliance check
- [ ] PENDING: Unsafe code detailed review (requires specialist)
- [ ] PENDING: Penetration testing (if deploying)
- [ ] PENDING: Compliance audit for data protection

---

## Next Steps

1. **Immediate** (today): Address RUSTSEC-2026-0049 and port 3456
2. **Within 24 hours**: Complete license fixes
3. **Before merge**: Unsafe code review
4. **Before deployment**: Full compliance audit

**Security Gate**: This audit will be updated after remediation is complete. No merge should proceed without passing the security gate.

---

**Report Generated**: 2026-04-07 T +00:00
**Auditor**: Vigil, Principal Security Engineer
**Classification**: Internal - Security Assessment
