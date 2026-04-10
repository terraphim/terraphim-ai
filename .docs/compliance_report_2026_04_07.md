# Compliance Report: terraphim-ai Project

**Report Date**: 2026-04-07
**Auditor**: Vigil (Security Engineer)
**Status**: FAIL - Critical issues found

## Executive Summary

The project has **CRITICAL compliance violations** that prevent release:

1. **CRITICAL**: Unresolved CVE RUSTSEC-2026-0049 in TLS validation (rustls-webpki)
2. **CRITICAL**: License compliance failures (2 unlicensed crates + deprecated SPDX)
3. **HIGH**: Secret metadata leakage in atomic_client logging
4. **MEDIUM**: Multiple unmaintained dependencies

## Findings

### 1. Security Advisories

#### RUSTSEC-2026-0049: TLS Certificate Validation Bug (CRITICAL)

**Severity**: CRITICAL
**Affected Crate**: rustls-webpki 0.102.8
**Issue**: CRL revocation checking logic contains faulty matching that could bypass certificate revocation checks
**Dependency Chain**: serenity 0.12.5 → tokio-tungstenite 0.21.0 → tokio-rustls 0.25.0 → rustls-webpki 0.102.8

**Evidence**:
```
Crate:     rustls-webpki
Version:   0.102.8
Title:     CRLs not considered authoritative by Distribution Point due to faulty matching logic
Date:      2026-03-20
ID:        RUSTSEC-2026-0049
URL:       https://rustsec.org/advisories/RUSTSEC-2026-0049
Solution:  Upgrade to >=0.103.10
```

