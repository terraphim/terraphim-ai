# Compliance Audit Report
**Date**: 2026-04-07 02:55 CEST
**Project**: terraphim-ai
**Branch**: fix/worktree-shared-target
**Auditor**: Vigil (Security Engineer)
**Status**: **FAIL** - Critical license compliance violations found

---

## Executive Summary

The compliance audit identified **critical license violations** and one **known vulnerability** in the supply chain. The system currently fails verification due to unlicensed internal crates.

| Category | Status | Finding Count |
|----------|--------|---|
| **License Compliance** | **FAIL** | 3 violations (2 critical, 1 warning) |
| **Dependency Advisories** | **WARN** | 1 yanked dependency |
| **Security Vulnerabilities** | **WARN** | 1 ignored CVE (mitigated) |
| **GDPR/Data Handling** | **PASS** | No critical data exposure patterns found |

---

## 1. License Compliance Analysis

### CRITICAL: Unlicensed Internal Crates

Two internal crates are missing license declarations in their Cargo.toml files:

#### 1.1 terraphim_ccusage v1.16.9
- **Severity**: CRITICAL
- **Issue**: No license field in Cargo.toml
- **Location**: `/home/alex/terraphim-ai/crates/terraphim_ccusage/Cargo.toml`
- **Impact**: Cannot be published to crates.io; unclear license obligations for consumers
- **Remediation**: Add `license` field to Cargo.toml, e.g., `license = "Apache-2.0 OR MIT"`

#### 1.2 terraphim_usage v1.16.9
- **Severity**: CRITICAL
- **Issue**: No license field in Cargo.toml
- **Location**: `/home/alex/terraphim-ai/crates/terraphim_usage/Cargo.toml`
- **Impact**: Cannot be published to crates.io; unclear license obligations for consumers
- **Remediation**: Add `license` field to Cargo.toml, e.g., `license = "Apache-2.0 OR MIT"`

### WARNING: Deprecated License Identifier

#### 1.3 html2md v0.2.15
- **Severity**: WARNING (transitive dependency)
- **Issue**: Uses deprecated SPDX identifier "GPL-3.0+" instead of "GPL-3.0-or-later"
- **Location**: Pulled from registry, transitive dep of terraphim_middleware
- **Impact**: May cause issues in some compliance systems; maintainer should update
- **Remediation**: Monitor for html2md update; consider alternative if blocking

### License Allow List Status
- **Allowed licenses**: MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, MPL-2.0, CC0-1.0, Unicode, BSL-1.0, 0BSD, OpenSSL, Unlicense, GPL-3.0-or-later, AGPL-3.0-or-later, CDLA-Permissive-2.0, bzip2-1.0.6
- **Unmatched in deny.toml**: OpenSSL, Unicode-DFS-2016 (minor configuration issue, not blocking)

---

## 2. Dependency Supply Chain Analysis

### 2.1 Yanked Dependency: fastrand v2.4.0
- **Severity**: WARNING
- **Issue**: Version 2.4.0 is yanked (deprecated by maintainer)
- **Transitive Path**: fastrand → backon → opendal → terraphim_config, terraphim_persistence, terraphim_service
- **Impact**: Should update to 2.4.1 or later via transitive updates
- **Remediation**: Run `cargo update -p fastrand` to pull latest patch version
- **Status**: Non-blocking (warning severity); version works despite yanked status

### 2.2 Ignored Vulnerabilities in deny.toml
The following CVEs are explicitly allowed/ignored (with documented justification):

| CVE ID | Issue | Justification | Status |
|--------|-------|--------------|--------|
| RUSTSEC-2026-0049 | rustls-webpki CRL revocation bypass | Transitive via serenity 0.12 → hyper-rustls 0.24 → rustls 0.21. Disabled by default (discord removed from tinyclaw features). TODO: Remove once serenity 0.13+ released | MITIGATED |
| RUSTSEC-2023-0071 | RSA Marvin Attack | Transitive via octocrab → jsonwebtoken → rsa. No safe upgrade available. RustCrypto team migrating to constant-time impl. | DOCUMENTED |
| RUSTSEC-2021-0145 | atty unaligned read | Transitive dep; only affects Windows with custom allocators | DOCUMENTED |
| RUSTSEC-2021-0141 | dotenv unmaintained | Used by atlassian_haystack. TODO: Replace with dotenvy | TODO |
| RUSTSEC-2024-0375 | atty unmaintained | Used by terraphim_agent. TODO: Migrate to std::io::IsTerminal | TODO |
| RUSTSEC-2025-0141 | bincode unmaintained | Used by redb persistence backend. TODO: Evaluate alternatives | TODO |
| RUSTSEC-2020-0163 | term_size unmaintained | Transitive via terraphim_validation. TODO: Replace with terminal_size | TODO |

**Assessment**: All ignored CVEs have documented TODOs. RUSTSEC-2026-0049 is the most critical but is mitigated by feature flags (discord disabled by default in tinyclaw).

---

## 3. GDPR & Data Handling Audit

### 3.1 Sensitive Data Logging Analysis

**Examined**: Secret/password logging patterns, credential handling, PII exposure

