# Compliance Report: terraphim-ai

**Date**: 2026-04-27 12:09 CEST
**Agent**: Vigil (security-sentinel compliance-watchdog)
**Verdict**: FAIL

---

## 1. Licence Compliance (`cargo deny check licenses`)

**Result**: PASS

All dependency licences are within the approved allowlist. Two minor warnings for unused allowlist entries (`OpenSSL`, `Unicode-3.0`) that are no longer encountered — stale entries in `deny.toml`, not violations.

---

## 2. Dependency Supply Chain (`cargo deny check advisories`, `bans`, `sources`)

**Result**: PASS — significant improvement

```
advisories ok, bans ok, licenses ok, sources ok
```

### Notable improvement

CVEs that persisted for 45+ consecutive audit cycles are now CLEARED:

| Advisory | Status |
|----------|--------|
| RUSTSEC-2026-0097 (rand UB) | CLEARED — crate no longer in dependency tree |
| RUSTSEC-2026-0098 (rustls-webpki TLS bypass) | CLEARED |
| RUSTSEC-2026-0099 (rustls-webpki TLS bypass variant) | CLEARED |
| RUSTSEC-2025-0141 (bincode abandoned) | CLEARED |

The serenity dependency that blocked all of the above for 45+ cycles appears to have been removed or updated. This is the first clean advisory scan in recorded history.

Stale `advisory-not-detected` warnings in `deny.toml` (entries for cleared CVEs) should be pruned to reduce noise.

---

## 3. Port Exposure — Infrastructure Compliance

**Result**: FAIL (P0, recurring)

The following services remain globally exposed despite repeated audit findings:

| Port | Service | Binding | Severity | Cycles Unresolved |
|------|---------|---------|----------|-------------------|
| 6380 | Redis (dragonfly/keydb) | `0.0.0.0:6380`, `[::]:6380` | P0 | 45+ |
| 11434 | Ollama API | `*:11434` | P0 | 45+ |
| 15432 | PostgreSQL (secondary) | `0.0.0.0:15432`, `[::]:15432` | P0 | recurring |
| 15433 | PostgreSQL (secondary) | `0.0.0.0:15433`, `[::]:15433` | P0 | recurring |
| 8984 | Unknown | `0.0.0.0:8984`, `[::]:8984` | P1 | NEW |
| 8443 | Unknown HTTPS | `0.0.0.0:8443`, `[::]:8443` | P1 | NEW |
| 18334 | Unknown | `0.0.0.0:18334`, `[::]:18334` | P1 | NEW |

Remediation (unchanged from prior audits):
- Redis: `bind 127.0.0.1` in `redis.conf`; restart service. Issue #967 tracks this.
- Ollama: `OLLAMA_HOST=127.0.0.1` environment variable or `ListenStream=127.0.0.1:11434` in systemd unit.
- PostgreSQL 15432/15433: confirm whether these are intentionally internet-facing; bind to localhost if not.
- Ports 8984/8443/18334: identify owning process, assess necessity of global exposure.

---

## 4. GDPR / Data Handling Audit

**Result**: PASS (no critical findings)

### Findings

- No PII, passwords, or tokens found in log/debug/info macro call sites (no secrets-in-logs violations).
- Atlassian/JMAP credentials (`token`, `username`) passed as function arguments — not logged.
- Configuration fields for API keys are read from environment variables via `std::env`, not hardcoded.
- No `Display` or `Debug` derives on structs containing sensitive fields that would expose them via tracing.

### Minor observations (informational)

- `crates/haystack_atlassian/src/confluence.rs`: Basic Auth constructed as `username:token` string and Base64-encoded in memory. The raw token is not logged, but the string is kept in a local variable on the stack. Acceptable pattern; no violation.

---

## 5. Unsafe Code Inventory

**Result**: P1 finding persists (unchanged)

| Location | Finding | Severity |
|----------|---------|----------|
| `crates/terraphim_automata/src/sharded_extractor.rs:212` | `deserialize_unchecked` called on artifact bytes with no integrity check (checksum/magic) before the unsafe call | P1 |

22 files contain `unsafe` blocks total. The `sharded_extractor.rs` finding is the highest-risk instance due to external file input. All other `unsafe` blocks reviewed are in test harnesses, FFI wrappers, or performance-critical code with adequate safety comments.

