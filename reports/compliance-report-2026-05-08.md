compliance-watchdog verdict: FAIL

# Terraphim AI Compliance Report — 2026-05-08

**Run date**: 2026-05-08 12:19 CEST
**Auditor**: Vigil (Security Engineer — SFIA SCTY-5/VUAS-4)
**Scope**: terraphim-ai workspace — licence compliance, dependency supply chain, GDPR/data handling

---

## 1. Licence Compliance (`cargo deny check licenses`)

**Result: PASS**

All dependency licences are within the approved allow-list (MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, MPL-2.0, CC0-1.0, Unicode-3.0, GPL-3.0-or-later, AGPL-3.0-or-later, and others).

Two minor warnings — `OpenSSL` and `Unicode-DFS-2016` listed in allow-list but not encountered in current dependency graph. These are harmless (allowances can be pruned in a later housekeeping pass).

---

## 2. Dependency Supply Chain (`cargo deny check advisories`)

**Result: PASS**

No active advisories detected against the current dependency graph. All 8 previously-ignored RUSTSEC advisories are either:
- No longer present in the dependency graph (advisory-not-detected), or
- Suppressed with documented rationale in `deny.toml`

Outstanding suppressed advisories with remediation debt (track separately):

| Advisory | Reason suppressed | TODO |
|----------|-------------------|------|
| RUSTSEC-2023-0071 | RSA Marvin Attack via octocrab -> jsonwebtoken | Remove once octocrab upgrades rsa to constant-time impl |
| RUSTSEC-2024-0375 | `atty` unmaintained | Replace with `std::io::IsTerminal` |
| RUSTSEC-2025-0141 | `bincode` unmaintained | Evaluate postcard/rkyv for redb backend |
| RUSTSEC-2021-0141 | `dotenv` unmaintained | Replace with `dotenvy` |
| RUSTSEC-2020-0163 | `term_size` unmaintained | Replace with `terminal_size` |

All active advisory suppression is documented with context. Supply chain posture is acceptable.

---

## 3. GDPR / Data Handling Audit

**Result: FAIL — 4 critical findings, 3 high findings**

### CRITICAL-1: Unfiltered debug logging of API response bodies

**Severity**: Critical | **CWE**: CWE-532 (Insertion of Sensitive Information into Log File)

**Locations**:
- `crates/haystack_atlassian/src/confluence.rs` — `eprintln!("DEBUG: Response body: {}", response_text)`
- `crates/haystack_atlassian/src/jira.rs` — `eprintln!("DEBUG: Response body: {}", response_text)` and `eprintln!("DEBUG: Request body: {:?}", search_request)`
- `crates/haystack_discourse/src/client.rs` — `println!("Response body: {}", response_text)`

**Impact**: Full JSON payloads including user email addresses, assignee names, issue descriptions, and forum post content are emitted to stderr/stdout unconditionally. In containerised or CI environments these streams are captured in logs visible to operators.

**Remediation**: Gate all debug prints behind `tracing::debug!` or `log::debug!` with `cfg(debug_assertions)` guard. Apply the existing redaction module (`terraphim_agent/src/learnings/redaction.rs`) to any structured log that may carry PII.

---

### CRITICAL-2: No encryption at rest for personal data

**Severity**: Critical | **CWE**: CWE-311 (Missing Encryption of Sensitive Data)

**Location**: `crates/terraphim_persistence/` — all backends (memory, DashMap, SQLite, ReDB, S3)

**Impact**: Conversations, documents (including full email body, forum posts, issue descriptions), and configuration are stored as plaintext. A compromised host or backup leaks all indexed personal data.

**Remediation**: Implement field-level or database-level encryption for SQLite/ReDB backends. S3 backend should enforce SSE-KMS. Compression (zstd) already present — encryption should wrap after compression.

---

### CRITICAL-3: Redaction module not applied consistently

**Severity**: Critical | **CWE**: CWE-532

**Location**: `crates/terraphim_agent/src/learnings/redaction.rs` exists but is scoped to the learning store only.

