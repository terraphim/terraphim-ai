# Compliance Report — terraphim-ai
**Date**: 2026-05-18 10:11 CEST  
**Auditor**: Vigil (security-sentinel)  
**Verdict**: CONDITIONAL PASS

---

## 1. Licence Compliance (`cargo deny check licenses`)

**Result: PASS** (exit 0)

| Finding | Severity | Detail |
|---------|----------|--------|
| `html2md-0.2.15` uses deprecated SPDX id `GPL-3.0+` | Info | Should be `GPL-3.0-or-later`; functionally identical, cargo-deny accepts it as warning |
| `OpenSSL` allowance unused | Info | Stale entry in deny.toml allow list |
| `Unicode-DFS-2016` allowance unused | Info | Stale entry in deny.toml allow list |

No licence violations. All dependency licences are within the approved set.

---

## 2. Advisory / Supply-Chain (`cargo deny check advisories`)

**Result: PASS** (exit 0)

All 8 advisory ignore entries in `deny.toml` produced `advisory-not-detected` warnings, meaning those vulnerable crates are **no longer present** in the dependency tree. This is a positive signal: prior remediations have been effective.

| Previously ignored advisory | Status |
|-----------------------------|--------|
| RUSTSEC-2023-0071 (RSA Marvin Attack) | Crate removed — ignore entry stale |
| RUSTSEC-2021-0145 (atty unaligned read) | Crate removed |
| RUSTSEC-2024-0375 (atty unmaintained) | Crate removed |
| RUSTSEC-2025-0141 (bincode unmaintained) | Crate removed |
| RUSTSEC-2021-0141 (dotenv unmaintained) | Crate removed |
| RUSTSEC-2020-0163 (term_size unmaintained) | Crate removed |
| RUSTSEC-2026-0049/0098/0099 (rustls-webpki) | Crate removed from default build |
| RUSTSEC-2026-0097 (rand unsound) | Crate removed |

**Recommendation (low priority)**: Remove the 8 stale `ignore` entries from `deny.toml` to keep the file clean and surface future advisories promptly.

---

## 3. Dependency Bans (`cargo deny check bans`)

**Result: PASS** (exit 0)

Multiple-version warnings expected in a workspace of this size; configuration correctly uses `warn` not `deny` for this check.

---

## 4. Source / Supply Chain (`cargo deny check sources`)

**Result: PASS with warnings** (exit 0)

Two git sources produce `source-not-allowed` warnings (warnings only, not errors):

| Package | Git source | Risk |
|---------|-----------|------|
| `fcctl-core v0.1.0` | `github.com/terraphim/firecracker-rust` | First-party terraphim fork — low risk |
| `fff-grep/fff-query-parser/fff-search v0.5.1` | `github.com/AlexMikhalev/fff.nvim.git` | First-party fork — low risk |

**Remediation**: Add both URLs to the `allow-git` list in `deny.toml`:
```toml
"https://github.com/terraphim/firecracker-rust",
"https://github.com/AlexMikhalev/fff.nvim.git",
```

---

## 5. GDPR / Data Handling Audit

### 5.1 Architecture Assessment

Terraphim AI is a privacy-first, local-first system. User documents are processed within the user's environment; no data is sent to third parties except via explicitly configured integrations (Atlassian, Linear, Discourse, JMAP). This architectural choice minimises GDPR Article 6 lawfulness concerns.

### 5.2 Credential Redaction in Debug Output

Manual `fmt::Debug` implementations confirmed **present and correct** for:

| Struct | Location | Credential field | Status |
|--------|----------|-----------------|--------|
| `GiteaOutputConfig` | orchestrator/src/config.rs:516 | `token` | REDACTED |
| `WebhookConfig` | orchestrator/src/config.rs:588 | `secret` | REDACTED |
| `TrackerConfig` | orchestrator/src/config.rs:975 | `api_key` | REDACTED |
| `GiteaOutputConfig` (token) | orchestrator/src/config.rs:530 | `token` | REDACTED |
| `LinearConfig` | terraphim_tracker/src/linear.rs:13 | `api_key` | REDACTED |
| `GiteaWikiConfig` | terraphim_agent/src/shared_learning/wiki_sync.rs:28 | `token` | REDACTED |
| `LlmConfig` | terraphim_agent/src/onboarding/prompts.rs:469 | `api_key` | REDACTED |
| `ProxyConfig` | terraphim_agent/src/repl/web_operations.rs:74 | `password` | REDACTED in Debug |

### 5.3 NEW FINDING — P2 Gap: `RlmConfig` unredacted Debug

**Severity: P2 (Medium)**  
**Location**: `crates/terraphim_rlm/src/config.rs`, line 9

`RlmConfig` uses `#[derive(Debug)]` and contains two credential-bearing fields:

- `alert_webhook_url: Option<String>` (line 108) — webhook URLs often embed auth tokens in path/query
- `e2b_api_key: Option<String>` (line 126) — E2B sandbox API key

These fields will appear in plaintext in any `{:?}` formatting (panic messages, tracing logs, assertion failures).

This gap was **not captured by issue #1667** (which tracked 5 other structs). Requires a new issue.

**Remediation**: Replace `#[derive(Debug)]` on `RlmConfig` with a manual `impl std::fmt::Debug` that redacts `alert_webhook_url` and `e2b_api_key`. Pattern to follow: `GiteaOutputConfig` at orchestrator/src/config.rs:528.

### 5.4 `ProxyConfig.password` Serialization Note

`ProxyConfig` has `#[derive(Serialize, Deserialize)]` — the `password` field will appear in JSON serialisation even though Debug is redacted. The source file includes a comment acknowledging this. No custom `Serialize` implementation exists.

**Severity: P3 (advisory)** — covered in #1667 AC2. Acceptable as documented risk; callers must not log serialised form.

### 5.5 PII Scope

No evidence of personal data (email addresses, phone numbers, IP addresses) stored or logged in structured fields. The JMAP haystack (`haystack_jmap`) processes email content but the crate is an integration layer — data handling is delegated to the user's JMAP server. No GDPR consent or deletion APIs are required for the local-first model.

---

## 6. Summary

| Check | Result | Notes |
|-------|--------|-------|
| `cargo deny licenses` | PASS | 3 info-level warnings |
| `cargo deny advisories` | PASS | 8 stale ignores; 0 active advisories |
| `cargo deny bans` | PASS | Multiple-version warnings expected |
| `cargo deny sources` | PASS | 2 first-party git sources need allow-listing |
| Credential Debug redaction | CONDITIONAL PASS | 8 structs correct; **RlmConfig** gap unaddressed |
| GDPR/PII handling | PASS | Local-first model; no structured PII storage found |

---

## 7. Required Actions

| # | Action | Priority | Owner |
|---|--------|----------|-------|
| 1 | Fix `RlmConfig` Debug redaction (`alert_webhook_url`, `e2b_api_key`) | P2 | Security |
| 2 | Add `firecracker-rust` and `fff.nvim.git` to `deny.toml allow-git` | P3 | Maintainer |
| 3 | Remove 8 stale advisory ignore entries from `deny.toml` | P3 | Maintainer |
| 4 | Verify `ProxyClientConfig` in tinyclaw crate if it still exists | P2 | #1667 assignee |