**Remediation**: Add a HMAC or xxHash checksum to the artifact header in `save_to_artifact()` and verify before calling `deserialize_unchecked`. This prevents UB if the artifact file is corrupted or tampered with.

---

## Summary

| Check | Status |
|-------|--------|
| Licence compliance | PASS |
| Advisory supply chain | PASS (45-cycle CVE block CLEARED) |
| Bans / sources | PASS |
| Port exposure | FAIL (P0 recurring) |
| GDPR / data handling | PASS |
| Unsafe code | P1 (sharded_extractor.rs:212) |

**Overall verdict: FAIL** — port exposure P0 findings continue to block compliance sign-off. Supply chain posture is the strongest it has been since auditing began.

---

## Recommendations

1. **Immediate (P0)**: Bind Redis 6380 and Ollama 11434 to localhost. This is a 15-minute fix that has been deferred 45+ audit cycles. Escalate to human operator if automated remediation continues to fail.
2. **Short-term (P1)**: Add integrity verification before `deserialize_unchecked` in `sharded_extractor.rs`.
3. **Hygiene**: Prune stale advisory entries from `deny.toml` (8 `advisory-not-detected` warnings).
4. **Investigate**: Identify services on ports 8984, 8443, 18334 and determine if global exposure is intentional.

---

## 6. Issue #672 — Token Budget Management Code Review

**Updated:** 2026-04-27 13:00 CEST
**Branch:** `task/672-token-budget-management`
**Verdict for code changes: PASS**

### Files Reviewed

| File | Purpose |
|------|---------|
| `crates/terraphim_agent/src/robot/budget.rs` | BudgetEngine — core token budget logic |
| `crates/terraphim_agent/src/robot/schema.rs` | TokenBudget, Pagination, SearchResultItem types |
| `crates/terraphim_agent/src/robot/output.rs` | RobotConfig, FieldMode, RobotFormatter |
| `crates/terraphim_agent/src/main.rs` (lines 710–1901, 3743–3831) | Search command wiring |

### Acceptance Criteria

| Criterion | Status | Evidence |
|-----------|--------|----------|
| `--max-tokens` flag wired to Search | PASS | main.rs:710, 1833–1836 |
| `--max-results` flag wired to Search | PASS | main.rs:713, 1829–1832 |
| `--max-content-length` flag wired | PASS | main.rs:1839–1841 |
| `preview_truncated: true` when content capped | PASS | budget.rs:76–83, schema.rs:318 |
| Pagination `has_more` field present | PASS | schema.rs:144, `Pagination::new` |
| Token budget in response meta envelope | PASS | schema.rs:81, `ResponseMeta::with_token_budget` |
| `cargo test -p terraphim_agent` | PASS | 228 tests, 0 failures |
| `cargo clippy -p terraphim_agent -- -D warnings` | PASS | 0 warnings, 0 errors |

### Security Findings

**Severity: Low — informational, do not block merge**

1. **Silent serialisation fallback** (`budget.rs:108`):
   `serde_json::to_string(item).unwrap_or_default()` returns an empty string on failure (contributing 0 to token estimate). In practice `serde_json::Value` never fails to serialise, so risk is theoretical. Recommend `unwrap_or_else(|_| "{}".to_string())` for readability.

2. **Silent null on field filter failure** (`budget.rs:140`):
   `serde_json::to_value(item).unwrap_or(serde_json::Value::Null)` — same reasoning applies. `SearchResultItem` is fully `Serialize`; failure path is unreachable. Informational only.

3. **Custom field matching is case-insensitive**: User-supplied field names are lowercased before matching `KNOWN_FIELDS`. Correct defensive behaviour — no injection vector.

4. **No upper bound on `--max-tokens`**: Large values are safe — the engine is a filter, not an allocator.

### Unsafe Code

None introduced by #672. No FFI boundaries added.

### Test Coverage

228 unit tests pass covering: field modes (Full/Summary/Minimal/Custom), content length capping, max-results enforcement, token budget progressive truncation, combined budget+results limits, pagination metadata, empty result sets, and serialisation round-trips.

**Issue #672 code verdict: PASS — ready for merge.**
