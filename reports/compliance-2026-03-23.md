# Security Compliance Report

**Project:** terraphim-ai  
**Date:** 2026-03-23  
**Auditor:** Vigil, Principal Security Engineer  
**Classification:** CONFIDENTIAL - Internal Use Only

---

## Executive Summary

**OVERALL POSTURE: CRITICAL RISK**

This compliance audit has identified **4 active security vulnerabilities** in the dependency supply chain that require immediate remediation. License compliance is acceptable with minor warnings. Data handling practices show awareness of privacy concerns but lack comprehensive GDPR compliance documentation.

**Immediate Actions Required:**
1. Upgrade rustls-webpki to >=0.103.10 (2 instances)
2. Upgrade tar to >=0.4.45
3. Replace unmaintained term_size with terminal_size
4. Document data retention policies for GDPR compliance

---

## 1. License Compliance

### Status: PASS (with warnings)

**Tool:** cargo deny check licenses

### Findings

| Severity | Finding | Details | Remediation |
|----------|---------|---------|-------------|
| WARNING | Deprecated SPDX identifier | html2md v0.2.15 uses deprecated `GPL-3.0+` instead of `GPL-3.0-or-later` | Upstream fix required; consider fork or replacement |
| INFO | Unused license allowance | OpenSSL license not encountered in dependency tree | No action - defensive configuration |
| INFO | Unused license allowance | Unicode-DFS-2016 license not encountered | No action - defensive configuration |

### Dependency Tree Impact

```
html2md v0.2.15 (GPL-3.0+) - DEPRECATED IDENTIFIER
  └── terraphim_middleware v1.13.0
      ├── terraphim_agent v1.13.0
      ├── terraphim_mcp_server v1.0.0
      ├── terraphim_server v1.13.0
      └── terraphim_service v1.4.10
          └── [9 downstream crates]
```

**Risk Assessment:** Low - License is GPL-3.0 compatible, only the SPDX expression format is deprecated. No legal compliance risk.

---

## 2. Supply Chain Security

### Status: CRITICAL - IMMEDIATE ACTION REQUIRED

**Tool:** cargo deny check advisories

### Critical Vulnerabilities

#### VULN-001: CRL Revocation Bypass (RUSTSEC-2026-0049)
- **Severity:** HIGH
- **CVSS Estimate:** 7.5 (High)
- **Affected Crates:** rustls-webpki 0.102.8, 0.103.9
- **Attack Vector:** Network - Certificate validation bypass

**Description:**  
When a certificate has multiple `distributionPoint` entries, only the first is considered against each CRL's `IssuingDistributionPoint`. This allows revoked certificates to be accepted if `UnknownStatusPolicy::Allow` is configured.

**Impact:**
- Man-in-the-middle attacks possible with compromised CA
- Revoked credentials may remain usable
- Affects all TLS connections using rustls-webpki

**Affected Code Paths:**
```
rustls-webpki 0.102.8
  └── rustls v0.22.4
      ├── tokio-rustls v0.25.0
      │   └── tokio-tungstenite v0.21.0
      │       └── serenity v0.12.5 (Discord bot functionality)

rustls-webpki 0.103.9
  └── rustls v0.23.37
      ├── hyper-rustls v0.27.7
      │   ├── octocrab v0.49.5 (GitHub API)
      │   └── reqwest v0.12.28 (HTTP client - WIDESPREAD)
      └── tokio-rustls v0.26.4
```

**Remediation:**
```bash
cargo update -p rustls-webpki
```

**Verification:**
```bash
cargo deny check advisories 2>&1 | grep -E "(RUSTSEC-2026-0049|rustls-webpki)"
```

---

#### VULN-002: Directory Traversal via Symlink (RUSTSEC-2026-0067)
- **Severity:** HIGH
- **CVSS Estimate:** 7.1 (High)
- **Affected Crate:** tar 0.4.44
- **Attack Vector:** Local - Archive extraction

**Description:**  
The `unpack_dir` function uses `fs::metadata()` which follows symbolic links. A crafted tarball with a symlink followed by a directory entry of the same name causes chmod to be applied to the symlink target outside the extraction root.

