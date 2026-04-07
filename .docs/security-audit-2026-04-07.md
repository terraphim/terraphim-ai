# Security Audit Report - 2026-04-07

**Auditor:** Vigil, Principal Security Engineer
**Status:** 🚫 **FAIL** - Critical vulnerabilities detected
**Verdict:** Block merge until critical issues resolved

---

## Executive Summary

Comprehensive security audit identifies **2 CRITICAL vulnerabilities** that prevent merge:

1. **CVE RUSTSEC-2026-0049** - rustls-webpki TLS validation flaw
2. **Port 3456 Exposed** - terraphim-llm-proxy accessible from all networks

Plus 1 HIGH-priority unmaintained dependency issue.

---

## Critical Vulnerabilities

### 1. CVE RUSTSEC-2026-0049: TLS Certificate Validation Flaw

**Severity:** CRITICAL
**Package:** rustls-webpki 0.102.8
**Issue:** CRL matching logic flaw in TLS certificate validation
**Solution:** Upgrade to rustls-webpki >=0.103.10

**Dependency Chain:**
```
rustls-webpki 0.102.8 → rustls 0.22.4 → tokio-rustls 0.25.0 → 
tokio-tungstenite 0.21.0 → serenity 0.12.5 → terraphim_tinyclaw
```

**Fix:**
```bash
cargo update rustls-webpki
# Verify: rustls-webpki = "0.103.10" in Cargo.lock
cargo audit  # Must show 0 vulnerabilities
```

---

### 2. LLM Proxy Exposed to All Networks

**Severity:** CRITICAL
**Service:** terraphim-llm-proxy (PID 947)
**Finding:** Listening on 0.0.0.0:3456 (all interfaces)
**Risk:** LLM service accessible from untrusted networks without isolation

**Current State:**
```
$ lsof -i :3456
terraphim 947 alex TCP *:3456 (LISTEN)  ← EXPOSED
```

**Required State:**
```
$ ss -tln | grep 3456
tcp 0 0 127.0.0.1:3456 0.0.0.0:* LISTEN  ← LOCALHOST ONLY
```

**Fix:**
1. Update config file (usually /etc/terraphim-llm-proxy/config.toml):
   ```toml
   [server]
   host = "127.0.0.1"  # Change from "0.0.0.0"
   port = 3456
   ```

2. Restart service:
   ```bash
   pkill -f terraphim-llm-proxy
   ss -tln | grep 3456  # Verify localhost binding
   ```

---

## High Priority Issues

### 3. Unmaintained Dependency: bincode 1.3.3

**Severity:** HIGH
**Advisory:** RUSTSEC-2025-0141
**Impact:** Core serialization library without security updates
**Timeline:** Evaluate migration within 30 days
**Options:** bincode2, serde_json, or postcard

---

## Audit Results

### Vulnerability Scan
- ✅ No additional CVEs in dependencies
- ❌ 1 CRITICAL CVE (rustls-webpki)
- ⚠️ 1 HIGH warning (bincode unmaintained)

### Network Isolation
- ❌ 1 service exposed to 0.0.0.0 (port 3456)
- ✅ 12 services properly bound to localhost
- ✅ 3 services bound to specific IPs (VPN/Docker)

### Code Security
- ✅ No hardcoded secrets detected
- ✅ No recent security regressions
- ⚠️ Unsafe code present (separate review needed)

---

## Remediation Path

**Phase 1: Fix CVE (< 2 hours)**
```bash
cargo update rustls-webpki
cargo audit  # Verify 0 vulnerabilities
cargo build --release
git commit -m "fix(security): upgrade rustls-webpki CVE-2026-0049"
```

**Phase 2: Fix Port Exposure (< 1 hour)**
```bash
# Update config: host = "127.0.0.1"
pkill -f terraphim-llm-proxy
ss -tln | grep 3456  # Verify localhost binding
```

**Phase 3: Re-verification**
```bash
cargo audit                  # Must show 0
ss -tln | grep 3456        # Must show 127.0.0.1
lsof -i :3456              # Verify localhost binding
```

---

## Gate Decision: FAIL

**Cannot merge with critical vulnerabilities present.**

**Blockers:**
- [ ] CVE RUSTSEC-2026-0049 must be resolved
- [ ] Port 3456 must be bound to localhost
- [ ] Both fixes must pass re-verification

**Required Before Approval:**
1. Both critical issues fixed and verified
2. `cargo audit` showing 0 vulnerabilities
3. `ss -tln` showing localhost binding for port 3456
4. Full test suite passing
5. Re-verification from security team

---

**Audit Date:** 2026-04-07 04:34 CEST
**Posted to Gitea:** Issue #438
**Verdict:** BLOCK - Re-verify after remediation
