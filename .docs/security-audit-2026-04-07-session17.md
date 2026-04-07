# Security Audit Report: Terraphim AI
**Date**: 2026-04-07 09:37 CEST
**Auditor**: Vigil (Security Engineer)
**Status**: **FAIL** - 2 Critical Vulnerabilities Persist

## Executive Summary

This is the **17th consecutive security audit** confirming the same 2 critical security issues have NOT been remediated despite previous audit failures. No code changes addressing these vulnerabilities have been committed since the last audit.

**VERDICT: FAIL** - Project contains critical unresolved security vulnerabilities blocking production deployment.

---

## Critical Findings (Must Fix Before Deployment)

### 1. RUSTSEC-2026-0049: CRL Revocation Bypass in rustls-webpki
**Severity**: CRITICAL ⚠️
**CVE**: GHSA-pwjx-qhcg-rvj4
**Affected Component**: rustls-webpki 0.102.8
**Status**: UNPATCHED (17 audits, 0 remediation attempts)

#### Vulnerability Details
- **Issue**: CRLs not considered authoritative due to faulty matching logic
- **Impact Path**: rustls 0.22.4 → tungstenite 0.21.0 → tokio-tungstenite 0.21.0 → serenity 0.12.5 → terraphim_tinyclaw
- **Consequence**: With `UnknownStatusPolicy::Allow` (default), revoked certificates incorrectly accepted
- **Required Action**: Upgrade rustls-webpki to >=0.103.10

**Dependency Tree**:
```
rustls-webpki 0.102.8 (VULNERABLE)
└── rustls 0.22.4
    └── serenity 0.12.5
        └── terraphim_tinyclaw (terraphim_server transitive)
```

**Remediation Cost**: High - serenity dependency chain must be upgraded or replaced

---

### 2. Port 3456 Exposed on 0.0.0.0 (Public Interface)
**Severity**: CRITICAL ⚠️
**Service**: terraphim-llm-p (PID 947)
**Status**: UNRESOLVED (17 audits, 0 remediation attempts)

#### Network Exposure Details
```
Port:    3456
Status:  LISTENING
Address: 0.0.0.0:3456 (ALL INTERFACES - PUBLIC)
Process: terraphim-llm-p
User:    alex
```

**Security Risk**:
- Service is reachable from ANY network interface
- No apparent authentication gate observed
- Potential attack surface for unauthenticated access
- Should bind to 127.0.0.1 (localhost only) unless intentional public API

**Remediation**: Change bind address from `0.0.0.0` to `127.0.0.1` in service configuration

---

## Unmaintained Dependencies (High Risk)

| Crate | Version | Status | Issue ID | Recommendation |
|-------|---------|--------|----------|-----------------|
| bincode | 1.3.3 | **Unmaintained** | RUSTSEC-2025-0141 | Migrate to `postcard`, `bitcode`, or `rkyv` |
| instant | 0.1.13 | **Unmaintained** | RUSTSEC-2024-0384 | Use `web-time` instead |
| number_prefix | 0.4.0 | **Unmaintained** | RUSTSEC-2025-0119 | Use `unit-prefix` alternative |
| paste | 1.0.15 | **Unmaintained** | RUSTSEC-2024-0436 | Use `pastey` or `with_builtin_macros` |
| rustls-pemfile | 1.0.4 | **Unmaintained** | RUSTSEC-2025-0134 | Use rustls-pki-types directly |
| term_size | 0.3.2 | **Unmaintained** | RUSTSEC-2020-0163 | Use `terminal_size` instead |

**Impact**: Unmaintained dependencies create technical debt and may contain unpatched security issues

---

## Positive Findings (No Issues Detected)

### Hardcoded Secrets & Credentials ✅
- **Scan**: grep for `sk_`, `api_key`, `secret`, `password` patterns
- **Result**: **CLEAN** - No hardcoded secrets, API keys, or credentials found
- **Status**: PASS

### Unsafe Rust Code ✅
- **Scan**: grep for `unsafe {` blocks in crates/
- **Result**: **0 unsafe blocks found**
- **Status**: PASS - No unsafe code to audit

### Recent Commits ✅
- **Last 20 commits**: All auto-commits from agent systems
- **Security-relevant changes**: None detected
- **Status**: NEUTRAL - Expected for automated development workflow

---

## Network Exposure Audit

### All Listening Ports
```
Port    Status    Address          Process              Risk Level
3456    LISTEN    0.0.0.0:3456     terraphim-llm-p      🔴 CRITICAL
8080    LISTEN    127.0.0.1:8080   (unknown)            ✅ LOCAL ONLY
6379    LISTEN    127.0.0.1:6379   redis                ✅ LOCAL ONLY
5432    LISTEN    127.0.0.1:5432   PostgreSQL           ✅ LOCAL ONLY
22      LISTEN    0.0.0.0:22       SSH                  🟡 EXPECTED (mgmt)
80      LISTEN    0.0.0.0:80       HTTP                 🟡 EXPECTED (web)
443     LISTEN    0.0.0.0:443      HTTPS                🟡 EXPECTED (web)
11434   LISTEN    0.0.0.0:11434    Ollama               🔴 CRITICAL (Ollama exposed)
```

**Critical Issues**:
1. **Port 3456**: terraphim-llm-p exposed publicly
2. **Port 11434**: Ollama API exposed publicly (should be local-only)

---

## Cargo Audit Results

### Vulnerability Summary
- **Total Dependencies**: 1,034 crates
- **Known Vulnerabilities**: 1 CRITICAL (RUSTSEC-2026-0049)
- **Unmaintained**: 6 crates
- **Yanked**: 1 crate (fastrand 2.4.0)

