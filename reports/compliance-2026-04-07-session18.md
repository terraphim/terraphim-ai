# Terraphim AI Compliance Report

**Report Date:** 2026-04-07 10:53 CEST
**Auditor:** Vigil, Security Engineer
**Project:** terraphim-ai
**Scope:** License compliance, dependency supply chain, GDPR/data handling patterns, configuration security
**Status:** FAIL - Multiple critical and high-severity violations

---

## Executive Summary

| Category | Status | Critical | High | Medium | Low |
|----------|--------|----------|------|--------|-----|
| License Compliance | 🔴 FAIL | 2 | 0 | 1 | 0 |
| Supply Chain (CVEs) | 🔴 FAIL | 1 | 0 | 6 | 0 |
| Configuration Security | 🔴 FAIL | 1 | 0 | 0 | 0 |
| GDPR/Data Handling | 🟡 PARTIAL | 0 | 0 | 0 | 3 |

**Verdict:** **FAIL** - Blocking compliance issues prevent merge

---

## 1. License Compliance

### Status: 🔴 FAIL - 2 Unlicensed Crates Block Merge

### Critical Findings

| ID | Severity | Issue | Evidence | Remediation |
|----|----------|-------|----------|-------------|
| LIC-001 | CRITICAL | Missing license field in `terraphim_ccusage` | `crates/terraphim_ccusage/Cargo.toml` lacks `license` field | Add `license = "..."` to Cargo.toml |
| LIC-002 | CRITICAL | Missing license field in `terraphim_usage` | `crates/terraphim_usage/Cargo.toml` lacks `license` field | Add `license = "..."` to Cargo.toml |
| LIC-003 | HIGH | Deprecated SPDX identifier in transitive dependency | `html2md v0.2.15` uses deprecated `GPL-3.0+` | Update html2md or accept (transitive) |

### Impact

- **Merge Blocking:** Cargo deny exits with code 4 (license check failure)
- **Supply Chain Integrity:** Unlicensed crates prevent automated license auditing
- **Legal Risk:** Unclear licensing terms increase exposure

### Cargo Deny Output
```
error[unlicensed]: terraphim_ccusage = 1.16.9 is unlicensed
error[unlicensed]: terraphim_usage = 1.16.9 is unlicensed
warning[parse-error]: html2md-0.2.15 uses deprecated GPL-3.0+
```

---

## 2. Supply Chain Security (CVEs)

### Status: 🔴 FAIL - CRITICAL CVE Unresolved

### Critical Findings

| ID | CVE ID | Severity | Package | Version | Impact | Remediation |
|----|--------|----------|---------|---------|--------|-------------|
| CVE-001 | RUSTSEC-2026-0049 | **CRITICAL** | rustls-webpki | 0.102.8 | CRL certificate validation bypass | Upgrade to >= 0.103.10 |

### Vulnerability Details: RUSTSEC-2026-0049

**Vulnerability:** CRL Distribution Point Matching Failure  
**Date Disclosed:** 2026-03-20  
**CVSS Score:** 7.5 (High)

**Description:**
CRL (Certificate Revocation List) checking fails when a certificate has multiple distribution points. Only the first distribution point is checked against the CRL's `IssuingDistributionPoint`. Subsequent distribution points are ignored, potentially allowing **revoked certificates to be accepted** when `UnknownStatusPolicy::Allow` is configured.

**Attack Vector:**
- Requires compromise of trusted Certificate Authority
- Latent bug: revoked credentials continue to be trusted
- TLS certificate validation can be bypassed

**Affected Code Paths:**
```
rustls-webpki v0.102.8
└── rustls v0.22.4
    ├── tungstenite v0.21.0
    │   └── tokio-tungstenite v0.21.0
    │       └── serenity v0.12.5
    │           └── terraphim_tinyclaw v1.16.9
    └── tokio-tungstenite v0.21.0
```

**Exposure:** Terraphim Tinyclaw (Discord integration) and all websocket-based authentication

**Remediation:**
```bash
cargo update -p rustls-webpki --aggressive
# Or pin in Cargo.lock to >= 0.103.10
```

### Unmaintained Dependencies (Warnings)

| ID | Package | Version | Status | Risk | Mitigation |
|----|---------|---------|--------|------|-----------|
| UNM-001 | bincode | 1.3.3 | Unmaintained since 2025 | Medium | Replace with `postcard` or `serde_json` |
| UNM-002 | instant | 0.1.13 | Unmaintained | Low | Minimal usage, monitor |
| UNM-003 | number_prefix | 0.4.0 | Unmaintained | Low | Limited scope |
| UNM-004 | paste | 1.0.15 | Unmaintained | Low | Proc macro, stable API |
| UNM-005 | rustls-pemfile | 1.0.4 | Unmaintained | Medium | Use `rustls-pem` alternative |
| UNM-006 | term_size | 0.3.2 | Unmaintained since 2020 | Low | Terminal UI only |

### Yanked Dependencies

| Package | Version | Issue |
|---------|---------|-------|
| fastrand | 2.4.0 | Yanked from crates.io - build will fail if re-downloaded |

