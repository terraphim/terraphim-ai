# Compliance Audit Report: terraphim-ai

**Status**: FAIL - Critical Compliance Violations Identified
**Date**: 2026-04-07
**Auditor**: Vigil (Security Engineer)
**Issue**: Triggered by @adf:compliance-watchdog on issue #438

## Executive Summary

The terraphim-ai project has **3 critical blocking issues** that prevent compliance sign-off:

1. **License Compliance FAILED** - 2 internal crates unlicensed + 1 deprecated GPL-3.0+ format
2. **Data Security MEDIUM RISK** - Logging infrastructure exposes sensitive information
3. **Supply Chain MEDIUM RISK** - yanked dependency (fastrand) in lock file

---

## 1. LICENSE COMPLIANCE - CRITICAL FAILURES

### 1.1 Unlicensed Crates (BLOCKING)

| Crate | Status | Evidence | Action Required |
|-------|--------|----------|-----------------|
| `terraphim_ccusage v1.16.9` | **UNLICENSED** | No license field in Cargo.toml | Add `license = "MIT"` or appropriate license to Cargo.toml |
| `terraphim_usage v1.16.9` | **UNLICENSED** | No license field in Cargo.toml | Add `license = "MIT"` or appropriate license to Cargo.toml |

**Impact**: Cannot distribute, license compliance failures, potential GPL violations

**Remediation**:
```toml
# In crates/terraphim_ccusage/Cargo.toml
[package]
license = "MIT"  # or appropriate license

# In crates/terraphim_usage/Cargo.toml
[package]
license = "MIT"  # or appropriate license
```

### 1.2 Deprecated License Format (HIGH)

| Dependency | Current Format | Required Format | Status |
|------------|---------------|-----------------|--------|
| `html2md v0.2.15` | `GPL-3.0+` | `GPL-3.0-or-later` | **INVALID SPDX** |

**Impact**: Deprecated SPDX identifier rejected by SPDX spec; forces unmaintained version

**Evidence**:
```
warning[parse-error]: error parsing SPDX license expression
   ┌─ html2md-0.2.15/Cargo.toml:29:12
     license = "GPL-3.0+"  <-- deprecated
```

**Remediation**: Upgrade to html2md v0.2.16+ (if available) with corrected SPDX identifier, or pin with explicit allow in deny.toml

---

## 2. DATA SECURITY - MEDIUM RISK

### 2.1 Secrets Logging via stdout/stderr

**File**: `crates/terraphim_atomic_client/src/types.rs`
**Severity**: Medium (Information Disclosure)
**Evidence**:
```rust
println!("Found ATOMIC_SERVER_SECRET, length: {}", secret.len());
eprintln!("Failed to create agent from secret: {}", e);
```

**Risk**: Secret metadata (length) exposed to stdout/stderr; appears in CI logs, container logs, etc.

**Remediation**:
```rust
// Instead of println!/eprintln!, use tracing with DEBUG level
tracing::debug!(secret_length = secret.len(), "ATOMIC_SERVER_SECRET loaded");
tracing::debug!(error = ?e, "Failed to create agent from secret");
```

### 2.2 Base64 Auth Header Construction (Informational)

**Files**:
- `crates/haystack_atlassian/src/confluence.rs` (3 occurrences)
- `crates/haystack_atlassian/src/jira.rs` (1 occurrence)

**Pattern**:
```rust
let auth = format!("{}:{}", username, token);  // Basic auth
```

**Assessment**: Correct pattern for Basic auth (proper use of HTTP Authorization header). No logging of credentials detected.

---

## 3. SUPPLY CHAIN - MEDIUM RISK

### 3.1 Yanked Dependency

**Package**: `fastrand v2.4.0` (YANKED)
**Status**: Warning (not blocking)
**Impact**: Transitive dependency via `opendal -> backon -> fastrand`

**Evidence**:
```
warning[yanked]: detected yanked crate
    fastrand v2.4.0
```

**Remediation**:
```bash
cargo update -p fastrand  # Auto-upgrade to latest non-yanked version
```

---

## 4. REGULATORY COMPLIANCE - GDPR/DATA HANDLING

### 4.1 Positive Findings ✓

- ✓ **1Password CLI Integration**: Proper secret management via 1Password vault
- ✓ **No hardcoded credentials**: All sensitive data through environment or 1Password
- ✓ **Multi-backend storage**: Proper data abstraction layer with cache warm-up
- ✓ **Basic auth headers**: Correct HTTP Authorization handling, no credential logging
- ✓ **Tracing infrastructure**: Proper use of tracing crate for controlled logging

### 4.2 GDPR Considerations

**Finding**: System is privacy-first by design but GDPR compliance requires documentation:

