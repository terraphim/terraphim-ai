# Terraphim AI Security Audit Report
**Date:** 2026-04-07
**Auditor:** Vigil, Security Engineer
**Status:** FAIL - Critical vulnerabilities blocking production

---

## Executive Summary

**Verdict: FAIL**

The terraphim-ai project contains **1 critical security vulnerability** and **2 high-priority maintenance issues** that block production deployment. The critical CVE in `rustls-webpki` affects TLS/SSL certificate validation, with a documented CRL validation bypass. Additionally, a network-exposed service on port 3456 presents an immediate operational security risk.

### Risk Score
- **Critical Issues:** 1 (CVE)
- **High Issues:** 1 (Port exposure)
- **Medium Issues:** 2 (Unmaintained deps)
- **Overall Rating:** UNACCEPTABLE FOR PRODUCTION

---

## Critical Findings

### 1. CRITICAL: rustls-webpki CRL Validation Bypass (RUSTSEC-2026-0049)

**Severity:** CRITICAL
**CVSS Score:** High (TLS certificate validation bypass)
**Status:** Currently Ignored in deny.toml (line 35)
**Discoverer:** RustSec Advisory Database
**Published:** 2026-03-20

#### Description
The rustls-webpki crate version 0.102.8 contains a critical vulnerability in CRL (Certificate Revocation List) handling where Distribution Point matching logic is faulty. This allows an attacker to bypass certificate revocation checks and use revoked certificates for TLS connections.

#### Evidence
```bash
$ cargo audit
Crate:     rustls-webpki
Version:   0.102.8
Title:     CRLs not considered authoritative by Distribution Point due to faulty matching logic
ID:        RUSTSEC-2026-0049
URL:       https://rustsec.org/advisories/RUSTSEC-2026-0049
Solution:  Upgrade to >=0.103.10
```

#### Impact Chain
```
rustls-webpki 0.102.8
└── rustls 0.22.4
    ├── tungstenite 0.21.0
    │   └── tokio-tungstenite 0.21.0
    │       └── serenity 0.12.5
    │           └── terraphim_tinyclaw 1.16.9 (affects Discord integration)
    ├── tokio-tungstenite 0.21.0 (WebSocket communication)
    └── tokio-rustls 0.25.0 (TLS layer)
```

#### Root Cause
The vulnerability is transitive via `serenity 0.12.5` which pins `hyper-rustls 0.24` → `rustls 0.21.x` → `rustls-webpki 0.102.x`.

Current workaround (deny.toml line 35): The CVE is currently **ignored**, relying on the fact that Discord integration is disabled in default tinyclaw features. **This is insufficient** because:
1. The dependency is still present in the dependency tree
2. If Discord integration is ever re-enabled, the vulnerability becomes active
3. The fix requires serenity 0.13+ which is not yet released

#### Remediation Path

**Short-term (BLOCKING):**
- [ ] Option A: Remove serenity/tinyclaw dependency entirely if Discord integration is not essential
- [ ] Option B: Fork serenity with rustls 0.23+ to unblock upgrade
- [ ] Option C: Switch to alternative Discord library without vulnerable rustls transitive

**Medium-term:**
- [ ] Monitor serenity releases for v0.13+ with rustls 0.23+ support
- [ ] Update to rustls-webpki >=0.103.10 once serenity is updated
- [ ] Remove RUSTSEC-2026-0049 from deny.toml ignore list

**Recommended:** Remove serenity/tinyclaw from default workspace unless Discord integration is actively used.

#### Timeline
- **Published:** 2026-03-20
- **Current Status:** Ignored since at least 2026-03-20
- **Days Exposed:** 18 days
- **Urgency:** IMMEDIATE ACTION REQUIRED

---

### 2. HIGH: Network-Exposed Service on Port 3456

**Severity:** HIGH
**Type:** Operational Security / Network Exposure
**Status:** Active and Listening