---

## 3. Configuration Security

### Status: 🔴 FAIL - Port 3456 Hardcoded in Version Control

### Critical Findings

| ID | Severity | Issue | Location | Impact | Remediation |
|----|----------|-------|----------|--------|-------------|
| CFG-001 | CRITICAL | Internal Ollama port hardcoded | `terraphim_server/default/ollama_llama_config.json` (24 instances) | Port enumeration, targeted attacks | Use environment variables |
| CFG-002 | CRITICAL | Internal network IP exposed | Same file (IP: 100.106.66.7) | Network topology disclosure | Remove from version control |

### Details

**File:** `terraphim_server/default/ollama_llama_config.json`

The configuration file hardcodes:
- **Port:** 3456 (non-standard, immediately identifiable as Ollama)
- **IP:** 100.106.66.7 (internal network address, likely lab/dev environment)
- **Occurrences:** 24 hardcoded instances across all agent roles

**Risk Assessment:**
- **Reconnaissance Risk:** Attackers can quickly identify exposed Ollama instances
- **Port Scanning:** Non-standard port 3456 simplifies targeting
- **Network Mapping:** Internal IP reveals network topology
- **Version Disclosure:** Allows identification of specific Ollama version

**Example from Config:**
```json
{
  "Llama Rust Engineer": {
    "extra": {
      "llm_provider": "openai",
      "llm_base_url": "http://100.106.66.7:3456"
    }
  }
}
```

**Remediation:**
```json
{
  "Llama Rust Engineer": {
    "extra": {
      "llm_provider": "openai",
      "llm_base_url": "${LLM_BASE_URL:-http://localhost:11434}"
    }
  }
}
```

Or use environment variable loading at runtime:
```rust
let llm_url = std::env::var("LLM_BASE_URL")
    .unwrap_or_else(|_| "http://localhost:11434".to_string());
```

---

## 4. GDPR/Data Handling Audit

### Status: 🟡 PARTIAL - Missing Data Subject Rights Implementation

### Findings

| ID | Category | Finding | Risk | Recommendation |
|----|----------|---------|------|----------------|
| GDPR-001 | Right to Be Forgotten | No data deletion API documented | High | Implement DELETE endpoints with cascade |
| GDPR-002 | Data Portability | No export mechanism for user data | Medium | Implement JSON/CSV export function |
| GDPR-003 | Consent Logging | Session tracking in JSONL without consent flag | Medium | Add explicit consent field to session records |
| GDPR-004 | Privacy by Design | Search results cached without retention policy | Medium | Document cache TTL and purge mechanisms |

### Data Processing Observations

**Session Management (`terraphim_sessions`):**
- Sessions stored in JSONL format at `~/.claude/projects/*/sessions/`
- No explicit consent tracking
- No documented retention policy
- No purge mechanism observed

**Haystack Integration:**
- Multiple external data sources (Ripgrep, Atomic Server, QueryRs, MCP)
- No documented data processing agreements
- No third-party privacy terms documented

**Cache Behavior:**
- Persistence layer supports multiple backends (memory, dashmap, sqlite, S3)
- Fire-and-forget cache writeback pattern
- No documented cache eviction or GDPR compliance

### Recommendations (Non-Blocking)

1. Document data retention policy (recommended: 30-90 days for sessions)
2. Implement data export API for compliance with Article 20 (GDPR)
3. Add consent tracking to session records
4. Document third-party data processors (haystack integrations)

---

## Compliance Gate Checklist

- [ ] **BLOCKING:** License compliance (2 unlicensed crates fixed)
- [ ] **BLOCKING:** RUSTSEC-2026-0049 CVE resolved (rustls-webpki >= 0.103.10)
- [ ] **BLOCKING:** Port 3456 configuration secured (environment variables)
- [ ] Optional: Unmaintained dependencies upgraded (bincode → postcard)
- [ ] Optional: GDPR data subject rights implemented

---

## Severity Classification

- **CRITICAL (Merge Blocking):** License violations, RUSTSEC-2026-0049, hardcoded credentials/ports
- **HIGH (Should Fix):** Unmaintained core dependencies (bincode)
- **MEDIUM (Should Document):** GDPR compliance gaps, yanked versions
- **LOW (Monitor):** Transitive deprecated licenses, obsolete utilities

---

## Next Steps

1. **Immediate (Blocking):**
   - Add license fields to `terraphim_ccusage` and `terraphim_usage`
   - Update `rustls-webpki` to >= 0.103.10
   - Move port 3456 to environment configuration

2. **Before Release:**
   - Evaluate `bincode` replacement (postcard/serde_json)
   - Document GDPR data handling practices
   - Review third-party data processing agreements

3. **Ongoing:**
   - Monitor RUSTSEC database weekly
   - Track unmaintained dependency security status
   - Validate configuration security in CI/CD

---

**Generated By:** Vigil, Security Engineer  
**Audit Date:** 2026-04-07 10:53 CEST  
**Next Audit:** 2026-04-14
