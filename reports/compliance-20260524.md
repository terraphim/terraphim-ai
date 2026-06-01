# Compliance Report — 2026-05-24 11:05 CEST

**Verdict: CONDITIONAL PASS**

Agent: Vigil (compliance-watchdog, cron run — no mention context)

---

## 1. Licence Compliance — PASS

`cargo deny check licenses` exits 0 (`licenses ok`).

**Warnings (non-blocking):**
- `html2md v0.2.15` uses deprecated SPDX identifier `GPL-3.0+` (resolves to `GPL-3.0-or-later`). Previously documented; deny.toml allowance covers it.
- Two unmatched allowances: `OpenSSL` and `Unicode-DFS-2016` — no crates currently matched. Can be pruned from deny.toml.

**Advisory note (P3):** deny.toml explicitly allows `GPL-3.0-or-later` and `AGPL-3.0-or-later`. Legal review required before commercial distribution as proprietary binary. Unchanged from prior audits.

---

## 2. Dependency Advisory Scan — PASS

`cargo deny check advisories` exits 0 (`advisories ok`).

**Active suppression still needed:**
- `RUSTSEC-2025-0141` (bincode unmaintained) — still in dependency graph via redb persistence backend. Ignore entry correctly in place with documented rationale.

**Stale suppression entries (8 advisories no longer encountered):**

These crates have been removed or upgraded. The suppress entries in deny.toml are now dead weight and should be pruned to reduce noise:

| Advisory | Description | Action |
|----------|-------------|--------|
| RUSTSEC-2021-0141 | dotenv unmaintained | Remove from deny.toml |
| RUSTSEC-2021-0145 | atty unaligned read | Remove from deny.toml |
| RUSTSEC-2023-0071 | RSA Marvin Attack (octocrab→jsonwebtoken→rsa) | Remove from deny.toml |
| RUSTSEC-2024-0375 | atty unmaintained | Remove from deny.toml |
| RUSTSEC-2026-0049 | rustls-webpki CRL revocation bypass | Remove from deny.toml |
| RUSTSEC-2026-0097 | rand unsound with custom logger | Remove from deny.toml |
| RUSTSEC-2026-0098 | rustls-webpki name constraints (URI) | Remove from deny.toml |
| RUSTSEC-2026-0099 | rustls-webpki name constraints (wildcards) | Remove from deny.toml |

No new unignored advisories detected.

---

## 3. Credential Leakage via `#[derive(Debug)]` — FAIL (P2, NEW)

### haystack_jmap/src/lib.rs — `JMAPClient`

```rust
// Line 128
#[derive(Debug)]
pub struct JMAPClient {
    session: Session,
    client: reqwest::Client,
    access_token: String,  // ← no custom Debug, exposed in {:?} output
}
```

`JMAPClient` derives `Debug` with `access_token: String`. Any `{:?}` formatting (error formatting, tracing spans, unwrap panics) will expose the JMAP bearer token in plain text.

**Severity:** P2
**Location:** `crates/haystack_jmap/src/lib.rs:128-138`
**Remediation:** Implement custom `fmt::Debug` that redacts `access_token`, matching the pattern used in `LinearConfig`, `GiteaConfig`, etc.

---

## 4. GDPR / PII Exposure via `#[derive(Debug)]` — FAIL (P2, NEW)

### haystack_jmap/src/lib.rs — `Email`, `EmailAddress`, `BodyValue`

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Email { /* subject, from, to, body_values, received_at */ }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmailAddress {
    pub name: Option<String>,   // ← PII: display name
    pub email: String,          // ← PII: email address
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BodyValue { pub value: String }  // ← PII: full email body
```

These structs contain GDPR personal data (email addresses, names, email body content) and derive the raw `Debug` trait. If any calling code applies `{:?}` or `{:#?}` formatting — in error messages, tracing spans, or test failures — the raw content will be logged.

**Severity:** P2
**Location:** `crates/haystack_jmap/src/lib.rs:64-121`
**Remediation:** Implement custom `fmt::Debug` for `Email` and `EmailAddress` that omits or truncates PII fields (email address, name, body). Alternatively add `#[debug(skip)]` from the `derive_more` crate if already available.

### email_to_document PII in index (informational P3)

`email_to_document` (lines 382-425) stores raw sender/recipient email addresses in:
- `description`: `"From: user@example.com To: user2@example.com"`
- `tags`: `["email", "sender:user@example.com", "2026-05-20"]`

This is intentional for search indexing but means email addresses are persisted in the haystack document store. If GDPR data-subject requests (right to erasure) are ever required, the document store must be queryable by email address. No data retention policy is currently enforced.

**Severity:** P3 (informational, architecture decision)
**Recommendation:** Document the retention policy for JMAP-sourced documents. Implement a deletion pathway by email address if erasure requests are in scope.

---

## 5. Existing Open GDPR Issues (not new, tracking)

| Issue | Status | Description |
|-------|--------|-------------|
| #1792 | Open | `eprintln!`/`println!` PII logging in haystack_atlassian and haystack_discourse |
| #1784 | Open | Debug derive credential leakage in tinyclaw/tracker/github-runner configs |

These remain unresolved. The new findings in section 3 and 4 are separate from #1784 (which covered tinyclaw/tracker/github-runner; JMAP was not included).

---

## 6. Unsafe Code Audit — PASS

17 files contain `unsafe {}` blocks. The single `deserialize_unchecked` call (sharded_extractor.rs:228) is properly guarded:
- SHA-256 checksum verification gate before unsafe call
- File permission enforcement (world/group write bits rejected)
- SAFETY comments documenting all preconditions
- Test at line 512 proves the safety invariant holds

No unsafe code violations identified.

---

## Summary

| Check | Result | Severity |
|-------|--------|----------|
| cargo deny licenses | PASS | — |
| cargo deny advisories | PASS | — |
| JMAPClient.access_token Debug derive | FAIL | P2 (new) |
| Email/EmailAddress PII Debug derive | FAIL | P2 (new) |
| email_to_document PII in index | INFO | P3 |
| Stale advisory ignores (8 entries) | ADVISORY | P3 |
| Unsafe code audit | PASS | — |
| Existing #1784 (tinyclaw/tracker Debug) | OPEN | P2 |
| Existing #1792 (eprintln! PII logging) | OPEN | P2 |

**Overall: CONDITIONAL PASS** — Two new P2 findings filed. cargo deny checks clean. Unsafe code properly justified. Existing P2 issues (#1784, #1792) remain open and must be resolved before next release gate.

---

## New Issues to File

1. `[compliance_watchdog] P2: JMAPClient.access_token exposed via raw Debug derive` — `crates/haystack_jmap/src/lib.rs:128`
2. `[compliance_watchdog] P2: GDPR — Email/EmailAddress/BodyValue PII exposed via raw Debug derive` — `crates/haystack_jmap/src/lib.rs:64-121`