**Impact**: Patterns for AWS keys, OpenAI tokens, Slack/GitHub tokens, and connection strings are detected in the learning store but no equivalent filtering is applied in haystack API call logs, persistence write paths, or the tracing spans.

**Remediation**: Extract redaction logic to a shared crate or module; apply at persistence write boundary and at all `tracing::` instrumentation call sites that accept user-supplied or externally-fetched content.

---

### CRITICAL-4: Full personal communication content indexed without minimisation

**Severity**: Critical | **CWE**: CWE-359 (Exposure of Private Personal Information)

**Locations**: `crates/haystack_jmap/src/lib.rs`, `crates/haystack_atlassian/`, `crates/haystack_discourse/`

**Impact**: Full email bodies (sender/recipient addresses, message content), full Jira issue descriptions with reporter/assignee emails, full Discourse post bodies are fetched and indexed. No truncation, PII filtering, or user-level consent gate is applied before data enters the search index.

**Remediation**: Implement data minimisation step in indexing pipeline — strip raw email addresses from stored bodies, truncate document bodies to a configurable limit, add a consent configuration field per haystack.

---

### HIGH-1: No right-to-erasure (GDPR Article 17) implementation

**Severity**: High

**Location**: `crates/terraphim_persistence/src/conversation.rs` — `delete()` exists per conversation ID but no user-level bulk deletion or anonymisation endpoint exists.

**Impact**: Inability to satisfy a data subject erasure request without direct database access.

**Remediation**: Implement a `forget_user` or `delete_by_subject` API that removes all documents, conversations, and index entries associated with a given identity. Expose via the admin API endpoint.

---

### HIGH-2: No data retention policy or automatic expiry

**Severity**: High

**Location**: `crates/terraphim_persistence/` — conversations and documents stored indefinitely.

**Impact**: Personal data accumulates without bound; retention exceeds what is proportionate under GDPR Article 5(1)(e).

**Remediation**: Add a configurable `retention_days` field to settings; implement a background task that purges records older than the retention window.

---

### HIGH-3: Base64 credential encoding misrepresented as security

**Severity**: High | **CWE**: CWE-261 (Weak Encoding for Password)

**Location**: `crates/haystack_atlassian/src/confluence.rs` lines 102-106

**Impact**: HTTP Basic Auth credentials are base64-encoded (not encrypted) before being passed in Authorization headers. If the credential value appears in a log or memory dump it is trivially decoded.

**Remediation**: Base64 for HTTP Basic Auth is the correct protocol mechanism — the finding is that the raw token must never be logged. Audit all `tracing::` and `println!` call sites in the Atlassian integration to confirm the Authorization header value is never emitted.

---

## 4. Deny.toml Housekeeping Observations

- `OpenSSL` and `Unicode-DFS-2016` appear in the licence allow-list but are not encountered. Low priority — clean up in a maintenance pass.
- Eight advisory ignores with documented rationale present — debt is tracked. Acceptable posture for now.
- `[bans] multiple-versions = "warn"` — not enforced as error. Consider tightening for supply chain hygiene once version duplication is audited.

---

## Summary

| Check | Result |
|-------|--------|
| Licence compliance | PASS |
| Advisory supply chain | PASS |
| GDPR / data handling | **FAIL** |
| **Overall verdict** | **FAIL** |

### Prioritised remediation order

1. **CRITICAL-1** — Remove unconditional `eprintln!`/`println!` debug logging (low effort, high impact)
2. **CRITICAL-3** — Apply existing redaction module at persistence boundary
3. **CRITICAL-4** — Add data minimisation step to indexing pipeline
4. **HIGH-1** — Implement right-to-erasure endpoint
5. **HIGH-2** — Add configurable data retention with background expiry
6. **CRITICAL-2** — Implement encryption at rest (highest effort)
7. **HIGH-3** — Audit Atlassian credential logging

Licence and supply chain checks are GREEN. The GDPR/data handling findings block a clean PASS verdict. Critical-1 is a one-session fix. The remaining items warrant dedicated Gitea issues.

# cron run - no mention context