#### Description
A service identified as `terraphim-llm-p` (likely terraphim-llm-proxy based on git history) is listening on **0.0.0.0:3456**, making it accessible from all network interfaces, not just localhost.

#### Evidence
```bash
$ ss -tlnp
LISTEN 0 1024 0.0.0.0:3456 0.0.0.0:* users:(("terraphim-llm-p",pid=947,fd=9))
```

The service is bound to `0.0.0.0` (all interfaces), which means:
- ✗ Accessible from external networks (if firewall permits)
- ✗ Accessible from other containers/VMs on the same network
- ✓ Vulnerable to network reconnaissance and potential exploitation

#### Impact
- **Confidentiality:** High - LLM proxy may expose model requests/responses
- **Integrity:** High - Attacker could intercept and modify requests
- **Availability:** Medium - DDoS vector if externally exposed

#### Remediation
- [ ] Bind service to `127.0.0.1` (localhost only) for local development
- [ ] If remote access needed: Use VPN, SSH tunnel, or secure reverse proxy
- [ ] Document port binding requirements in security guidelines
- [ ] Add network security tests to CI/CD

#### References
- Git commits mentioning port 3456: Several "E2E" and "terraphim-llm-proxy" commits
- Related: Firecracker VM isolation features suggest security is a concern

---

## High-Priority Findings

### 3. HIGH: Unmaintained Core Dependency - bincode (RUSTSEC-2025-0141)

**Severity:** HIGH
**Type:** Maintenance / Supply Chain Risk
**Status:** Ignored in deny.toml (line 24)

#### Description
The `bincode 1.3.3` crate is unmaintained and appears in the critical dependency chain for serialization:
- terraphim_automata (core search indexing)
- terraphim_service (main service logic)
- terraphim_persistence (data storage)
- terraphim_agent (agent framework)

#### Usage Context
According to deny.toml comments: "used by redb persistence backend"

#### Remediation
- [ ] Replace with maintained alternatives:
  - **Recommended:** `postcard` (minimal, maintained)
  - **Alternative:** `rkyv` (high-performance, well-maintained)
  - **Alternative:** `serde` with custom implementations
- [ ] Timeline: Must complete before production deployment
- [ ] Update deny.toml once bincode is removed

---

### 4. HIGH: Yanked Dependency Versions

**Severity:** MEDIUM
**Type:** Maintenance / Stability Risk

#### Affected Crates
1. **fastrand 2.4.0** - yanked version (should not be used)
2. **term_size 0.3.2** - yanked version

#### Remediation
- [ ] Update fastrand to stable version
- [ ] Replace term_size with maintained alternative (terminal_size)
- [ ] Run `cargo update` to pull latest compatible versions

---

## Verification Results

### Secret Scanning
- ✓ **PASS:** No hardcoded API keys found (sk_*, api_key patterns)
- ✓ **PASS:** No hardcoded secrets in Rust source files
- ✓ **PASS:** No .env files committed

### Unsafe Code Audit
- ✓ **PASS:** No unsafe blocks detected in crate/ directory
- **Note:** Firecracker integration may use unsafe code (separate binary)

