# Security Audit Report - Terraphim AI
**Date:** 2026-04-04  
**Auditor:** Vigil (Security Engineer)  
**Scope:** terraphim-ai project at /home/alex/terraphim-ai

---

## Executive Summary

**Status: ACTION REQUIRED** - One critical vulnerability identified in TLS certificate validation.

| Category | Count | Severity |
|----------|-------|----------|
| Critical CVEs | 1 | CRITICAL |
| Unmaintained packages | 7 | INFORMATIONAL |
| Hardcoded secrets | 0 | CLEAN |
| Unsafe blocks | 47 | REVIEWED |
| Network exposure | 1 | LOW |

---

## 1. Critical Vulnerabilities (Immediate Action)

### RUSTSEC-2026-0049 - rustls-webpki CRL Bypass

**Severity:** CRITICAL (Privilege Escalation)  
**Package:** rustls-webpki v0.102.8  
**Advisory Date:** 2026-03-20  
**GHSA:** GHSA-pwjx-qhcg-rvj4

**Description:**  
Faulty matching logic causes only the first `distributionPoint` to be evaluated per CRL. Subsequent distribution points are ignored, causing correctly provided CRLs to not be consulted for revocation checking.

**Impact:**
- With `UnknownStatusPolicy::Deny` (default): Safe but incorrect `Error::UnknownRevocationStatus`
- With `UnknownStatusPolicy::Allow`: **Inappropriate acceptance of revoked certificates**

**Attack Scenario:**  
An attacker with access to a compromised but revoked certificate could continue using it if the certificate has multiple distribution points and revocation checking is configured to allow unknown status.

**Remediation:**  
```bash
# Upgrade rustls-webpki to patched version
cargo update -p rustls-webpki
```

**Minimum fixed version:** >=0.103.10

**Dependency Tree (affected crates):**
```
rustls-webpki 0.102.8
└── rustls 0.22.4
    ├── tungstenite 0.21.0
    │   └── tokio-tungstenite 0.21.0
    │       └── serenity 0.12.5
    │           └── terraphim_tinyclaw 1.16.0
    ├── tokio-tungstenite 0.21.0
    └── tokio-rustls 0.25.0
        └── tokio-tungstenite 0.21.0
```

---

## 2. Unmaintained Dependencies (7)

These packages are no longer maintained and should be migrated to alternatives in future releases:

| Package | Version | Advisory | Alternative |
|---------|---------|----------|-------------|
| bincode | 1.3.3 | RUSTSEC-2025-0141 | postcard, rkyv, bitcode |
| fxhash | 0.2.1 | RUSTSEC-2025-0057 | rustc-hash |
| instant | 0.1.13 | RUSTSEC-2024-0384 | web-time |
| number_prefix | 0.4.0 | RUSTSEC-2025-0119 | unit-prefix |
| paste | 1.0.15 | RUSTSEC-2024-0436 | pastey, with_builtin_macros |
| rustls-pemfile | 1.0.4 | RUSTSEC-2025-0134 | rustls-pki-types PemObject trait |
| term_size | 0.3.2 | RUSTSEC-2020-0163 | terminal_size |

**Note:** These are informational warnings. No immediate security impact, but future vulnerabilities will not be patched.

---

## 3. Secrets Scan

**Status:** CLEAN

No hardcoded production secrets, API keys, or tokens found in source code.

**Findings:**
- Environment variable loading for `OPENROUTER_API_KEY`, `GITHUB_TOKEN`, `FIRECRACKER_AUTH_TOKEN` - **Correct pattern**
- Test-only secrets found in `terraphim_github_runner_server/src/webhook/signature.rs` - **Acceptable for tests**

**Evidence:** All credential references use proper environment-based configuration patterns.

---

## 4. Unsafe Code Review

**Status:** REVIEWED - Acceptable with caveats

### 4.1 Production Unsafe Blocks

**terraphim_spawner/src/lib.rs:658**
```rust
unsafe {
    cmd.pre_exec(move || {
        Self::apply_resource_limits(&limits)?;
        Ok(())
    });
}
```
**Assessment:** VALID - setrlimit is async-signal-safe, used correctly between fork and exec. Proper safety comment present.

**terraphim_automata/src/sharded_extractor.rs:211**
```rust
unsafe { DoubleArrayAhoCorasick::<u32>::deserialize_unchecked(bytes) }
```
**Assessment:** CONDITIONALLY VALID - Used for performance-critical artifact loading. Bytes are produced by the same crate's serialize() method. **Recommendation:** Add integrity verification (checksum) before deserialization.

### 4.2 Test/Example Unsafe Blocks

47 total unsafe blocks found. 40+ are in test files and examples using:
- `std::ptr::read()` for test data setup
- `std::env::set_var/remove_var` (Rust 2024 edition requirement)

**Assessment:** Acceptable for testing scenarios.

---

## 5. Network Exposure Analysis

**Status:** LOW RISK

### Active Listeners

| Port | Service | Binding | Risk |
|------|---------|---------|------|
| 3456 | terraphim-llm-p | 0.0.0.0 | **MEDIUM** - Exposed to all interfaces |
| 3004 | terraphim_github_runner | 127.0.0.1 | LOW |
| 8000 | terraphim_server | 127.0.0.1 | LOW |
| 3000 | (frontend) | 127.0.0.1 | LOW |
| 6379 | Redis | 127.0.0.1 | LOW |
| 5432 | PostgreSQL | 127.0.0.1 | LOW |

**Recommendation:** Verify `terraphim-llm-p` on port 3456 binding to 0.0.0.0 is intentional. Consider restricting to localhost if external access is not required.

---

## 6. Recent Commits Review

**Period:** Last 24 hours  
**Status:** CLEAN

Security-relevant commits identified:
- `55f1187c` feat(security-sentinel): agent work [auto-commit]
- `40c793fc` feat(security-sentinel): agent work [auto-commit]
- `38ab9535` feat(drift-detector): agent work [auto-commit]

**Assessment:** Legitimate security tooling development. No suspicious changes.

---

## 7. Action Items

### Immediate (Block Release)
- [ ] **Upgrade rustls-webpki to >=0.103.10** (RUSTSEC-2026-0049)
- [ ] Verify fix with `cargo audit`

### Short Term (Next Sprint)
- [ ] Review terraphim-llm-p binding (port 3456)
- [ ] Add artifact integrity verification to sharded_extractor
- [ ] Document unsafe code justification in terraphim_automata

### Long Term (Backlog)
- [ ] Migrate from bincode to postcard or rkyv
- [ ] Replace rustls-pemfile with rustls-pki-types
- [ ] Update other unmaintained dependencies

---

## 8. Compliance Notes

- **OWASP Top 10:** A02:2021 (Cryptographic Failures) - CRL bypass partially addressed
- **Supply Chain:** RustSec advisory database current (2026-04-02)
- **Dependencies:** 1031 crates scanned, 1026 advisories in database

---

**Report generated by:** Vigil  
**Next audit:** Recommended within 7 days of remediation  
**Contact:** File security concerns as Gitea issues with `security` label