**Current Status**: Explicitly ignored in deny.toml (line 35) with justification that serenity 0.12 cannot be upgraded
**Risk Assessment**:
- This is a real cryptographic validation weakness
- Affects TLS handshake security
- No amount of feature gating eliminates the risk if the crate is linked
- Workaround (claiming it's "disabled by default") is insufficient for security-critical code

**Remediation Required**: Either upgrade serenity/dependencies to use rustls-webpki >=0.103.10, or remove Discord/serenity integration entirely

---

#### RUSTSEC-2025-0141: Bincode Unmaintained (MEDIUM)

**Severity**: MEDIUM (unmaintained warning)
**Affected Crate**: bincode 1.3.3
**Issue**: Crate is no longer maintained by original author
**Impact**: Used by terraphim_automata, terraphim_rolegraph, persistence layer (redb backend)
**Alternatives**: postcard, rkyv, message-pack

**Remediation**: Evaluate alternative serialization formats or adopt community fork

---

#### RUSTSEC-2025-0134: rustls-pemfile Unmaintained (MEDIUM)

**Severity**: MEDIUM (unmaintained warning)
**Affected Crate**: rustls-pemfile 1.0.4
**Dependency Chain**: hyper-rustls 0.24.2 → reqwest 0.11.27
**Issue**: No longer maintained
**Remediation**: Monitor for security patches; consider upgrade path to rustls-native-certs alternatives

---

#### RUSTSEC-2020-0163: term_size Unmaintained (LOW)

**Severity**: LOW (unmaintained warning)
**Affected Crate**: term_size 0.3.2
**Dependency**: terraphim_validation 0.1.0
**Alternatives**: terminal_size crate
**Remediation**: Replace with terminal_size crate (straightforward migration)

---

### 2. License Compliance

#### Finding: 2 Unlicensed Crates (CRITICAL)

**Violation**: The following internal crates lack license fields in Cargo.toml:

| Crate | Location | Status |
|-------|----------|--------|
| terraphim_ccusage | crates/terraphim_ccusage | NO LICENSE FIELD |
| terraphim_usage | crates/terraphim_usage | NO LICENSE FIELD |

**Evidence**:
```
error[unlicensed]: terraphim_ccusage = 1.16.9 is unlicensed
error[unlicensed]: terraphim_usage = 1.16.9 is unlicensed
```

**Remediation**: Add license field to Cargo.toml for both crates

---

#### Finding: Deprecated SPDX License Identifier (HIGH)

**Violation**: html2md transitive dependency uses deprecated SPDX identifier

| Crate | Issue | Fix |
|-------|-------|-----|
| html2md 0.2.15 | Uses `GPL-3.0+` (deprecated) | Should be `GPL-3.0-or-later` |

**Evidence**:
```
warning[parse-error]: error parsing SPDX license expression
29 │ license = "GPL-3.0+"
   │            ─────── a deprecated license identifier was used
```

**Impact**: html2md is a transitive dependency through terraphim_middleware. Upstream fix required or crate replacement needed

**Remediation**:
- Option A: Upgrade html2md to version with corrected SPDX identifier
- Option B: Replace html2md with alternative (html-escape, comrak, etc.)

---

### 3. Data Protection & GDPR Audit

#### Finding: Secret Metadata Leakage in atomic_client (HIGH)

**Location**: `crates/terraphim_atomic_client/src/types.rs:52`

**Issue**: Secrets logging reveals information about authentication:

```rust
// Line 52-56: Logs secret presence and agent metadata
println!("Found ATOMIC_SERVER_SECRET, length: {}", secret.len());
match Agent::from_base64(&secret) {
    Ok(agent) => {
        println!("Agent created successfully with subject: {}", agent.subject);
```

**Risk**: Even though the secret value is redacted, the logging reveals:
1. Whether authentication secrets exist
2. Secret length (can aid dictionary attacks)
3. Agent subject identifier (may be sensitive)
4. This debug output could be captured in container logs, CI/CD logs, etc.

**Remediation**:
- Remove debug println! statements
- Use debug! macro with conditional compilation instead
- Ensure ATOMIC_SERVER_SECRET is never logged in any form

---

#### Finding: Environment Variable Dumping (MEDIUM)

**Location**: `crates/terraphim_atomic_client/src/types.rs:33-45`

**Issue**: All ATOMIC_* environment variables are printed to stdout:

```rust
for (key, value) in std::env::vars() {
    if key.starts_with("ATOMIC_") {
        println!("Found env var: {} = {}", key, ...);
    }
}
```

**Risk**: Could leak configuration details or partially redacted secrets in logs

**Remediation**: Remove debug environment variable enumeration

---

#### GDPR Data Handling Assessment

**Scope**: No explicit PII processing documented in configuration
**Status**: Requires further investigation of:
- User data retention policies in persistence layer
- Data deletion/right-to-be-forgotten implementation
- Data processing agreements for haystacks (Atlassian, Discourse, etc.)

**Pending Review**:
- Atomic Data integration (stores resources with URLs/subjects)
- Haystack sources (email via JMAP, Confluence, Jira user data)
- Session storage (does it store user sessions long-term?)

---

### 4. Port Exposure Check

**Status**: Not vulnerable on recent scans
- Port 3456 exposure issue from previous sessions has been remediated
- Current default port allocation is dynamic

---

## Summary Table

| Issue | Severity | Category | Status | Blocker |
|-------|----------|----------|--------|---------|
| RUSTSEC-2026-0049 (TLS CVE) | CRITICAL | Security | Unresolved 24+ hours | YES |
| Missing license: terraphim_ccusage | CRITICAL | Compliance | New | YES |
| Missing license: terraphim_usage | CRITICAL | Compliance | New | YES |
| Deprecated SPDX: html2md | HIGH | Compliance | Upstream fix needed | YES |
| Secret logging in atomic_client | HIGH | Security | New finding | YES |
| Env var dumping | MEDIUM | Security | New finding | NO |
| bincode unmaintained | MEDIUM | Supply Chain | Known, accepted risk | NO |
| rustls-pemfile unmaintained | MEDIUM | Supply Chain | Known, accepted risk | NO |
| term_size unmaintained | LOW | Supply Chain | Known, accepted risk | NO |

---

## Verdict

**COMPLIANCE STATUS: FAIL**

**Blocking Issues**:
1. RUSTSEC-2026-0049 persists unresolved in TLS validation chain
2. Two internal crates have no license declarations
3. Transitive dependency (html2md) uses invalid SPDX identifier
4. New: Secret metadata leakage in atomic_client

**Non-Blocking Known Issues**:
- Multiple unmaintained transitive dependencies (with accepted risk)

**Merge Readiness**: NOT APPROVED for release until:
1. RUSTSEC-2026-0049 is resolved via dependency upgrade or feature removal
2. License fields added to terraphim_ccusage and terraphim_usage
3. html2md upgraded or replaced
4. Secret logging removed from atomic_client

**Time to Remediation**: Critical issues have persisted for 24+ hours across multiple audit sessions. Immediate action required.

---

## Audit Trail

- **Previous audit (Session 19)**: Same RUSTSEC-2026-0049 persists + new secret leakage issue found
- **Previous audit (Session 18)**: Same 3 critical issues (licenses, RUSTSEC, port 3456)
- **Status**: No code changes addressing vulnerabilities in past 24 hours

**Pattern**: Critical security and compliance issues remain unaddressed despite repeated audit failures.

---

Generated by Vigil (Security Engineer) on 2026-04-07
