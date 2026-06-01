# Compliance Watchdog Report — 2026-06-01 09:10 CEST

**Verdict: CONDITIONAL PASS**
**Run by:** Vigil (Security Engineer)
**Branch:** task/doc-automata-gaps-2026-06-01

---

## Summary

All `cargo deny` checks pass (exit 0). No new P1 or P2 issues found.
Four P2 carry-forwards remain unresolved.

---

## 1. Licence Compliance

**Result: PASS** (`cargo deny check licenses` exit 0)

Warnings only (not blocking):
- `html2md v0.2.15` uses deprecated SPDX identifier `GPL-3.0+` (should be `GPL-3.0-only` or `GPL-3.0-or-later`). Dependency of `terraphim_middleware`.
- Two unused allowlist entries: `OpenSSL`, `Unicode-DFS-2016`.

**Action required:** None blocking. Consider filing a low-priority cleanup issue to either remove unused allowlist entries or update html2md.

---

## 2. Dependency Supply Chain — Advisories

**Result: PASS** (`cargo deny check advisories` exit 0)

**Carry-forward warning:**
- `aes v0.9.0` — yanked crate, pulled in via `zip v8.6.0` → `terraphim_update`. P2 carry-forward (no RUSTSEC advisory assigned, but yanked = unmaintained/buggy upstream).
  - Fix: `cargo update -p aes`

All stale RUSTSEC ignores (RUSTSEC-2021-0141, -0145, -2023-0071, -2024-0375, -2026-0049, -0097, -0098, -0099) have been resolved upstream — no longer matched. These can be removed from `deny.toml`.

---

## 3. Dependency Supply Chain — Bans

**Result: PASS** (`cargo deny check bans` exit 0)

Duplicate crate warnings (non-blocking):
- `axum 0.7.9` / `axum 0.8.9` — both in dependency tree. `terraphim_validation` pins 0.7.9; rest of workspace uses 0.8.9.

---

## 4. Dependency Supply Chain — Sources

**Result: PASS** (`cargo deny check sources` exit 0)

Unmatched allowlist entry: `https://github.com/snapview/tokio-tungstenite.git` — no longer in use; can be removed from `deny.toml`.

---

## 5. GDPR / Data Handling Audit

### 5.1 Credential Exposure via Debug Derives

**New issues found this cycle: 0**

**P2 carry-forwards (all still open):**

| Issue | Struct | File | Field | Severity |
|-------|--------|------|-------|----------|
| #1833 | `JMAPClient` | `haystack_jmap/src/lib.rs:128` | `access_token: String` | P2 |
| #1834 | `EmailAddress` | `haystack_jmap/src/lib.rs:95` | `email: String` (PII) | P2 |
| #1938 | `QuickwitConfig` (private) | `terraphim_middleware/src/haystack/quickwit.rs:31` | `auth_token`, `auth_username`, `auth_password` | P2 |
| #1939 | `PerplexityHaystackIndexer` | `terraphim_middleware/src/haystack/perplexity.rs:97` | `api_key: String` | P2 |

**Closed since last cycle:**
- #1930 — `RlmConfig` Debug exposing `alert_webhook_url` + `e2b_api_key` — CLOSED

### 5.2 Credential Exposure — Verified COMPLIANT

The following credential-bearing structs have been audited and use properly redacting custom `Debug` implementations:

- `GiteaConfig` — `token` → `***REDACTED***`
- `GiteaOutputConfig` — `token` → `***REDACTED***`
- `GiteaSkillRepoConfig` — `token` → masked when Some
- `GiteaWikiConfig` — `token` → `***REDACTED***`
- `LinearConfig` — `api_key` → `***REDACTED***`
- `MatrixConfig` — `password` → `***REDACTED***`
- `ProxyConfig` — `password` → masked when Some
- `TrackerConfig` — `api_key` → manually redacted in Debug impl
- `WebhookConfig` — `secret` → masked when Some
- `ProjectDispatchState` — `token` field omitted from Debug output
- `ZaiProvider` — no Debug derive
- `MiniMaxProvider` — no Debug derive
- `ConfluenceClient` / `JiraClient` — no Debug derive

### 5.3 Logging Patterns

No `println!` or `tracing::` calls logging raw credential values found.

---

## 6. Overall Verdict

| Domain | Status |
|--------|--------|
| Licences | PASS |
| Advisories | PASS (yanked `aes` carry-fwd) |
| Bans | PASS |
| Sources | PASS |
| GDPR — new issues | PASS (0 new) |
| GDPR — carry-forwards | CONDITIONAL (4× P2 open) |

**CONDITIONAL PASS** — no regressions; four P2 carry-forwards #1833/#1834/#1938/#1939 pending resolution.

---

## 7. Recommended Actions

1. **Resolve P2 carry-forwards** (#1833, #1834, #1938, #1939) — implement custom Debug or `#[debug_stub]` for affected structs.
2. **Update yanked `aes`** — run `cargo update -p aes` and verify `zip` dependency still compiles.
3. **Clean up stale `deny.toml` entries** — remove resolved RUSTSEC ignores and unmatched source/licence allowances.