**Impact:**
- Arbitrary directory permission modification
- Potential privilege escalation
- Affects terraphim_update crate (self-update functionality)

**Affected Code Paths:**
```
tar v0.4.44
  ├── self_update v0.42.0
  │   └── terraphim_update v1.5.0 (auto-updater)
  └── terraphim_update v1.5.0
      ├── terraphim-cli v1.13.0
      └── terraphim_agent v1.13.0
```

**Remediation:**
```bash
cargo update -p tar
```

---

#### VULN-003: PAX Header Size Mishandling (RUSTSEC-2026-0068)
- **Severity:** MEDIUM
- **CVSS Estimate:** 5.9 (Medium)
- **Affected Crate:** tar 0.4.44
- **Attack Vector:** Local - Archive parsing inconsistency

**Description:**  
When the base header size is nonzero, PAX size headers are incorrectly skipped. This creates parsing inconsistencies between tar-rs and other implementations (Go archive/tar, astral-tokio-tar).

**Impact:**
- Inconsistent archive interpretation
- Potential for smuggled content
- Cross-tool incompatibility

**Affected Code Paths:** Same as VULN-002

**Remediation:** Same as VULN-002 (upgrade tar to >=0.4.45)

---

#### VULN-004: Unmaintained Dependency (RUSTSEC-2020-0163)
- **Severity:** MEDIUM
- **Affected Crate:** term_size 0.3.2
- **Status:** Unmaintained since 2020

**Description:**  
The term_size crate is no longer maintained. No security patches will be forthcoming.

**Affected Code Paths:**
```
term_size v0.3.2
  └── terraphim_validation v0.1.0
```

**Remediation:**
Replace with actively maintained `terminal_size` crate:
```toml
# Cargo.toml
[dependencies]
terminal_size = "0.4"
```

---

### Advisory Exception Review

The following advisories are explicitly ignored in `deny.toml` but were not encountered:

| Advisory | Status | Assessment |
|----------|--------|------------|
| RUSTSEC-2021-0141 | Not triggered | Likely no longer in dependency tree - review for removal |
| RUSTSEC-2021-0145 | Not triggered | Likely no longer in dependency tree - review for removal |
| RUSTSEC-2024-0375 | Not triggered | Likely no longer in dependency tree - review for removal |

**Recommendation:** Review and remove obsolete exceptions from deny.toml to reduce technical debt.

---

## 3. GDPR & Data Handling Compliance

### Status: PARTIAL - POLICY GAPS IDENTIFIED

### 3.1 Data Processing Activities

| Activity | Status | GDPR Article | Finding |
|----------|--------|--------------|---------|
| Session logging | ACTIVE | Art. 5(1)(c) - Data minimization | Secret redaction implemented; no retention policy documented |
| API token storage | ACTIVE | Art. 32 - Security | Tokens in config files; no encryption at rest identified |
| LLM token tracking | ACTIVE | Art. 5(1)(b) - Purpose limitation | Usage metrics collected; purpose documented |
| Learning capture | ACTIVE | Art. 5(1)(d) - Accuracy | Secret redaction active; user correction mechanism not identified |
| Self-update | ACTIVE | Art. 7 - Consent | No explicit consent mechanism for update checks |

### 3.2 Secret Redaction Assessment

**Implementation:** `crates/terraphim_agent/src/learnings/redaction.rs`

**Strengths:**
- Comprehensive regex patterns for common secrets
- Environment variable value stripping
- AWS, OpenAI, Slack, GitHub token patterns
- Connection string redaction

**Coverage Gaps:**
```rust
// Current patterns cover:
- AWS Access Keys (AKIA...)
- AWS Secret Keys (40 char base64)
- OpenAI keys (sk-...)
- Slack tokens (xoxb-...)
- GitHub tokens (ghp_, gho_)
- Database connection strings

// Not covered:
- Azure service principals
- GCP service account keys
- JWT tokens (generic pattern)
- Private keys (PEM format)
- Kubernetes secrets
- Docker registry credentials
```

