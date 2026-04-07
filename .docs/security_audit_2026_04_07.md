# Security Audit Report
**Date**: 2026-04-07 03:49 CEST
**Scope**: terraphim-ai project at /home/alex/terraphim-ai
**Auditor**: Vigil, Security Engineer
**Status**: **FAIL** - Critical vulnerabilities detected

---

## Executive Summary

The terraphim-ai project contains **1 critical CVE** and **1 critical network exposure** that must be resolved before production deployment or merge to main branch.

**Verdict**: 🔴 **SECURITY GATE FAILED**

---

## Critical Findings

### 1. CVE: RUSTSEC-2026-0049 - Certificate Revocation Checking Bypass

**Severity**: CRITICAL
**CVSS**: Medium-High (privilege escalation category)
**Package**: `rustls-webpki` v0.102.8
**Status**: UNPATCHED IN LOCKED DEPENDENCIES

#### Vulnerability Details
- **ID**: RUSTSEC-2026-0049 / GHSA-pwjx-qhcg-rvj4
- **Title**: CRLs not considered authoritative by Distribution Point due to faulty matching logic
- **Impact**: Certificate revocation checking is bypassed when certificates have multiple distribution points
- **Affected Component**: Certificate validation chain in TLS/HTTPS connections
- **Introduced**: rustls-webpki < 0.102.0
- **Fixed in**: rustls-webpki >= 0.103.10

#### Attack Vector
1. Attacker obtains a revoked certificate (requires compromise of trusted CA)
2. Certificate has multiple `distributionPoint` entries
3. Only first distribution point is checked against CRL
4. Subsequent distribution points ignored
5. Revocation status unknown → accepted (with `UnknownStatusPolicy::Allow`)
6. Revoked credential continues to be accepted

#### Detection
```
$ cargo audit --json | jq '.vulnerabilities.list[] | select(.advisory.id == "RUSTSEC-2026-0049")'
Found: rustls-webpki 0.102.8 in Cargo.lock
```

#### Remediation Required
```bash
# Pull request needed to upgrade rustls-webpki
# Current state: Cargo.lock contains BOTH patched (0.103.10) and vulnerable (0.102.8) versions
# - 0.103.10 (git tag) - patched version (source: rustls repository)
# - 0.102.8 - vulnerable version (source: crates.io) <- MUST REMOVE

# The dependency chain shows rustls crate is pulling 0.102.8:
# rustls -> rustls-webpki 0.102.8
```

**Action Required**:
- [ ] Bump rustls crate to version that uses rustls-webpki >= 0.103.10
- [ ] Run `cargo update` and verify Cargo.lock
- [ ] Run `cargo audit` to confirm resolution
- [ ] Test TLS certificate validation thoroughly

**Blocking**: ✅ YES - Merge blocked until resolved

---

### 2. Critical Network Exposure: Port 3456 Open to 0.0.0.0

**Severity**: CRITICAL
**Type**: Network Exposure / Misconfiguration
**Service**: terraphim-llm-p (PID 947)

#### Current State
```
LISTEN 0 1024 0.0.0.0:3456 0.0.0.0:* users:(("terraphim-llm-p",pid=947,fd=9))
```

#### Findings
- ✅ Default configuration: `127.0.0.1:8000` (localhost only)
- ✅ Dev configuration: `127.0.0.1:8000` (localhost only)
- ❌ **Running process binds to 0.0.0.0:3456** - exposed to all network interfaces
- ❌ **No network authentication** visible on listening port
- ❌ **Firewall check**: Port accessible from external networks

#### Risk Assessment
**High Risk**: Service on 0.0.0.0 binding accepts connections from:
- All IPv4 addresses on the system
- All IPv4 networks (if port-forwarded)
- All IPv6 addresses (if dual-stack)

#### Remediation Required
```bash
# Investigation steps:
1. Identify what spawns terraphim-llm-p process
2. Check environment variables: TERRAPHIM_SERVER_HOSTNAME
3. Check configuration override mechanism
4. Verify no production config has 0.0.0.0 binding

# Immediate actions:
- Check systemd/supervisor config for bind address
- Audit startup scripts for hardcoded 0.0.0.0
- Add network policy: restrict to 127.0.0.1 in production
- Add CI check: reject Cargo changes with 0.0.0.0 defaults
```

**Action Required**:
- [ ] Identify what is starting the terraphim-llm-p process
- [ ] Check all configuration sources (env vars, config files, systemd units)
- [ ] Restrict binding to 127.0.0.1 for localhost-only access
- [ ] Document intended network topology and firewall rules
- [ ] Add pre-commit check to prevent 0.0.0.0 in configs

**Blocking**: ✅ YES - Cannot merge without understanding this exposure

---

## Warnings: Unmaintained Dependencies

**Severity**: MEDIUM (deferred maintenance risk)

| Package | Version | Status | Notes |
|---------|---------|--------|-------|
| `bincode` | 1.3.3 | ⚠️ Unmaintained | Team ceased development after harassment. Recommend postcard, bitcode, or rkyv |
| `instant` | 0.1.13 | ⚠️ Unmaintained | Recommend `web-time` |
| `number_prefix` | 0.4.0 | ⚠️ Unmaintained | Recommend `unit-prefix` |
| `paste` | 1.0.15 | ⚠️ Unmaintained | Repository archived. Recommend `pastey` or `with_builtin_macros` |
| `rustls-pemfile` | 1.0.4 | ⚠️ Unmaintained | Repository archived (Aug 2025). Code integrated in rustls-pki-types since 1.9.0 |
| `term_size` | 0.3.2 | ⚠️ Unmaintained | Recommend `terminal_size` |
| `fastrand` | 2.4.0 | ⚠️ Yanked | Check if updates available |

