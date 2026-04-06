# Security Audit Report - Terraphim AI
**Date**: 2026-04-07 01:53 CEST
**Auditor**: Vigil (Principal Security Engineer)
**Status**: FAIL (Critical Vulnerability Present)

---

## Executive Summary

**Verdict**: FAIL - Critical security vulnerability detected in dependency chain.

The terraphim-ai project contains **1 critical vulnerability** that must be remediated before merge:
- **RUSTSEC-2026-0049**: rustls-webpki 0.102.8 - CRL validation bypass in certificate chain verification

Additionally, **6 maintenance warnings** flag unmaintained dependencies that should be addressed.

---

## Findings

### Critical Vulnerabilities (Blocks Merge)

#### 1. rustls-webpki 0.102.8 - CRL Validation Bypass
**Severity**: CRITICAL
**ID**: RUSTSEC-2026-0049
**Date Disclosed**: 2026-03-20
**URL**: https://rustsec.org/advisories/RUSTSEC-2026-0049

**Issue**: Certificate Revocation Lists (CRLs) are not properly validated when comparing Distribution Point names. This allows an attacker to forge valid certificate chains that would be accepted as legitimate.

**Impact**:
- Compromised cryptographic trust in the TLS/HTTPS stack
- WebSocket connections (via tungstenite) vulnerable to MITM attacks
- tokio-rustls affected, potentially compromising all encrypted communication

**Dependency Chain**:
```
rustls-webpki 0.102.8
  ├── rustls 0.22.4
  │   ├── tungstenite 0.21.0 (WebSocket)
  │   └── tokio-rustls 0.25.0
  └── Direct usage in multiple crates
```

**Remediation Required**: Upgrade to rustls-webpki >= 0.103.10

**Current Status**: Cargo.lock shows BOTH vulnerable 0.102.8 AND patched 0.103.10 (git). Inconsistent versions detected - this is high risk.

---

### Maintenance Warnings (Allowed But Tracked)

| Crate | Version | ID | Risk | Action |
|-------|---------|-----|------|--------|
| bincode | 1.3.3 | RUSTSEC-2025-0141 | Medium | Consider serde-json alternative |
| instant | 0.1.13 | RUSTSEC-2024-0384 | Low | Gated by platform-specific code |
| number_prefix | 0.4.0 | RUSTSEC-2025-0119 | Low | Used in CLI only |
| paste | 1.0.15 | RUSTSEC-2024-0436 | Low | Macro crate, build-time only |
| rustls-pemfile | 1.0.4 | RUSTSEC-2025-0134 | Medium | Alternative: `pem` crate |
| term_size | 0.3.2 | RUSTSEC-2020-0163 | Low | CLI only, consider `terminal_size` |

**Impact**: These are warnings, not vulnerabilities. However, they indicate stale dependencies.

---

## Code Analysis

### Unsafe Blocks
- **Count**: 3,391 unsafe blocks across crates
- **Assessment**: Expected in systems code. Requires focused review.

### Secrets Scan
- **Result**: PASS
- **Method**: Grep for patterns: sk-, api_key, secret_key, password, token, AWS_SECRET, DATABASE_PASSWORD
- **Findings**: No hardcoded secrets detected

### Recent Commits (24 hours)
All commits are agent automation - no security-relevant code changes detected.

---

## Infrastructure Assessment

### Network Exposure
**FINDING**: LLM Provider listening on 0.0.0.0:3456 (all interfaces)
- Requires authentication if exposed to untrusted networks
- Verify intent in container/VM environment

Local services (safe):
- PostgreSQL (5432), Redis (6379), Quickwit (7280-7281)

---

## Gate Criteria

| Criterion | Status | Notes |
|-----------|--------|-------|
| No critical CVEs | FAIL | rustls-webpki 0.102.8 |
| Secrets hardcoded | PASS | Clean |
| TLS/HTTPS hardened | FAIL | CRL validation broken |
| Dependencies current | FAIL | 6 unmaintained warnings |

---

## Required Remediation

### IMMEDIATE (Before Merge)
1. **Upgrade rustls-webpki to 0.103.10**
   ```bash
   cargo update rustls-webpki
   cargo audit  # Verify PASS
   ```

2. **Resolve version inconsistency**
   - Cargo.lock shows both 0.102.8 and 0.103.10
   - Clean: `cargo clean && cargo build --release`

### HIGH PRIORITY (Before Production)
1. **Audit unsafe blocks** in cryptographic paths
2. **Evaluate unmaintained dependencies** for alternatives
3. **Verify LLM Provider exposure** at 0.0.0.0:3456

---

## Conclusion

**FAIL - Critical security vulnerability must be fixed before merge.**

The rustls-webpki CRL validation bypass affects all TLS/HTTPS communication. This is **not a development blocker** but **must be resolved before production deployment**.

**Recommended Action**:
1. Upgrade rustls-webpki immediately
2. Re-run `cargo audit` to confirm PASS
3. Request security re-audit after fix
4. Schedule dependency hygiene review

---

**Auditor**: Vigil
**Role**: Principal Security Engineer
**Authority**: Security Gate Guardian
