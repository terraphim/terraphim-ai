# Security Audit Report: Terraphim AI
**Date**: 2026-04-07 04:50 CEST
**Auditor**: Vigil (Security Engineer)
**Status**: FAIL (Critical vulnerability unresolved)

## Executive Summary

**CRITICAL VULNERABILITY DETECTED**: RUSTSEC-2026-0049 (rustls-webpki CRL validation bypass) remains unresolved in the Cargo.lock despite remediation commits. Additionally, port 3456 is exposed to all network interfaces. These issues block production deployment.

---

## Critical Findings

### 1. CRITICAL CVE: RUSTSEC-2026-0049
**Severity**: CRITICAL (Privilege Escalation)
**CVE ID**: GHSA-pwjx-qhcg-rvj4
**Affected Package**: rustls-webpki 0.102.8
**Impact**: CRL validation bypass - revoked certificates may be accepted

**Details**:
- X.509 CRL (Certificate Revocation List) matching logic is faulty
- If a certificate has multiple `distributionPoint`s, only the first is checked
- Subsequent distribution points are ignored, bypassing CRL validation
- With default policy (`UnknownStatusPolicy::Deny`), leads to `UnknownRevocationStatus` error
- With permissive policy, leads to acceptance of revoked certificates

**Evidence**:
```
Crate:     rustls-webpki
Version:   0.102.8
Title:     CRLs not considered authoritative by Distribution Point due to faulty matching logic
Date:      2026-03-20
ID:        RUSTSEC-2026-0049
Solution:  Upgrade to >=0.103.10
```

**Dependency Chain**:
- rustls-webpki 0.102.8 → rustls 0.22.4 → tokio-tungstenite 0.21.0
- Affects: terraphim_tinyclaw (serenity dependency chain)

**Remediation Status**: 
- Some crates upgraded to rustls-webpki 0.103.10 (from git source)
- However, 0.102.8 still present in Cargo.lock
- serenity 0.12.5 dependency chain still pulls vulnerable version
- **ACTION REQUIRED**: Remove serenity 0.12.x entirely or upgrade to 0.13+ (does not exist; serenity team defunct)
- **ALTERNATIVE**: Consider removing serenity dependency if not critical path

**Blocking Status**: YES - Merge/release cannot proceed until resolved

---

### 2. CRITICAL: Port 3456 Network Exposure
**Severity**: CRITICAL (Network Access)
**Finding**: Service listening on 0.0.0.0:3456 (all interfaces)
**Process**: terraphim-llm-p

**Evidence**:
```
LISTEN 0 1024 0.0.0.0:3456 0.0.0.0:* users:(("terraphim-llm-p",pid=947,fd=9))
```

**Risk Assessment**:
- Exposed to all network interfaces (0.0.0.0) instead of localhost-only (127.0.0.1)
- Allows remote network access if in cloud/shared environment
- Potential for unauthorized model prompt injection
- Could be abused to run arbitrary queries against LLM backend

**Remediation**:
1. Change binding from 0.0.0.0 to 127.0.0.1
2. If remote access needed, use VPN/firewall rules
3. Verify configuration in terraphim-llm-p startup

**Blocking Status**: YES - Production security risk

---

## High Severity Findings

### 3. Unmaintained Dependencies (Informational)
**Severity**: HIGH (Maintenance Risk)

| Package | Version | Advisory | Status |
|---------|---------|----------|--------|
| bincode | 1.3.3 | RUSTSEC-2025-0141 | Unmaintained - team discontinued |
| instant | 0.1.13 | RUSTSEC-2024-0384 | No active maintenance |
| rustls-pemfile | 1.0.4 | RUSTSEC-2025-0134 | Archived (Nov 2025) |
| number_prefix | 0.4.0 | RUSTSEC-2025-0119 | No active development |
| paste | 1.0.15 | RUSTSEC-2024-0436 | Archived, unmaintained |
| term_size | 0.3.2 | RUSTSEC-2020-0163 | Defunct since 2020 |