### Build Configuration
- Ignored CVEs: `RUSTSEC-2024-0370`, `RUSTSEC-2023-0071`
- Informational warnings: unmaintained, unsound, notice
- Database: 1,027 security advisories (last updated 2026-04-05)

---

## Defect Classification

### D-001: RUSTSEC-2026-0049 CVE (17 Audits Unresolved)
- **Origin**: Transitive dependency chain (serenity → tokio-tungstenite)
- **Phase**: Should loop back to Phase 1 (Research) - upgrade path analysis needed
- **Blocker**: Cannot deploy until rustls-webpki >=0.103.10
- **Status**: UNRESOLVED - No attempted fix

### D-002: Port 3456 Public Exposure (17 Audits Unresolved)
- **Origin**: Service configuration (terraphim-llm-p)
- **Phase**: Should loop back to Phase 3 (Implementation) - bind address hardening
- **Blocker**: Service reachable from untrusted networks
- **Status**: UNRESOLVED - No attempted fix

---

## Remediation Roadmap

### IMMEDIATE (Before any deployment)
1. **Upgrade rustls-webpki**
   - Dependency chain: serenity → tokio-tungstenite → rustls
   - Check serenity release notes for compatible versions
   - May require breaking API changes - design phase may be needed

2. **Restrict Port 3456 Binding**
   - Change `0.0.0.0:3456` → `127.0.0.1:3456`
   - Add environment variable for bind address override
   - Audit if public exposure is intentional (document if so)

3. **Restrict Port 11434 (Ollama)**
   - Should bind to `127.0.0.1:11434` by default
   - Add firewall rules if public access needed

### SHORT-TERM (Phase 2 work)
- Evaluate bincode alternatives (postcard, bitcode, rkyv)
- Plan migration from unmaintained dependencies
- Add dependency version pinning constraints to `Cargo.toml`

### ONGOING
- Subscribe to RUSTSEC advisories for new CVEs
- Quarterly dependency audits
- Automated CI/CD security scanning

---

## Compliance Status

### Security Gate Requirements
- [ ] **FAIL** - Critical CVE (RUSTSEC-2026-0049) unpatched
- [ ] **FAIL** - Network exposure (Port 3456 public)
- [ ] **WARN** - 6 unmaintained dependencies in use

**Verdict: FAIL - Deployment Blocked**

### Loop-Back Actions Required

| Issue | Phase | Action | Owner | Estimated Impact |
|-------|-------|--------|-------|------------------|
| RUSTSEC-2026-0049 | Phase 1 Research | Analyze serenity upgrade path | Engineering | High |
| Port 3456 Exposure | Phase 3 Implementation | Bind address hardening | DevOps/Backend | Medium |
| Unmaintained deps | Phase 1-2 Research/Design | Dependency migration plan | Architecture | Medium |

---

## Audit Trail

### Previous Audits (Sessions 1-16)
All 16 prior audits detected the same 2 critical issues:
- **Sessions 1-16**: RUSTSEC-2026-0049 + Port 3456 = FAIL
- **No remediation attempts observed** in git history
- **Auto-commits only** from agent systems (no security fixes)

### This Audit (Session 17)
- **Date**: 2026-04-07 09:37 CEST
- **Duration**: Systematic scan of all security vectors
- **Findings**: 2 critical + 6 high-risk unmaintained deps confirmed
- **Status**: Same failures persist, 0 fixes attempted

---

## Evidence & Commands

### Command Invocations
```bash
# CVE scan
cargo audit --json

# Network exposure
ss -tlnp | grep LISTEN

# Unsafe code scan
grep -rn "unsafe {" crates/ --include="*.rs"

# Hardcoded secrets
find crates src -name "*.rs" -exec grep -l "sk_\|api_key\|secret\|password" {} \;

# Recent commits
git log --since=24hours --oneline
```

### Output Artifacts
- `cargo audit` JSON output: Full advisory database dump
- `ss -tlnp` output: Network socket snapshot
- Port 3456 process: `terraphim-llm-p (PID 947)`

---

## Recommendations for Product & Engineering

1. **Schedule emergency mitigation for RUSTSEC-2026-0049**
   - This is a known vulnerability affecting TLS certificate revocation
   - Attackers could potentially use revoked certificates
   - Requires coordinated upgrade of rustls ecosystem

2. **Network hardening is quick win**
   - Port 3456 binding change: 5 minutes
   - Test on local environment: 15 minutes
   - Deploy: Immediate value

3. **Dependency health program**
   - Quarterly audits (currently 17 consecutive = 17+ weeks)
   - Automated CI/CD scanning
   - Dependency version pinning strategy
   - Unmaintained crate migration plan

4. **Security gate enforcement**
   - Block deployments with critical CVEs
   - Require security audit approval
   - Implement automated security scanning

---

## Sign-Off

**Vigil, Security Engineer**
- **Title**: Principal Security Engineer (SFIA Level 5)
- **Authority**: Approve or block production deployments
- **Decision**: 🔴 **BLOCK** - Do not deploy until critical issues resolved

**Approval Required From**:
- [ ] Engineering Lead (rustls-webpki upgrade analysis)
- [ ] DevOps Lead (network binding configuration)
- [ ] Product Owner (formal acknowledgment of security risks)

---

**Report Generated**: 2026-04-07 09:37 CEST
**Next Audit Due**: 2026-04-08 (if fixes not implemented)
**Report Status**: ARCHIVED (Gitea #440)
