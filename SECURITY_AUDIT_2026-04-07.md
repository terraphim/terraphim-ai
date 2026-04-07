# Security Audit Report
**Date**: 2026-04-07 03:02 CEST
**Auditor**: Vigil (Security Engineer)
**Project**: terraphim-ai
**Status**: **FAIL** ⚠️ CRITICAL CVE BLOCKS MERGE

---

## Executive Summary

**VERDICT: FAIL** - One CRITICAL security vulnerability detected that blocks merge/release.

**Critical Issue**: RUSTSEC-2026-0049 in rustls-webpki 0.102.8
- **Severity**: High (Privilege Escalation via X.509 Verification)
- **Category**: TLS certificate revocation checking bypass
- **Impact**: Revoked certificates may not be detected
- **Status**: Merge blocked until patched

**Secondary Issue**: Port 3456 exposed to network (terraphim-llm-p)
- **Severity**: High (Unexpected network exposure)
- **Impact**: LLM service accessible from outside localhost
- **Status**: Requires immediate review

---

## Finding 1: CRITICAL CVE - RUSTSEC-2026-0049

**Crate**: `rustls-webpki`
**Vulnerable Version**: 0.102.8
**Required Version**: >= 0.103.10
**Advisory**: GHSA-pwjx-qhcg-rvj4
**Date Discovered**: 2026-03-20

### Description

Certificate Revocation List (CRL) checking has faulty matching logic. When a certificate contains multiple distribution points, only the first one is validated against each CRL's issuingDistributionPoint. Subsequent distribution points are silently ignored.

### Impact Chain

1. **With UnknownStatusPolicy::Deny (default)**: Results in safe rejection with `UnknownRevocationStatus`
2. **With UnknownStatusPolicy::Allow**: Revoked certificates are incorrectly accepted

### Attack Vector

- Requires compromising a trusted certificate authority to exploit effectively
- In normal use, this bug is latent but could allow attackers to continue using revoked credentials
- Affects any system using rustls for TLS that depends on CRL revocation checking

### Current Status

- **Locked Version**: 0.102.8 in some dependency chains
- **Cargo Tree Shows**: rustls-webpki 0.103.10 from git repository (partially patched)
- **Cargo Audit Reports**: Still detects 0.102.8 as vulnerable
- **Issue**: Inconsistent versions detected - high risk

### Remediation

**Required Action**: Upgrade rustls-webpki to >= 0.103.10 and ensure no other crates pull in the vulnerable version.

```bash
# 1. Update Cargo.lock
cargo update rustls-webpki

# 2. Verify patch
cargo audit

# 3. Re-run verification
cargo test --all-features
```

### Evidence

```
Crate:     rustls-webpki
Version:   0.102.8
Title:     CRLs not considered authoritative by Distribution Point due to faulty matching logic
Date:      2026-03-20
ID:        RUSTSEC-2026-0049
Solution:  Upgrade to >=0.103.10
Categories: privilege-escalation
Keywords: crl, x509
```

---

## Finding 2: HIGH - Unexpected Network Exposure

**Service**: terraphim-llm-p
**Listening Port**: 3456
**Bind Address**: 0.0.0.0
**Status**: Exposed to network

### Evidence

```
LISTEN 0   1024   0.0.0.0:3456   0.0.0.0:*   users:(("terraphim-llm-p",pid=947,fd=9))
```

### Risk Assessment

- **Current**: Listening on all interfaces (0.0.0.0)
- **Exposure**: Accessible from any network reachable to this host
- **Expected**: Should bind to 127.0.0.1 unless intentionally exposed
- **Severity**: High - Unexpected external access to LLM service

### Remediation Options

1. **Restrict to localhost**: Bind to `127.0.0.1:3456` if only local access needed
2. **Document exposure**: If intentional, document security requirements and authentication
3. **Add firewall rules**: If network access is needed, restrict via iptables/firewall
4. **Add authentication**: If exposed, implement API authentication/authorization

### Investigation Required

- Is this intentional exposure for distributed architecture?
- Are there authentication/authorization controls on port 3456?
- Should this be documented in architecture as an exposed service?

---

## Finding 3: INFO - Unmaintained Dependencies

Several transitive dependencies are marked unmaintained but pose no immediate security risk:

| Crate | Version | Status | Alternatives |
|-------|---------|--------|--------------|
| bincode | 1.3.3 | Unmaintained | postcard, rkyv, bitcode |
| instant | 0.1.13 | Unmaintained | web-time |
| number_prefix | 0.4.0 | Unmaintained | unit-prefix |
| paste | 1.0.15 | Unmaintained | pastey |
| rustls-pemfile | 1.0.4 | Unmaintained | rustls-pki-types 1.9.0+ |
| term_size | 0.3.2 | Unmaintained | terminal_size |
| fastrand | 2.4.0 | Yanked | (use next release) |

**Action**: Monitor for updates; no immediate security threat.

---

## Finding 4: GOOD - Secure Coding Practices

✅ **No hardcoded secrets detected**
- No `sk-` patterns (OpenAI API keys)
- No embedded passwords or credentials
- No environment variable injection patterns

✅ **No unsafe blocks detected**
- Zero unsafe code in crate sources
- Safe Rust throughout implementation

✅ **No suspicious recent commits**
- Last 24 hours: Feature work and agent coordination
- No security-related hotfixes suggesting active incidents
- No emergency patches detected

---

## Detailed Audit Checklist

| Check | Status | Details |
|-------|--------|---------|
| **Known CVEs** | ❌ FAIL | RUSTSEC-2026-0049 in rustls-webpki |
| **Dependency Versions** | ⚠️ WARNING | Inconsistent rustls-webpki versions (0.102.8 + 0.103.10) |
| **Hardcoded Secrets** | ✅ PASS | No API keys, passwords, or credentials found |
| **Unsafe Code** | ✅ PASS | Zero unsafe blocks in crates/ |
| **Network Exposure** | ❌ FAIL | Port 3456 exposed to 0.0.0.0 |
| **Recent Security Fixes** | ✅ PASS | No emergency patches in last 24h |
| **Outdated Dependencies** | ⚠️ WARNING | 6 unmaintained crates (transitive) |
| **Yanked Dependencies** | ⚠️ WARNING | fastrand 2.4.0 yanked (consider update) |

---

## Merge Gate Status

### BLOCKED ❌

**Blocking Issues**:
1. **CRITICAL**: RUSTSEC-2026-0049 must be resolved before merge
2. **HIGH**: Port 3456 exposure must be reviewed and documented

**Before Merge**:
- [ ] Upgrade rustls-webpki to >= 0.103.10
- [ ] Verify `cargo audit` returns 0 vulnerabilities
- [ ] Review and document port 3456 exposure (restrict or justify)
- [ ] Re-run this security audit
- [ ] Obtain security sign-off

---

## Recommendations

### Priority 1 (Must Fix Before Merge)
1. **Update rustls-webpki dependency**
   ```bash
   cargo update rustls-webpki --aggressive
   cargo audit  # Should show 0 vulnerabilities
   ```

2. **Address port 3456 exposure**
   - Determine: Is this intentional?
   - If intentional: Add authentication and document
   - If unintentional: Bind to 127.0.0.1

### Priority 2 (Should Address Before Release)
1. Replace `bincode` with `postcard` or `rkyv`
2. Replace `instant` with `web-time`
3. Replace `paste` with `pastey`
4. Update `fastrand` to next release (not yanked)

### Priority 3 (Medium Term)
1. Replace `rustls-pemfile` with `rustls-pki-types` 1.9.0+
2. Replace `number_prefix` with `unit-prefix`
3. Replace `term_size` with `terminal_size`

---

## Audit Trail

**Audit Method**:
- `cargo audit` - Known CVE database from RustSec
- `Cargo.lock` inspection - Dependency version tracking
- Source code grep - Hardcoded secrets scan
- AST search - Unsafe code detection
- `ss -tlnp` - Network exposure check
- `git log` - Recent security-relevant commits

**Tools Used**:
- RustSec Advisory Database (1027 advisories, last updated 2026-04-05)
- Cargo workspace inspection (1034 dependencies)
- System network diagnostics

**Findings Date**: 2026-04-07
**Audit Duration**: ~5 minutes
**Confidence Level**: High (automated detection + manual verification)

---

## Next Steps

1. **Immediate**: Fix RUSTSEC-2026-0049 by updating rustls-webpki
2. **Immediate**: Review and document port 3456 exposure
3. **Re-run**: Execute security audit after fixes
4. **Approval**: Obtain sign-off from security team before merge
5. **Document**: Add security requirements to CLAUDE.md if needed

---

**Report Generated By**: Vigil, Security Engineer
**Classification**: INTERNAL - SECURITY FINDING
**Distribution**: Project team, merge gate, security review
