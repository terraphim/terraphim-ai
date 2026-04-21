# Security Audit Session 2026-04-21

**Agent:** Vigil (security-sentinel)
**Date:** 2026-04-21  
**Branch:** task/696-repl-format-robot-dispatch
**Verdict:** **FAIL** - 4 critical/unsound vulnerabilities persistent

## CVE Findings

### Critical (TLS Bypass)
- **RUSTSEC-2026-0098**: rustls-webpki 0.101.7 + 0.102.8 - Name constraints for URI names incorrectly accepted
  - Path: serenity 0.12.5 → rustls 0.22.4 → rustls-webpki 0.102.8
  - Affects: TLS validation for domain names in certificates

- **RUSTSEC-2026-0099**: rustls-webpki 0.101.7 + 0.102.8 - Wildcard name constraints accepted
  - Same dependency path as RUSTSEC-2026-0098
  - Blocks: All HTTPS connections until remediated

### Critical (Cryptographic Unsoundness)
- **RUSTSEC-2026-0097**: rand 0.8.5, 0.9.2, 0.10.0 - Unsound with custom loggers
  - Affects: cryptographic randomness in tungstenite, sqlx-postgres, slack-morphism
  - Impact: Weakened RNG in Discord bot (serenity), database connections

### High (Unmaintained)
- **RUSTSEC-2025-0141**: bincode 1.3.3 unmaintained serialisation library
- **RUSTSEC-2024-0384**: instant 0.1.13 unmaintained timing library
- **RUSTSEC-2025-0119**: number_prefix 0.4.0 unmaintained progress bar
- **RUSTSEC-2024-0436**: paste 1.0.15 unmaintained proc-macro helper
- **RUSTSEC-2025-0134**: rustls-pemfile 1.0.4 unmaintained certificate parser
- **RUSTSEC-2020-0163**: term_size 0.3.2 unmaintained terminal size detection

## Infrastructure Finding

- **Port 11434 (IPv6 Wildcard)**: `:::11434 LISTEN`
  - Ollama exposed on IPv6 0.0.0.0::/0
  - Allows unauthenticated LLM access from network boundary
  - Remediation: Bind to 127.0.0.1:11434 only

## Code Security Assessment

- **Hardcoded Secrets**: 0 found (safe)
- **Unsafe Blocks**: 0 found (safe-first architecture)
- **Recent Commits**: No security-relevant changes in past 24h

## Root Cause Analysis

**Architectural Blocker:** serenity 0.12.5 Discord bot library pins rustls 0.22.4 (December 2023, unmaintained). Upstream fix exists (commit 62b504fc removes serenity) but not merged to this branch.

**Decision Status:** Awaiting product owner decision on remediation:
- **Option A (Preferred)**: Remove serenity dependency - unblocks all TLS CVEs
- **Option B**: Fork serenity and update rustls dependency
- **Option C**: Accept CVE risk until upstream fix merged

## Audit History

This is the **43rd consecutive audit** with identical TLS findings:
- Sessions: 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43+
- Duration: 40+ cycles (24+ days)
- Remediation Attempts: 0 code changes
- Status: **Persistent, unresolved, awaiting decision**

## Dispatch Context

- **Mention Issue**: None (no explicit dispatch context provided)
- **Branch Task**: task/696-repl-format-robot-dispatch (feature task, unrelated)
- **Posting**: This audit documented as standing security mandate

---

**Next Actions:**
1. Escalate serenity decision to product owner
2. If serenity removal approved: remove dependency, re-audit (expect PASS)
3. If deferred: update threshold CVEs for next audit cycle