**Action**: Create follow-up issue to migrate away from unmaintained crates (non-blocking for this merge)

---

## Passing Security Checks

### ✅ Unsafe Code Review
- **Unsafe blocks found**: 0
- **Status**: PASS
- **Finding**: No unsafe code in current codebase

### ✅ Secrets Scanning
- **Hardcoded secrets detected**: 0
- **Status**: PASS
- **Patterns checked**: `sk-*`, `api_key`, `SECRET`, `PASSWORD`, `API_KEY`
- **False positives reviewed**: None

### ✅ Recent Commit Review
- **Security-relevant commits (24h)**: 5 security-sentinel agent commits
- **Concerning changes**: None identified
- **Status**: PASS

---

## Dependency Analysis

### Cargo.lock Status
```
Total dependencies: 1,034
CVE database entries: 1,027 (last updated: 2026-04-05)
Known vulnerabilities: 1 CRITICAL
Unmaintained packages: 6 (informational warnings)
Yanked packages: 1
```

### Vulnerable Dependency Chain
```
rustls-webpki v0.102.8 (VULNERABLE - RUSTSEC-2026-0049)
  ↓ (pulled by)
rustls crate (dependency)
  ↓ (propagates to)
All TLS/HTTPS-enabled services
```

### Patched Version Available
```
rustls-webpki v0.103.10
  ✅ GHSA-pwjx-qhcg-rvj4 FIXED
  ✅ Already in Cargo.lock (from git tag)
  ⚠️  Not being used due to rustls crate constraint
```

---

## Network Security Assessment

### Port Analysis
```
Port 3456/TCP (EXPOSED - CRITICAL)
  Service: terraphim-llm-p
  Binding: 0.0.0.0:3456 (all interfaces)
  Status: LISTENING
  Risk: Accessible from any network
  Credentials: Unknown auth mechanism

Port 22/TCP (SSH)
  Status: Normal
  Risk: Standard attack surface

Port 80/TCP (HTTP)
  Status: LISTENING (port 0.0.0.0:80)
  Risk: Unencrypted traffic

Port 443/TCP (HTTPS)
  Status: LISTENING (port 0.0.0.0:443)
  Risk: Reverse proxy likely (standard)

Other services (127.0.0.1):
  PostgreSQL, Redis, Quickwit, SCC Cache, Ollama, etc.
  Status: Localhost only (safe)
```

---

## Compliance & Standards

### OWASP Top 10
- ✅ A03:2021 Injection - No hardcoded SQL/commands
- ✅ A04:2021 Insecure Design - Default config safe (localhost)
- ⚠️ A01:2021 Broken Authentication - 0.0.0.0 binding bypasses network auth
- ⚠️ A07:2021 Identification & Auth Failure - Exposed LLM port

### CWE
- ⚠️ CWE-295: Improper Certificate Validation (rustls-webpki CVE)
- ⚠️ CWE-639: Authorization Bypass Through User-Controlled Key (network exposure)

---

## Remediation Timeline

### IMMEDIATE (Before ANY merge)
1. [ ] Upgrade rustls to resolve RUSTSEC-2026-0049
2. [ ] Identify/document why port 3456 is on 0.0.0.0
3. [ ] Restrict binding to 127.0.0.1
4. [ ] Run `cargo audit` to confirm clean bill of health

### SHORT TERM (This sprint)
1. [ ] Create follow-up issue: "Migrate from unmaintained dependencies"
2. [ ] Document firewall rules and network topology
3. [ ] Add CI check to prevent 0.0.0.0 regressions
4. [ ] Security training on credential/secret handling

### MEDIUM TERM (Next quarter)
1. [ ] Migrate bincode → postcard/bitcode/rkyv
2. [ ] Migrate term_size → terminal_size
3. [ ] Review and update other unmaintained crates
4. [ ] Quarterly security audit cycle

---

## Audit Methodology

### Tools & Techniques Used
- `cargo audit` - CVE database scanning
- `Cargo.lock` analysis - Dependency version verification
- `grep` patterns - Secrets scanning (sk-*, api_key, password, etc.)
- `ss -tlnp` - Network port enumeration
- Manual code review - Unsafe block verification
- `git log` - Security-relevant commit analysis

### Sources
- RUSTSEC Advisory Database (RustSec GitHub Advisory Database)
- Cargo.lock version pinning constraints
- Network interface listening state
- Process environment inspection

---

## Conclusion

**🔴 SECURITY GATE: FAIL**

The project has **2 blocking vulnerabilities** that must be resolved before merge:

1. **CVE in rustls-webpki** - Critical certificate validation bypass
2. **Network exposure on port 3456** - Unintended public binding

**Recommendation**: Do not merge until:
- [ ] CVE RUSTSEC-2026-0049 is patched
- [ ] Port 3456 binding is documented and restricted to localhost
- [ ] `cargo audit` reports zero critical vulnerabilities
- [ ] Network configuration is verified against security policy

---

## Sign-off

| Role | Status | Notes |
|------|--------|-------|
| Security Engineer (Vigil) | 🔴 FAIL | Critical issues block merge |
| Merge Coordinator | ⏸️ WAITING | Cannot approve until security gate passes |

**Audit completed**: 2026-04-07 03:49 CEST
**Next review**: After remediation (estimate: 24 hours)
**Escalation**: Yes - Post verdict to Gitea issue
