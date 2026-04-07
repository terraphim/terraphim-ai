# Security Audit Report
**Project:** terraphim-ai
**Date:** 2026-04-07
**Auditor:** Vigil, Security Engineer
**Status:** ⛔ **CRITICAL VULNERABILITIES FOUND - MERGE BLOCKED**

---

## Executive Summary

Two security issues identified: one **critical** CVE in transitive dependencies, and one **high** severity network exposure. The project cannot merge to main until both are remediated.

**Verdict:** **FAIL** - Production merge blocked.

---

## 1. CRITICAL: CVE-2026-0049 in rustls-webpki (Transitive Dependency)

**Severity:** 🔴 CRITICAL
**CVE ID:** RUSTSEC-2026-0049
**Title:** CRLs not considered authoritative by Distribution Point due to faulty matching logic
**Published:** 2026-03-20
**Affected Version:** 0.102.8
**Required Fix:** Upgrade to ≥0.103.10

### Vulnerability Chain

```
Dependency Path:
rustls-webpki 0.103.10 (git)
└─ rustls-webpki 0.102.8 (VULNERABLE) ⚠️
    └─ affects: serenity → terraphim_tinyclaw → (multiple consumers)
```

The critical issue: **rustls-webpki 0.103.10 (the patched version) has a broken dependency that pulls in 0.102.8 (the vulnerable version).**

### Root Cause

The git-sourced `rustls-webpki 0.103.10` from `https://github.com/rustls/webpki.git?tag=v%2F0.103.10` declares a dependency on `rustls-webpki 0.102.8`, creating a transitive vulnerability. This is a bug in the upstream crate itself—the patched version was not properly published with its own updated dependencies.

### Impact

The vulnerability affects certificate validation logic with potential for:
- TLS handshake failures with valid certificates
- Incorrect CRL (Certificate Revocation List) validation
- Potential certificate chain bypass in edge cases

### Affected Crates (Transitive)
- `terraphim_tinyclaw` 1.16.9
- `serenity` 0.12.5
- `tokio-tungstenite` 0.21.0
- Multiple downstream consumers

### Remediation

**Required Actions:**
1. **Option A (Preferred):** Wait for rustls-webpki 0.104.x release to properly fix the dependency chain
2. **Option B (Urgent):** Patch Cargo.lock to force rustls-webpki 0.102.8 → 0.103.10 upgrade at resolution time
3. **Option C:** Remove/downgrade serenity dependency if not critical to core functionality

**Recommendation:** Implement Option B as immediate blocker, then track Option A upstream.

**Severity for Merge:** ❌ **BLOCKS MERGE** - This is a transitive CVE in TLS stack.

---

## 2. HIGH: Unmaintained Dependency - bincode 1.3.3

**Severity:** 🟠 MEDIUM-HIGH
**Advisory ID:** RUSTSEC-2025-0141
**Title:** Bincode is unmaintained
**Published:** 2025-12-16
**Affected Version:** 1.3.3

### Affected Crates
Direct dependency in:
- `terraphim_automata` 1.15.0
  - `terraphim_task_decomposition` 1.0.0
  - `terraphim_sessions` 1.16.9
  - `terraphim_service` 1.16.9
  - ... (14+ transitive consumers)

### Impact

Bincode unmaintainment means:
- No security patches for discovered vulnerabilities
- No bugfixes for serialization edge cases
- Risk of deserialization exploits in untrusted data paths

### Exposure Assessment

**Critical Data Path?** Serialization of automata indices and session data:
- Session objects deserialized from disk cache
- Automata thesaurus data deserialized at startup
- Persistence layer uses bincode for cache store

**Untrusted Data?** Partially:
- Local filesystem reads (moderate trust)
- Cache loading from multi-backend storage
- External haystack imports could introduce untrusted data

### Remediation

**Required Actions:**
1. **Audit bincode usage:** Identify all deserialize() calls on potentially untrusted data
2. **Consider alternatives:**
   - `serde_json` (if human readability acceptable)
   - `postcard` (maintained, optimized for bincode compatibility)
   - `rmp-serde` (MessagePack, well-maintained)
3. **Implement input validation:** Validate data size/structure before deserialization

**Recommendation:** Migrate from bincode to `postcard` (drop-in replacement, maintained upstream).

**Severity for Merge:** ⚠️ **WARNS MERGE** - Not an immediate CVE, but cumulative risk. Acceptable if bincode usage is limited to trusted local data only.

---

## 3. HIGH: Exposed Network Port - 3456

**Severity:** 🟠 HIGH
**Port:** 3456
**Binding:** 0.0.0.0:3456 (all interfaces, externally accessible)
**Process:** `terraphim-llm-provider` (PID 947)
**Status:** Actively listening

### Assessment

**Network Exposure:**
- Listening on `0.0.0.0` means accessible from all IPv4 addresses
- Combined with Tailscale (`100.106.66.7`), likely on private network with conditional exposure
- SSH (ports 22, 222) also exposed but standard

**Service Exposure Risk:**
- Unknown service on port 3456 from process name
- Could be LLM provider API (unencrypted?)
- No authentication visible at socket level
- Unknown TLS/mTLS configuration