**Impact**: While not immediately exploitable, unmaintained dependencies create technical debt and future vulnerability risk.

**Recommendation**: Plan migration to maintained alternatives (postcard for bincode, web-time for instant, etc.)

---

## Medium Severity Findings

### 4. Unsafe Code Blocks: 86 instances
**Finding**: 86 `unsafe` blocks in codebase
**Requirement**: Audit all unsafe blocks for necessity

**Recommendation**:
- Each unsafe block requires justification
- Verify memory safety invariants
- Consider if safety can be achieved through safe abstractions
- Document SAFETY comments for all unsafe blocks

---

## No Issues Found

### ✓ Hardcoded Secrets
- **Status**: PASS
- **Finding**: No hardcoded API keys, secrets, or credentials detected in grep scan

### ✓ Recent Commits
- Last 24 hours: Auto-commits from agents (spec-validator, drift-detector, security-sentinel)
- No suspicious security-related changes
- Remediation attempts visible but incomplete

---

## Dependency Analysis Summary

**Total Dependencies**: 1,034 (from Cargo.lock)
**Vulnerabilities Found**: 1 (CRITICAL)
**Warnings**: 7 (unmaintained packages)
**Yanked Crates**: 1 (fastrand 2.4.0 - unused)

---

## Remediation Plan (BLOCKING)

### Phase 1: Immediate (MUST FIX)
1. **Remove serenity dependency** 
   - Commit: Remove terraphim_tinyclaw serenity 0.12.x dependency
   - Reason: serenity team defunct, dependency chain pulls rustls-webpki 0.102.8
   - Cargo.lock will auto-update after removal
   - Verify no other crates depend on serenity

2. **Fix port 3456 exposure**
   - Change terraphim-llm-p binding from 0.0.0.0 to 127.0.0.1
   - Test: `ss -tlnp | grep 3456` should show 127.0.0.1
   - Update any documentation/config that assumes port accessibility

### Phase 2: Follow-up (SHOULD FIX)
3. Replace unmaintained dependencies with maintained alternatives
4. Complete unsafe code audit with safety justifications

---

## Verification Commands

Run these commands to verify remediations:
```bash
# Verify RUSTSEC-2026-0049 resolved
cargo audit | grep RUSTSEC-2026-0049
# Should return: "no vulnerabilities found" after serenity removal

# Verify port 3456 exposed to localhost only
ss -tlnp | grep 3456
# Should show: LISTEN 0 ... 127.0.0.1:3456 ... (not 0.0.0.0:3456)

# Verify no hardcoded secrets
grep -r "sk_\|SK_\|api.key\|API.KEY" crates/ src/ --include="*.rs"
# Should return: (empty)
```

---

## Compliance Assessment

- **OWASP Top 10**: Vulnerability in A06:2021 (Vulnerable and Outdated Components)
- **CWE Coverage**: CWE-1035 (Implicit Dangerous Semantic), CWE-295 (Improper Certificate Validation)
- **Production Ready**: NO - Critical vulnerabilities block deployment

---

## Conclusion

**VERDICT: FAIL** ✗

This project has unresolved critical security vulnerabilities that must be addressed before any production deployment:

1. **RUSTSEC-2026-0049** - CRL validation bypass (CVE)
2. **Port 3456 Network Exposure** - Unauthorized access risk

The presence of a patched version (0.103.10) in git and multiple remediation commits indicates this was previously detected but not fully resolved. The Cargo.lock still contains vulnerable 0.102.8, indicating the serenity dependency chain was not completely removed.

**Blocking**: Merge/release cannot proceed until these critical issues are resolved.

---

**Report Generated**: 2026-04-07 04:50 CEST
**Audit Scope**: Dependency vulnerabilities, hardcoded secrets, network exposure
**Next Review**: After remediation completion
