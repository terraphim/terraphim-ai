# Terraphim AI Compliance Report

**Report Date:** 2026-03-27
**Project:** terraphim-ai
**Version:** 1.14.0

## Executive Summary

This report details the compliance status of the terraphim-ai project, covering license compliance, dependency security advisories, and GDPR/data handling patterns.

### Overall Status: ⚠️ ACTION REQUIRED

| Check | Status | Priority |
|-------|--------|----------|
| License Compliance | ❌ FAILED | HIGH |
| Security Advisories | ❌ FAILED | CRITICAL |
| GDPR/Data Handling | ⚠️ REVIEW NEEDED | MEDIUM |

---

## 1. License Compliance

### Status: ❌ FAILED

**Command:** `cargo deny check licenses`

### Critical Issues

#### 1.1 Unlicensed Crate: fcctl-core v0.1.0
- **Severity:** ERROR
- **Location:** `terraphim_rlm v1.14.0` depends on `fcctl-core v0.1.0`
- **Issue:** No license field specified in manifest
- **Source:** Git dependency `https://github.com/terraphim/firecracker-rust?branch=main`
- **Action Required:** Add license declaration to fcctl-core crate or replace dependency

#### 1.2 Deprecated License Identifier: html2md v0.2.15
- **Severity:** WARNING
- **License:** GPL-3.0+ (deprecated identifier)
- **Issue:** Uses deprecated SPDX license expression
- **Dependency Path:** `terraphim_middleware` → `terraphim_agent` → `terraphim_server` → `terraphim_validation`
- **Action Required:** Update to `GPL-3.0-or-later` or replace with MIT/Apache-2.0 alternative

### Allowed Licenses (from deny.toml)
The project permits the following licenses:
- MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception
- BSD-2-Clause, BSD-3-Clause, ISC, Zlib
- MPL-2.0, CC0-1.0, Unicode-3.0, Unicode-DFS-2016
- BSL-1.0, 0BSD, OpenSSL, Unlicense
- GPL-3.0-or-later, AGPL-3.0-or-later, CDLA-Permissive-2.0

### Recommendations

1. **Immediate:** Add license declaration to fcctl-core or remove dependency
2. **Short-term:** Replace html2md with MIT/Apache-2.0 licensed alternative
3. **Ongoing:** Run `cargo deny check licenses` in CI/CD pipeline

---

## 2. Security Advisories

### Status: ❌ FAILED

**Command:** `cargo deny check advisories`

### Critical Vulnerability

#### 2.1 RUSTSEC-2024-0421: idna Punycode Vulnerability
- **Severity:** CRITICAL
- **Affected Crate:** `idna v0.4.0`
- **Dependency Path:** `trust-dns-proto v0.23.2` → `trust-dns-resolver v0.23.2` → `terraphim_rlm v1.14.0`
- **Description:** Accepts Punycode labels that do not produce non-ASCII output, enabling domain spoofing attacks
- **Impact:** Potential privilege escalation when hostname comparison is part of privilege checks
- **Attack Vector:** Attacker could introduce DNS entry with `xn--`-masked name that resolves to target domain
- **CVSS Impact:** Hostname spoofing, privilege escalation

**Remediation:**
```bash
# Upgrade idna to 1.0.3+ or url to 2.5.4+
cargo update -p idna
cargo update -p url
```

**Note:** If using Rust < 1.81 with SQLx 0.8.2 or earlier, review compatibility issues before upgrading.

### Ignored Advisories (Acceptable Risk)

The following advisories are intentionally ignored in deny.toml:

| Advisory | Reason | Risk Level |
|----------|--------|------------|
| RUSTSEC-2023-0071 | RSA Marvin Attack - no safe upgrade via octocrab | Transitive, limited impact |
| RUSTSEC-2021-0145 | atty unaligned read - Windows only | Platform-specific |
| RUSTSEC-2024-0375 | atty unmaintained - should migrate | Maintenance |
| RUSTSEC-2025-0141 | bincode unmaintained - evaluate alternatives | Maintenance |
| RUSTSEC-2021-0141 | dotenv unmaintained - replace with dotenvy | Maintenance |
| RUSTSEC-2026-0049 | rustls-webpki CRL bypass - needs rustls 0.23.x | Requires upstream updates |
| RUSTSEC-2020-0163 | term_size unmaintained - replace with terminal_size | Maintenance |

### Recommendations

1. **Immediate:** Upgrade `idna` to v1.0.3+ or `url` to v2.5.4+
2. **Short-term:** Review ignored advisories quarterly
3. **Ongoing:** Integrate `cargo deny check advisories` into CI/CD

---

## 3. GDPR/Data Handling Patterns Audit

### Status: ⚠️ REVIEW NEEDED

### 3.1 Data Processing Locations

Based on code analysis of 6866+ logging and data handling statements across the codebase:

| Category | Finding | Risk Level |
|----------|---------|------------|
| User Data Storage | No centralized PII detection | Medium |
| Logging Practices | 6866+ log statements - review needed | Low-Medium |
| Data Retention | Backup retention policies exist in `terraphim_update` | Low |
| Data Deletion | Delete functions present in update/state modules | Low |
| Sensitive Patterns | Content filtering in multi-agent hooks | Low |

### 3.2 Key Findings

#### ✅ Positive Findings

1. **Content Filtering in Multi-Agent System**
   - Location: `crates/terraphim_multi_agent/src/vm_execution/hooks.rs:373-388`
   - Implementation: Sensitive pattern detection blocks potentially sensitive output
   - Patterns checked: API keys, passwords, tokens, private keys, secrets