#### PASS: Secure Credential Handling
- ✅ **1Password CLI Integration** (`terraphim_onepassword_cli`): Secrets managed via secure 1Password CLI integration; not embedded
- ✅ **Matrix Authentication**: Passwords passed as API parameters, not logged
- ✅ **Atomic Server Secrets**: Secret length logged but not value: `"Found ATOMIC_SERVER_SECRET, length: {}"` ✓
- ✅ **Session Keys**: Only logged at debug level with key identifiers, not full tokens

#### WARN: Error Logging Patterns
- ⚠️ **terraphim_atomic_client/src/types.rs**: eprintln! used for error messages about secret presence (not the secret itself, but reveals secret existence)
  - Line: `eprintln!("ATOMIC_SERVER_SECRET not set: {}", e);`
  - Recommendation: Use log::warn! instead of eprintln! for consistency with production logging

- ⚠️ **terraphim_onepassword_cli/src/lib.rs**: Error logging for secrets resolution
  - Line: `log::error!("Failed to resolve secret {}: {}", reference, e);`
  - Status: OK - logs reference name, not secret value; appropriate error level

### 3.2 Data Protection Patterns

**Architecture Analysis**:
- **Persistence Layer**: Supports transparent cache write-back for multi-backend storage (memory → dashmap → sqlite → s3)
  - ✅ Respects operator ordering by speed
  - ✅ Supports data compression (zstd) for objects > 1MB
  - ✅ Schema evolution: cached data that fails to deserialize is deleted and refetched

- **Configuration Management**: Role-based configuration with extra fields for sensitive data
  - ✅ Settings separated into TOML (system) and JSON (role config)
  - ⚠️ Ensure credentials in settings are not logged in debug output

### 3.3 GDPR Compliance Assessment

| Requirement | Finding | Status |
|------------|---------|--------|
| No automatic logging of personal data | Secrets masked in logs ✓ | PASS |
| Data deletion capability | Persistence layer supports deletion ✓ | PASS |
| Data encryption (at rest) | No explicit mention; depends on operator backend | WARN |
| Data minimization | Settings structure allows per-role config (minimal scope) | PASS |
| Consent/Legal basis documented | Not found in audit scope | N/A |
| Data subject rights (access/delete) | Persistence API supports operations | PASS |

**Recommendation**: Document encryption-at-rest strategy for backends (particularly S3 backend with KMS).

---

## 4. Summary of Violations

### Blockers (Must Fix Before Merge)
1. **terraphim_ccusage - Missing License** → Add to Cargo.toml
2. **terraphim_usage - Missing License** → Add to Cargo.toml

### Warnings (Should Fix, Non-Blocking)
1. **html2md v0.2.15 - Deprecated License** → Monitor for maintainer update
2. **fastrand v2.4.0 - Yanked** → Run `cargo update -p fastrand`
3. **eprintln! for secrets** → Replace with log::warn! in terraphim_atomic_client

### Mitigated Issues (Documented)
- RUSTSEC-2026-0049: rustls-webpki CRL bypass - Disabled by default (tinyclaw feature), serenity upgrade pending

---

## 5. Remediation Steps

### Immediate (Required for Merge)
```bash
# Step 1: Add licenses to terraphim_ccusage
echo 'license = "Apache-2.0"' >> crates/terraphim_ccusage/Cargo.toml

# Step 2: Add licenses to terraphim_usage
echo 'license = "Apache-2.0"' >> crates/terraphim_usage/Cargo.toml

# Step 3: Re-run compliance check
cargo deny check licenses
cargo deny check advisories
```

### Short Term (Before Next Release)
```bash
# Update yanked dependency
cargo update -p fastrand

# Fix logging patterns
# File: crates/terraphim_atomic_client/src/types.rs
# Replace: eprintln!(...) with log::warn!(...)
```

### Long Term (TODOs in deny.toml)
- [ ] Upgrade serenity to 0.13+ (removes rustls-webpki 0.21 constraint)
- [ ] Migrate atty → std::io::IsTerminal
- [ ] Replace dotenv → dotenvy in haystack_atlassian
- [ ] Evaluate bincode alternatives (postcard, rkyv)
- [ ] Replace term_size → terminal_size
- [ ] Document encryption-at-rest strategy for persistence backends

---

## 6. Compliance Verdict

### **FAIL** - Merge Blocked

**Reason**: Two internal crates lack required license declarations. This is a compliance requirement that prevents publication and creates legal ambiguity.

**Blocking Issues**:
- ❌ terraphim_ccusage v1.16.9 - UNLICENSED
- ❌ terraphim_usage v1.16.9 - UNLICENSED

**Gate Status**: Cannot proceed to merge until license fields are added and `cargo deny check licenses` passes.

---

## 7. Evidence Files

- License check log: `/tmp/cargo_deny_licenses.txt`
- Advisory check log: `/tmp/cargo_deny_advisories.txt`
- Source inspection: Reviewed terraphim_settings, terraphim_persistence, terraphim_atomic_client, terraphim_tinyclaw, terraphim_onepassword_cli

---

**Audit Timestamp**: 2026-04-07 02:55 CEST
**Next Review**: After license remediation
**Escalation**: Merge blocked until critical violations resolved
