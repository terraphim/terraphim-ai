# Compliance Audit Report: 2026-04-28 13:01 CEST

**Status**: PASS

**Agent**: Vigil (compliance-watchdog)
**Date**: 2026-04-28
**Audit Type**: Scheduled cron run

---

## Executive Summary

Compliance audit of terraphim-ai project completed. No critical or high severity findings detected. All dependency security advisories previously flagged have been resolved. License compliance verified across workspace crates.

## 1. License Compliance: PASS

**Command**: `cargo deny check licenses`
**Result**: `licenses ok`

### Findings

| Check | Status | Evidence |
|-------|--------|----------|
| Workspace crate licenses | PASS | All crates declare permitted licenses |
| Dependency tree licenses | PASS | All transitive dependencies use allowed licenses per deny.toml |
| html2md SPDX identifier | WARN | Uses deprecated "GPL-3.0+" identifier (external dep, not blocking) |
| Unused license allowances | INFO | OpenSSL, Unicode-DFS-2016 in deny.toml not currently matched |

**Assessment**: No license violations detected. The deprecated SPDX identifier in html2md is a warning-only issue in cargo-deny and does not block compliance.

## 2. Dependency Supply Chain: PASS

**Command**: `cargo deny check advisories`
**Result**: `advisories ok`

**Command**: `cargo audit`
**Result**: 5 allowed warnings, 0 critical vulnerabilities

### Advisory Status

| Advisory | Severity | Status | Evidence |
|----------|----------|--------|----------|
| RUSTSEC-2026-0049 (rustls-webpki) | CRITICAL | RESOLVED | No longer in dependency tree |
| RUSTSEC-2026-0098 (rustls-webpki) | CRITICAL | RESOLVED | No longer in dependency tree |
| RUSTSEC-2026-0099 (rustls-webpki) | CRITICAL | RESOLVED | No longer in dependency tree |
| RUSTSEC-2023-0071 (RSA) | HIGH | RESOLVED | No longer in dependency tree |
| RUSTSEC-2024-0375 (atty) | LOW | RESOLVED | No longer in dependency tree |
| RUSTSEC-2021-0141 (dotenv) | LOW | RESOLVED | No longer in dependency tree |
| RUSTSEC-2021-0145 (atty) | LOW | RESOLVED | No longer in dependency tree |
| RUSTSEC-2024-0384 (instant) | LOW | WARN | Unmaintained; allowed in deny.toml |
| RUSTSEC-2025-0119 (number_prefix) | LOW | WARN | Unmaintained; allowed in deny.toml |
| RUSTSEC-2024-0436 (paste) | LOW | WARN | Unmaintained; allowed in deny.toml |
| RUSTSEC-2025-0134 (rustls-pemfile) | LOW | WARN | Unmaintained; allowed in deny.toml |
| RUSTSEC-2020-0163 (term_size) | LOW | WARN | Unmaintained; allowed in deny.toml |

**Assessment**: All previously flagged critical and high severity advisories have been resolved through dependency updates. Five unmaintained crate warnings remain; all are documented in deny.toml with accepted risk rationale.

## 3. GDPR / Data Handling Patterns: PASS

### Data Collection Assessment

| Category | Finding | Status | Evidence |
|----------|---------|--------|----------|
| PII fields in structs | No explicit PII collection observed | PASS | Session models store author names only; no email/phone/address fields |
| API key storage | Stored as `Option<String>` in config | INFO | `llm_api_key` in Role, `atomic_server_secret` in Haystack |
| Secret persistence | Plain text in JSON/TOML config files | INFO | No at-rest encryption observed for config files |
| Secret serialization | Conditional for Atomic service only | PASS | `atomic_server_secret` only serialized when `service == Atomic` |
| Secret logging | Redaction implemented | PASS | `terraphim_agent/src/learnings/hook.rs:126` redacts secrets before stdout |
| Data retention | File-based local storage | INFO | `terraphim_persistence` handles storage lifecycle |
| Data deletion | Supported | PASS | `terraphim_persistence::delete()` and related APIs exist |
| Data subject rights | No dedicated GDPR framework | INFO | Delete operations available but no formal DSR automation |

### Code Locations Reviewed

- `crates/terraphim_config/src/lib.rs:215` - `llm_api_key: Option<String>`
- `crates/terraphim_config/src/lib.rs:338` - `atomic_server_secret: Option<String>`
- `crates/terraphim_config/src/lib.rs:346-379` - Custom Serialize impl for Haystack (conditional secret inclusion)
- `crates/terraphim_agent/src/learnings/hook.rs:124-130` - Secret redaction before stdout passthrough

### Risk Assessment

**Severity: LOW**

The project operates primarily as a local-first application with file-based storage. No explicit collection of personal data (email, phone, address, etc.) was identified. API keys are stored in configuration files as plain text, which is standard for local CLI tooling but represents a residual risk if config files are exposed. Secret redaction in logging demonstrates defensive coding practices.

**Recommendation**: Consider documenting secure storage guidance for users (e.g., file permissions, encrypted volumes). No code changes required for compliance.

## Compliance Status Summary

| Dimension | Status | Notes |
|-----------|--------|-------|
| License Compliance | PASS | All crates and dependencies permitted |
| CVE / Vulnerability | PASS | 0 critical, 0 high severity findings |
| Supply Chain | PASS | 5 unmaintained deps (documented/accepted) |
| Data Protection | PASS | No PII collection; secrets handled appropriately for local tool |
| Dependency Maintenance | WARN | 5 unmaintained (non-blocking) |

## Gate Verdict

**COMPLIANCE GATE: PASS**

No blocking issues identified. The project meets compliance thresholds for:
- License permissibility
- Security advisory hygiene (all critical/high resolved)
- Data protection baseline (local-first, minimal data collection)

**Warnings** (non-blocking, track for next sprint):
1. html2md deprecated SPDX identifier (external dependency)
2. 5 unmaintained dependencies (all with documented acceptance in deny.toml)
3. Config secrets stored plain text at rest (expected for local CLI; user guidance recommended)

**Action**: Safe to proceed. Schedule dependency refresh in next sprint to evaluate unmaintained crate replacements.

---
*Report generated by Vigil Security Audit - 2026-04-28 13:01 CEST*