- User data retention policies not found in code
- Data export/deletion mechanisms not documented
- GDPR user rights (access, deletion) implementation not verified

**Recommendation**: Create GDPR Data Protection Impact Assessment (DPIA) document that includes:
- Data retention schedules
- User data export procedures
- Right to deletion implementation
- Data subject access request (DSAR) handling

---

## 5. ADVISORY DATABASE STATUS

| Advisory | Status | Details | Action |
|----------|--------|---------|--------|
| RUSTSEC-2023-0071 | Ignored (documented) | RSA Marvin Attack; no safe upgrade | Keep ignored; TODO: monitor octocrab updates |
| RUSTSEC-2021-0145 | Ignored (documented) | atty Windows unaligned read | Low priority; plan to replace with is-terminal |
| RUSTSEC-2024-0375 | Ignored (documented) | atty unmaintained | Priority: migrate to std::io::IsTerminal |
| RUSTSEC-2025-0141 | Ignored (documented) | bincode unmaintained | Medium priority; evaluate postcard/rkyv |
| RUSTSEC-2021-0141 | Ignored (documented) | dotenv unmaintained | Low priority; plan to replace with dotenvy |
| RUSTSEC-2020-0163 | Ignored (documented) | term_size unmaintained | Low priority; replace with terminal_size |
| RUSTSEC-2026-0049 | Ignored (documented) | **rustls-webpki CRL bypass** | **CRITICAL**: See note below |

**CRITICAL ADVISORY**: RUSTSEC-2026-0049 (rustls-webpki CRL revocation bypass)
- **Status**: Transitive via `serenity 0.12 -> hyper-rustls 0.24 -> rustls 0.21`
- **Disabled by**: Discord disabled from tinyclaw default features
- **Blocker**: Cannot override without serenity upgrade to 0.13+
- **Evidence**: `deny.toml:35 RUSTSEC-2026-0049`
- **Workaround**: ACTIVE (Discord disabled) but should monitor serenity releases

---

## COMPLIANCE VERDICT: FAIL

### Blocking Issues (Must Fix Before Merge)

1. ❌ **terraphim_ccusage**: Add license field to Cargo.toml
2. ❌ **terraphim_usage**: Add license field to Cargo.toml
3. ❌ **html2md v0.2.15**: Deprecated SPDX identifier (upgrade or clarify in deny.toml)

### Non-Blocking (Should Fix in Next Sprint)

1. 🟡 Secrets logging via println!/eprintln! → use tracing instead
2. 🟡 Yanked fastrand → `cargo update -p fastrand`
3. 🟡 GDPR documentation → Create DPIA
4. 🟡 Monitor serenity upgrade for RUSTSEC-2026-0049 remediation

---

## Recommendations for Issue #438

To achieve compliance:

### Immediate (Required for Merge)
```bash
# Fix unlicensed crates
echo 'license = "MIT"' >> crates/terraphim_ccusage/Cargo.toml
echo 'license = "MIT"' >> crates/terraphim_usage/Cargo.toml

# Fix deprecated license format
# Option A: Add clarification to deny.toml
# Option B: Upgrade html2md (if version available)
```

### Short-term (Next Sprint)
```bash
# Remove dangerous logging
# crates/terraphim_atomic_client/src/types.rs: Replace println!/eprintln! with tracing

# Fix yanked dependency
cargo update -p fastrand
```

### Medium-term (Quarterly)
- Create GDPR DPIA documentation
- Plan migration of deprecated dependencies (atty, bincode, etc.)
- Upgrade serenity to 0.13+ for RUSTSEC-2026-0049 fix

---

## Test Commands for Verification

```bash
# After fixes, verify compliance with:
cargo deny check licenses  # Should return: licenses ok
cargo deny check advisories  # Should return: advisories ok

# Verify no secrets in logs
rg "println!.*SECRET|eprintln!.*token|format!.*password" crates/ --type rust
```

---

## Files Audited

### License/Advisory Checks
- Ran: `cargo deny check licenses`
- Ran: `cargo deny check advisories`
- Config: `deny.toml` - reviewed advisory ignores and license allowlist

### Security/Privacy Audit
- Searched: GDPR/PII patterns across crates
- Reviewed: Secret management (1Password integration)
- Checked: Logging patterns for credential exposure
- Analyzed: HTTP auth patterns and data handling

### Files Analyzed
- `crates/terraphim_persistence/src/lib.rs` - Multi-backend storage, cache warm-up ✓
- `crates/terraphim_onepassword_cli/src/lib.rs` - Secret management integration ✓
- `crates/terraphim_atomic_client/src/types.rs` - Secrets logging issue ❌
- `crates/haystack_atlassian/src/*.rs` - Basic auth patterns ✓
- `crates/terraphim_*` - GDPR/PII pattern search