**Recommendation:** Expand SECRET_PATTERNS to include:
```rust
(r"eyJ[A-Za-z0-9-_]*\.eyJ[A-Za-z0-9-_]*\.[A-Za-z0-9-_]*", "[JWT_REDACTED]"),
(r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----", "[PRIVATE_KEY_REDACTED]"),
(r"\{[\s\"']*type[\"':\s]*service_account", "[GCP_SERVICE_ACCOUNT_REDACTED]"),
```

### 3.3 Data Retention Findings

**Current State:**
- No documented data retention policy
- Session logs: Indefinite (filesystem-based)
- Update history: Persistent JSON file
- Procedure store: Persistent but supports deletion

**GDPR Requirements Not Met:**
- **Art. 17 (Right to erasure):** No automated mechanism for complete user data removal
- **Art. 20 (Data portability):** No export functionality identified for user data
- **Art. 5(1)(e) (Storage limitation):** No automatic data purging

**Required Actions:**

1. **Implement retention policies:**
```rust
// Example: crates/terraphim_types/src/policy.rs
pub struct DataRetentionPolicy {
    pub session_logs_days: u32,      // Suggested: 90 days
    pub update_history_days: u32,    // Suggested: 365 days
    pub learning_cache_days: u32,    // Suggested: 30 days
    pub auto_purge_enabled: bool,
}
```

2. **Add data export capability:**
```rust
pub async fn export_user_data(user_id: &str) -> Result<ExportArchive> {
    // Collect all user-associated data
    // Package in standard format (JSON/CSV)
    // Provide download mechanism
}
```

3. **Implement deletion hooks:**
```rust
pub async fn delete_all_user_data(user_id: &str) -> Result<DeletionReport> {
    // Remove from all stores
    // Verify deletion
    // Generate compliance report
}
```

### 3.4 Authentication & Authorization

**Findings:**

| Component | Token Storage | Encryption | Risk |
|-----------|--------------|------------|------|
| Gitea tracker | Config file (YAML) | None at rest | Medium |
| GitHub API | Config file (YAML) | None at rest | Medium |
| Discord bot | Config file (YAML) | None at rest | Medium |
| OpenAI API | Config file (YAML) | None at rest | High (broad permissions) |

**Risk Assessment:**
- Config files contain plaintext API tokens
- File permissions not validated on read
- No key rotation mechanism
- Tokens may be captured in shell history or logs

**Remediations:**

1. **Implement keyring integration:**
```rust
use keyring::Entry;

pub fn store_token_securely(service: &str, token: &str) -> Result<()> {
    let entry = Entry::new(service, "default")?;
    entry.set_password(token)?;
    Ok(())
}
```

2. **Add file permission checks:**
```rust
use std::os::unix::fs::PermissionsExt;

pub fn validate_config_permissions(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path)?;
    let mode = metadata.permissions().mode();
    
    if mode & 0o077 != 0 {
        return Err("Config file has overly permissive permissions".into());
    }
    Ok(())
}
```

---

## 4. Crate-by-Crate Security Assessment

### 4.1 High-Risk Crates

| Crate | Risk Level | Concerns |
|-------|------------|----------|
| terraphim_agent | HIGH | Secret redaction gaps, unencrypted config |
| terraphim_update | HIGH | tar vulnerabilities (VULN-002, VULN-003) |
| terraphim_tracker | MEDIUM | Token storage in config |
| terraphim_sessions | MEDIUM | No retention policy |
| terraphim_config | MEDIUM | Sensitive data in YAML |

### 4.2 Positive Security Controls

| Control | Implementation | Effectiveness |
|---------|---------------|---------------|
| Execution guards | terraphim_tinyclaw | Blocks dangerous operations (rm -rf /, > /dev/sda) |
| Secret redaction | terraphim_agent::learnings | Good coverage for common patterns |
| TLS everywhere | rustls usage | Strong default crypto |
| Dependency auditing | cargo-deny | Properly configured |

---

## 5. Recommendations

### Immediate (24-48 hours)