2. **Privacy-Aware Logging**
   - Location: `crates/terraphim_router/src/engine.rs:12`
   - Implementation: Truncates prompts to 50 chars for safe logging
   - Pattern: `/// Truncate prompt to first 50 chars for safe logging (privacy).`

3. **Security-Conscious Error Handling**
   - Location: `crates/terraphim_validation/src/testing/server_api/security.rs:227-238`
   - Implementation: Ensures no sensitive information leaked in error responses

4. **Data Retention Management**
   - Location: `crates/terraphim_update/src/state.rs`
   - Implementation: Delete functions for update history and backups
   - Retention policies tested: `test_multiple_backup_retention`, `test_backup_cleanup_retention_limit`

#### ⚠️ Areas for Review

1. **Email Handling via JMAP Protocol**
   - Location: `crates/terraphim_middleware/src/haystack/jmap.rs`, `crates/terraphim_middleware/src/indexer/mod.rs:135`
   - Activity: Email search and indexing via JMAP protocol (RFC 8620/8621)
   - Review Needed: Email content retention policies and consent mechanisms

2. **Session Data Storage**
   - Location: `crates/terraphim_sessions/src/service.rs`
   - Activity: Session search with case-insensitive matching
   - Review Needed: Session data retention periods and deletion procedures

3. **Knowledge Graph Data**
   - Location: `crates/terraphim_rolegraph/src/lib.rs`
   - Activity: Personal knowledge graph processing
   - Review Needed: User consent for knowledge graph construction and retention

4. **GitHub/GitLab Integration**
   - Location: `crates/terraphim_github_runner_server/src/github/mod.rs:29`
   - Activity: Personal token usage for GitHub API access
   - Review Needed: Token storage security and rotation policies

### 3.3 Metadata Handling

- **Pattern Found:** Extensive use of `HashMap<String, String>` for metadata in:
  - `crates/terraphim_multi_agent/src/llm_types.rs`
  - `crates/terraphim_multi_agent/src/agents/chat_agent.rs`
- **Review Needed:** Ensure metadata doesn't inadvertently store PII

### 3.4 GDPR Compliance Recommendations

1. **Data Inventory**
   - Create comprehensive data flow diagram
   - Document all PII processing locations
   - Identify data controllers and processors

2. **Consent Management**
   - Implement explicit consent for knowledge graph construction
   - Add consent tracking for email indexing
   - Provide opt-out mechanisms

3. **Right to Deletion**
   - Verify all user data deletion paths are complete
   - Test deletion propagation across modules
   - Document retention periods

4. **Data Minimization**
   - Review if all collected data is necessary
   - Implement automatic data purging
   - Anonymize data where possible

5. **Logging Review**
   - Audit 6866+ logging statements for PII leakage
   - Implement structured logging with PII redaction
   - Create log retention policy

---

## 4. Compliance Action Items

### Immediate (Within 1 Week)

- [ ] **CRITICAL:** Upgrade `idna` to v1.0.3+ or `url` to v2.5.4+ (RUSTSEC-2024-0421)
- [ ] **HIGH:** Add license declaration to fcctl-core or replace dependency
- [ ] **HIGH:** Review deny.toml configuration for outdated advisory ignores

### Short-term (Within 1 Month)

- [ ] **MEDIUM:** Replace html2md with MIT/Apache-2.0 licensed alternative
- [ ] **MEDIUM:** Document data retention policies for all user data
- [ ] **MEDIUM:** Implement consent management for knowledge graph features
- [ ] **MEDIUM:** Audit logging statements for PII leakage

### Long-term (Within 3 Months)

- [ ] **LOW:** Complete GDPR compliance documentation
- [ ] **LOW:** Implement automated PII detection in CI/CD
- [ ] **LOW:** Add privacy impact assessment documentation
- [ ] **LOW:** Create data processing agreement templates

---

## 5. Compliance Tools & Automation

### Recommended CI/CD Integration

```yaml
# Add to GitHub Actions workflow
- name: License Compliance Check
  run: cargo deny check licenses

- name: Security Advisory Check
  run: cargo deny check advisories

- name: GDPR Pattern Scan
  run: |
    # Check for potential PII patterns in logs
    grep -r "email\|phone\|ssn\|password" --include="*.rs" . || true
```

### Monitoring

- Run `cargo deny` checks weekly
- Review security advisories monthly
- Audit data handling patterns quarterly
- Update compliance documentation bi-annually

---

## 6. Appendix

### 6.1 Workspace Structure

The project consists of the following workspace members:
- `terraphim_server` - Main server crate
- `crates/*` - 30+ supporting crates
- `terraphim_firecracker` - Firecracker integration
- `terraphim_ai_nodejs` - Node.js bindings

### 6.2 Dependency Statistics

- Total crates analyzed: 200+
- Direct dependencies: ~30
- Dev dependencies: ~15
- Transitive dependencies: 150+

### 6.3 License Distribution

| License | Count | Status |
|---------|-------|--------|
| MIT | ~60% | ✅ Allowed |
| Apache-2.0 | ~25% | ✅ Allowed |
| GPL-3.0+ | 1 | ⚠️ Deprecated identifier |
| Unlicensed | 1 | ❌ ERROR |

---

## Report Generation

**Generated:** 2026-03-27
**Tools Used:**
- cargo-deny v0.19.0+
- cargo-tree
- grep/rg for pattern analysis

**Next Review Date:** 2026-04-27

---

*This report is automatically generated and should be reviewed by the compliance team.*