### Port Security
- ✗ **FAIL:** Port 3456 listening on 0.0.0.0 (see Finding #2)
- ✓ **PASS:** Standard ports (80, 443, 22) properly configured
- ✓ **PASS:** Internal services (Redis 6379, PostgreSQL 5432) localhost-bound

### Dependency Vulnerabilities
- ✗ **FAIL:** 1 critical CVE (rustls-webpki RUSTSEC-2026-0049)
- ✗ **FAIL:** 1 unmaintained dep (bincode RUSTSEC-2025-0141)
- ✗ **FAIL:** 2 yanked versions active
- ✓ **PASS:** No other known vulnerabilities

### Dependency License Compliance
- ✓ **PASS:** All licenses in approved list (MIT, Apache-2.0, BSD, etc.)
- ✓ **PASS:** No GPL/AGPL conflicts detected

---

## Security Commits Analysis

Recent attempts to address RUSTSEC-2026-0049:
1. **81c81fe3** - "security: upgrade rustls-webpki to fix RUSTSEC-2026-0049 (CRL validation bypass)"
2. **5b784f88** - "security: Update dependencies to resolve RUSTSEC-2026-0049"
3. **b12afb9e** - "security: Remove RUSTSEC-2026-0049 ignore from deny.toml"
4. **034b7f4** - "fix(security): remove serenity 0.12 to eliminate RUSTSEC-2026-0049"
5. **b3473294** - "feat(security-sentinel): agent work [auto-commit]" (ongoing sentinel work)

**Status:** Multiple attempts suggest the issue is difficult to resolve without breaking serenity-dependent features.

---

## Compliance Assessment

### Production Readiness
- ✗ **BLOCKED** - Cannot deploy to production with critical CVE present
- ✗ **BLOCKED** - Must remediate port exposure before external network access

### Security Standards
- **OWASP:** Dependency management failures (A06:2021)
- **CWE-539:** Overly Broad Use of Privilege (port exposure)
- **NIST:** Software Supply Chain Security deficiencies

### Data Protection
- ✓ Database (PostgreSQL) localhost-bound
- ✓ Redis (cache) localhost-bound
- ✗ LLM proxy exposed on network

---

## Remediation Priority & Timeline

| Priority | Finding | Effort | Timeline | Blocker? |
|----------|---------|--------|----------|----------|
| **CRITICAL** | RUSTSEC-2026-0049 | Medium | **IMMEDIATE** | YES |
| **CRITICAL** | Port 3456 exposure | Low | **TODAY** | YES |
| **HIGH** | bincode unmaintained | High | **Before Production** | YES |
| **MEDIUM** | yanked versions | Low | **This Sprint** | NO |

### Blocking Criteria for Production Deployment
1. ✗ rustls-webpki ≥0.103.10 OR serenity removed
2. ✗ Port 3456 bound to 127.0.0.1 only
3. ✗ bincode replaced with maintained alternative
4. ✗ No yanked versions in Cargo.lock

---

## Recommendations

### Immediate Actions (Next 24 hours)
1. **Disable Discord integration** - Mark serenity/tinyclaw as optional feature, disable by default
2. **Fix port binding** - Change terraphim-llm-proxy to bind `127.0.0.1:3456`
3. **Escalate to team** - Brief team on critical CVE and remediation path

### Short-term (This Sprint)
1. **Evaluate serenity alternatives** - If Discord is needed, find maintained alternative or fork
2. **Migrate from bincode** - Implement postcard serialization for persistence layer
3. **Update yanked versions** - Run dependency audit and force stable versions
4. **Add security gates** - CI/CD should fail on critical/unmaintained dependencies

### Long-term (Q2 2026)
1. **Implement supply chain security**:
   - Regular dependency audits (weekly)
   - Automated CVE monitoring with alerts
   - Security test gates for all PRs
2. **Documentation**:
   - Create security.md with vulnerability disclosure process
   - Document network security requirements
   - Add security section to CLAUDE.md
3. **Monitoring**:
   - Add port binding verification to health checks
   - Continuous vulnerability scanning in CI/CD
   - Dependency update automation

---

## Conclusion

The terraphim-ai project has achieved strong security posture in code quality (no unsafe blocks, no hardcoded secrets), but has critical supply chain security issues that must be resolved before production deployment.

**Status: SECURITY AUDIT FAILED**

The project cannot be deployed to production or merged until:
1. rustls-webpki vulnerability is resolved (remove serenity or upgrade path found)
2. Port 3456 exposure is fixed (bind to localhost)
3. bincode dependency is replaced

Estimated remediation time: 2-3 days for critical issues, 1 sprint for all issues.

---

**Audit Conducted:** 2026-04-07
**Auditor:** Vigil, Security Engineer
**Signature:** Shield-lock ⬜
**Next Review:** Upon remediation completion