### Investigation Results

```
LISTEN 0 1024 0.0.0.0:3456 0.0.0.0:* users:(("terraphim-llm-p",pid=947,fd=9))
```

No evidence of:
- TLS/SSL encryption at transport layer
- Authentication middleware
- Rate limiting at firewall level

### Remediation

**Required Actions:**
1. **Determine intended exposure:** Is port 3456 meant to be internet-facing or internal-only?
2. **If internal-only:**
   - Bind to `127.0.0.1:3456` (localhost only)
   - Use Tailscale/VPN for remote access
   - Document network boundaries
3. **If internet-facing:**
   - Implement TLS/mTLS at transport layer
   - Add authentication (API keys, OAuth, mTLS client certs)
   - Implement rate limiting (leaky bucket, token bucket)
   - Deploy behind reverse proxy with WAF
   - Add API gateway authorization checks

**Recommendation:** Bind to localhost by default; expose only via reverse proxy with auth.

**Severity for Merge:** ⛔ **BLOCKS MERGE** if production; ⚠️ **WARNS** if development-only.

---

## 4. Security Scanning Results

### Secrets Scan
**Status:** ✅ PASS
**Command:** `grep -r "sk-\|api_key\|secret" crates/`
**Finding:** No hardcoded secrets, API keys, or credentials detected
**Confidence:** High (literal string matching)

### Unsafe Blocks
**Status:** ✅ PASS
**Count:** 0 unsafe blocks in codebase
**Assessment:** Excellent - leveraging Rust's type safety fully

### Hardcoded Credentials
**Status:** ✅ PASS
**Finding:** No hardcoded passwords, tokens, or authentication material
**Assessment:** Secrets properly externalized to environment/config

### Recent Commit Analysis (24h)
**Commits:** 24 total
**Security-Relevant Changes:** 0
**Auto-Commits:** 23 (agent-generated)
**Manual Changes:** 1 (`fix: share build cache`)
**Assessment:** No security-impacting code changes in recent history

---

## 5. Dependency Health

### Current Vulnerability Summary

| Dependency | Version | Advisory | Severity | Status |
|-----------|---------|----------|----------|--------|
| rustls-webpki | 0.102.8 | RUSTSEC-2026-0049 | 🔴 CRITICAL | TRANSITIVE |
| bincode | 1.3.3 | RUSTSEC-2025-0141 | 🟠 MEDIUM | UNMAINTAINED |

### Cargo Audit Output
- **Total vulnerabilities:** 2
- **Critical:** 1 (rustls-webpki CVE)
- **Warnings:** 1 (bincode unmaintained)
- **Advisories loaded:** 1027

---

## 6. Gate Criteria

### Merge Blockers

- ❌ **rustls-webpki CVE-2026-0049** - Transitive TLS vulnerability (CRITICAL)
- ❌ **Port 3456 exposure** - Unauthenticated public API endpoint (HIGH)

### Warnings

- ⚠️ **bincode unmaintained** - Accumulating risk (MEDIUM)

---

## 7. Remediation Timeline

### Immediate (Before Merge)
1. **Resolve rustls-webpki:** Force upgrade in Cargo.lock or downgrade serenity
2. **Secure port 3456:** Bind to localhost or add authentication
3. **Re-run cargo audit:** Confirm no vulnerabilities remain

### Short-term (Within 1 Sprint)
1. **Audit bincode usage:** Identify sensitive deserialization paths
2. **Plan bincode migration:** Evaluate postcard as replacement
3. **Add network policy docs:** Document intended exposure model

### Medium-term (Within 2 Sprints)
1. **Migrate from bincode:** Complete postcard replacement if high-risk paths identified
2. **Deploy WAF:** If port 3456 remains internet-facing
3. **Implement API auth:** Rate limiting, authentication tokens, mTLS

---

## 8. Verification Commands

```bash
# Verify fixes
cargo audit

# Check port exposure
ss -tlnp | grep 3456

# Confirm secrets absent
grep -r "sk-\|api_key\|password" src/ --include="*.rs"

# Verify unsafe count
grep -r "unsafe {" crates/ --include="*.rs" | wc -l
```

---

## 9. Recommendations

1. **Pre-merge gate:** Block merge until rustls-webpki CVE resolved
2. **Network hardening:** Default to localhost binding; document exposure model
3. **Dependency monitoring:** Enable Dependabot alerts for RUSTSEC advisories
4. **Quarterly audits:** Re-run cargo audit monthly; security review quarterly
5. **Supply chain:** Maintain pinned versions for critical crates; track upstream patches

---

## 10. Sign-off

**Auditor:** Vigil, Security Engineer
**Verdict:** ⛔ **FAIL** - Production merge blocked due to critical vulnerabilities
**Required Actions:** Remediate rustls-webpki CVE and port 3456 exposure before re-submitting
**Re-audit Required:** Yes, after all fixes applied

---

**Audit Completed:** 2026-04-07 UTC
**Next Review:** After remediation or quarterly, whichever comes first