1. [ ] Upgrade rustls-webpki: `cargo update -p rustls-webpki`
2. [ ] Upgrade tar: `cargo update -p tar`
3. [ ] Verify fixes: `cargo deny check advisories`
4. [ ] File security issue for term_size replacement

### Short-term (1-2 weeks)

5. [ ] Expand secret redaction patterns (JWT, PEM keys, GCP)
6. [ ] Document data retention policy
7. [ ] Implement config file permission validation
8. [ ] Review and clean up deny.toml exceptions

### Medium-term (1 month)

9. [ ] Implement keyring-based token storage
10. [ ] Add automated data purging for old sessions
11. [ ] Create data export functionality for GDPR compliance
12. [ ] Add encryption at rest for sensitive config fields

### Long-term (3 months)

13. [ ] Implement comprehensive GDPR compliance framework
14. [ ] Add consent management for data collection
15. [ ] Conduct penetration testing
16. [ ] Establish security incident response procedures

---

## 6. Compliance Matrix

| Requirement | Status | Evidence | Gap |
|-------------|--------|----------|-----|
| **Supply Chain** |
| Dependency vulnerability scanning | PASS | cargo-deny integrated | - |
| License compliance | PASS | SPDX compliance | Deprecated identifier warning |
| Security advisory monitoring | PASS | RUSTSEC database | - |
| **Data Protection** |
| Secret redaction | PARTIAL | Implemented | Coverage gaps identified |
| Encryption in transit | PASS | rustls default | - |
| Encryption at rest | FAIL | Not implemented | No evidence found |
| Data retention policy | FAIL | Not documented | No policy defined |
| Right to erasure | FAIL | No mechanism | No automated deletion |
| Data portability | FAIL | No export feature | No evidence found |
| **Access Control** |
| Secure token storage | FAIL | Plaintext configs | No keyring integration |
| Config file permissions | FAIL | No validation | No checks implemented |
| **Operational** |
| Update mechanism security | CRITICAL | tar vulnerable | VULN-002, VULN-003 |
| TLS certificate validation | CRITICAL | CRL bypass | VULN-001 |

---

## 7. Appendices

### Appendix A: Vulnerability References

| ID | Advisory | URL |
|----|----------|-----|
| VULN-001 | RUSTSEC-2026-0049 | https://rustsec.org/advisories/RUSTSEC-2026-0049 |
| VULN-002 | RUSTSEC-2026-0067 | https://rustsec.org/advisories/RUSTSEC-2026-0067 |
| VULN-003 | RUSTSEC-2026-0068 | https://rustsec.org/advisories/RUSTSEC-2026-0068 |
| VULN-004 | RUSTSEC-2020-0163 | https://rustsec.org/advisories/RUSTSEC-2020-0163 |

### Appendix B: Relevant GDPR Articles

| Article | Title | Applicability |
|---------|-------|---------------|
| Art. 5 | Principles | Data minimization, storage limitation |
| Art. 7 | Conditions for consent | Update checks |
| Art. 17 | Right to erasure | No mechanism implemented |
| Art. 20 | Right to data portability | No export feature |
| Art. 25 | Data protection by design | Partial - redaction exists |
| Art. 32 | Security of processing | Encryption gaps identified |

### Appendix C: Commands for Reproduction

```bash
# License check
cargo deny check licenses 2>&1 | tee reports/licenses-output.txt

# Advisory check
cargo deny check advisories 2>&1 | tee reports/advisories-output.txt

# Full report
cargo deny check 2>&1 | tee reports/full-deny-output.txt

# Dependency tree for affected crates
cargo tree -p rustls-webpki
cargo tree -p tar
cargo tree -p term_size
```

---

## Sign-off

**Auditor:** Vigil (Security Engineer)  
**Review Date:** 2026-03-23  
**Next Review:** 2026-06-23 (Quarterly)  
**Status:** CRITICAL - Requires immediate remediation

**Distribution:** Engineering Leadership, Compliance Officer, Security Team

---

*"Assume compromise until proven otherwise." - Vigil*
