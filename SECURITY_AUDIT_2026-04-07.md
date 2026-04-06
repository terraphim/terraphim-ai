# Security Audit Report: terraphim-ai
**Date**: 2026-04-07
**Auditor**: Vigil (Security Engineer)
**Status**: FAIL - Critical vulnerabilities detected

---

## Executive Summary

This security audit identified **1 CRITICAL vulnerability** and **6 warnings** in dependency management, along with **1 HIGH severity network exposure** issue. The project cannot be cleared for production deployment until critical vulnerabilities are remediated.

**Verdict**: **FAIL** - Critical CVE requires immediate patching

---

## Critical Findings

### 1. RUSTSEC-2026-0049: CRL Validation Bypass in rustls-webpki (CRITICAL)

**Severity**: CRITICAL
**Affected Package**: `rustls-webpki 0.102.8`
**Impact**: Certificate Revocation List (CRL) validation bypass - allows revoked certificates to be accepted as valid
**CVE ID**: RUSTSEC-2026-0049
**Date Discovered**: 2026-03-20

**Description**:
The rustls-webpki library has a faulty CRL matching logic that does not properly consider CRLs as authoritative per the Distribution Point specification. This allows attackers to bypass certificate revocation checks by presenting revoked certificates.

**Affected Dependency Chain**:
```
rustls-webpki 0.102.8
└── rustls 0.22.4
    ├── tungstenite 0.21.0
    │   └── tokio-tungstenite 0.21.0
    │       └── serenity 0.12.5
    │           └── terraphim_tinyclaw 1.16.9
    └── tokio-tungstenite 0.21.0
```

**Remediation**: Upgrade to rustls-webpki ≥0.103.10
**Action**: Update Cargo.lock immediately - this is not optional

**Security Impact**:
- Any HTTPS connections brokered through this dependency may accept revoked certificates
- In the context of terraphim-ai's multi-LLM provider routing, revoked API endpoint certificates could be accepted
- Affects OAuth flows and API key management through untrusted endpoints

**Proof**:
```bash
$ cargo audit
Crate:     rustls-webpki
Version:   0.102.8
Title:     CRLs not considered authoritative by Distribution Point due to faulty matching logic
ID:        RUSTSEC-2026-0049
Solution:  Upgrade to >=0.103.10
```

---

## High Severity Findings

### 2. Network Exposure: LLM Proxy on Port 3456 (HIGH)

**Severity**: HIGH
**Affected Service**: `terraphim-llm-proxy` listening on `0.0.0.0:3456`
**Risk**: Unauthenticated access to LLM provider API routing

**Analysis**:
The terraphim-llm-proxy is exposed to all network interfaces (0.0.0.0:3456) with the following configuration issues:

```
[proxy]
host = "0.0.0.0"
port = 3456
api_key = "$PROXY_API_KEY"
rate_limiting = false
```

**Problems Identified**:
1. **Disabled Rate Limiting**: No protection against brute force or DoS attacks
   ```
   [security.rate_limiting]
   enabled = false
   requests_per_minute = 600
   concurrent_requests = 100
   ```

2. **Public Network Binding**: Accessible from any network interface
   - Requires network-level access control (firewall rules)
   - Assumes all incoming connections will provide valid API key
   - No documentation of authentication mechanism

3. **Permissive SSRF Protection**:
   ```
   [security.ssrf_protection]
   enabled = true
   allow_localhost = true
   allow_private_ips = true
   ```
   - Allowing private IPs in SSRF protection could enable attacks against internal services
   - localhost access in a proxy is dangerous for credential theft scenarios

4. **Multiple Unvalidated Provider APIs**:
   - Handles API keys for: OpenAI, Z.ai, MiniMax, Cerebras, Kimi, Ollama
   - Credentials stored in environment variables (less critical but worth auditing)

**Remediation**:
1. **Immediate**: Enable rate limiting with conservative defaults (100 req/min for unknown clients)
   ```toml
   [security.rate_limiting]
   enabled = true
   requests_per_minute = 100
   concurrent_requests = 10
   ```

2. **Network Isolation**: Bind to localhost or specific internal IP only
   ```toml
   [proxy]
   host = "127.0.0.1"  # or specific internal IP
   port = 3456
   ```

3. **SSRF Hardening**: Restrict to required destinations
   ```toml
   [security.ssrf_protection]
   enabled = true
   allow_localhost = false
   allow_private_ips = false
   ```

4. **Implement API Gateway**: Add authentication layer (API Gateway, WAF) in front of proxy

---

## Medium Severity Findings

### 3. Unmaintained Dependencies (6 warnings)

**Severity**: MEDIUM
**Count**: 6 unmaintained dependencies with warnings

**Affected Packages**:
1. **bincode 1.3.3** - RUSTSEC-2025-0141 (Unmaintained)
   - Widely used in terraphim_automata, terraphim_service, and agent crates
   - **Risk**: No security patches for discovered vulnerabilities
   - **Recommendation**: Evaluate migration to `serde_json` or `postcard`

2. **rustls-pemfile 1.0.4** - RUSTSEC-2025-0134 (Unmaintained)
   - Used in hyper-rustls → reqwest chain
   - **Risk**: PEM parsing vulnerabilities won't be fixed
   - **Recommendation**: Monitor for fork maintenance or replacement

3. **term_size 0.3.2** - RUSTSEC-2020-0163 (Unmaintained)
   - Used in terraphim_validation
   - **Risk**: Terminal size detection exploits
   - **Recommendation**: Replace with `terminal_size` crate (maintained fork)

**Action Required**:
- Assess impact of each unmaintained dependency
- Plan migration timeline (3-6 months)
- Document why each library was chosen over alternatives

---

## Good Findings

### ✓ Secret Detection Results
- No hardcoded API keys found in source code
- No AWS credentials or tokens detected
- No hardcoded database passwords
- Configuration properly externalizes secrets via environment variables

### ✓ Code Safety
- No obvious unsafe blocks in critical paths
- Proper use of Rust's type system for bounds checking

### ✓ Recent Commit Review (Last 24 hours)
- No security-related regression commits identified
- Release and CI/CD updates are properly tracked
- No credential leaks in commit history

---

## Network Exposure Assessment

**Overall Score**: 2/5 (Fair)

| Port | Service | Binding | Risk | Recommendation |
|------|---------|---------|------|-----------------|
| 22 | SSH | 0.0.0.0:22 | Medium | Expected for infrastructure |
| 80 | HTTP | 0.0.0.0:80 | Medium | Likely reverse proxy, monitor for redirects |
| 443 | HTTPS | 0.0.0.0:443 | Low | Standard HTTPS, acceptable |
| 3456 | LLM Proxy | 0.0.0.0:3456 | **HIGH** | **Enable auth, rate limiting, rebind to localhost** |
| 11434 | Ollama | 0.0.0.0:11434 | Medium | Local LLM service, should be localhost only |
| 6379 | Redis | 127.0.0.1:6379 | Low | Properly bound to localhost ✓ |
| 5432 | PostgreSQL | 127.0.0.1:5432 | Low | Properly bound to localhost ✓ |

---

## Verification Results

| Check | Result | Evidence |
|-------|--------|----------|
| CVE Scan | FAIL | 1 critical CVE in rustls-webpki 0.102.8 |
| Secret Detection | PASS | No hardcoded secrets found |
| Unsafe Blocks | PASS | No critical unsafe code identified |
| Network Exposure | FAIL | Port 3456 exposed without rate limiting |
| Recent Commits | PASS | No security regressions in last 24h |
| Dependency Maintenance | WARNING | 6 unmaintained dependencies |

---

## Remediation Roadmap

### Phase 1: CRITICAL (Immediate - Do Not Deploy)
- [ ] Upgrade rustls-webpki to ≥0.103.10
- [ ] Rebuild and test: `cargo build --release && cargo test --all`
- [ ] Verify no breaking changes from upgrade
- [ ] Update Cargo.lock and commit

### Phase 2: HIGH (This Week)
- [ ] Enable rate limiting on terraphim-llm-proxy
- [ ] Rebind LLM proxy to localhost or specific internal IP
- [ ] Harden SSRF protection settings
- [ ] Add firewall rules to restrict port 3456 access
- [ ] Document network security boundaries

### Phase 3: MEDIUM (This Month)
- [ ] Evaluate and plan migration from unmaintained dependencies:
  - bincode → postcard (or serde_json)
  - rustls-pemfile → (monitor for maintained fork)
  - term_size → terminal_size
- [ ] Create migration tickets for each dependency
- [ ] Plan rollout to minimize breaking changes

### Phase 4: ONGOING
- [ ] Set up automated security scanning in CI/CD
- [ ] Monthly dependency audit reviews
- [ ] Subscribe to security advisory feeds
- [ ] Implement SCA (Software Composition Analysis) in pipeline

---

## Testing Procedure for Verification

After applying remediations, run:

```bash
# 1. Verify dependency upgrades
cargo audit --deny warnings

# 2. Full test suite
cargo test --all --all-features --release

# 3. Check for new vulnerabilities
cargo update && cargo audit

# 4. Network verification (requires deployment)
netstat -tlnp | grep -E "3456|11434" # Verify proper binding
curl -H "Authorization: Bearer $PROXY_API_KEY" http://localhost:3456/health

# 5. Rate limiting verification
ab -n 1000 -c 100 http://localhost:3456/api/models # Should hit limits
```

---

## Compliance Implications

**Affected Policies**:
- OWASP Top 10: A06:2021 - Vulnerable and Outdated Components
- CWE-1035: Known Vulnerable Component
- CWE-327: Use of Broken or Risky Cryptographic Algorithm (certificate validation)

**Deployment Status**: Cannot proceed to production until CRITICAL remediation is complete.

---

## Appendix: Detailed Findings

### RUSTSEC-2026-0049 Technical Details

The vulnerability exists in the CRL validation logic in `rustls-webpki`. The issue is that when checking if a CRL is authoritative for a given certificate, the library does not properly match Distribution Points. This allows a CRL that is not the authority for a certificate to be accepted, enabling revoked certificates to be used.

**Impact on terraphim-ai**:
- LLM provider endpoints (OpenAI, Z.ai, MiniMax, etc.) are accessed via HTTPS
- If a provider's certificate becomes revoked (due to compromise, domain transfer, etc.), this vulnerability would allow connections to the revoked endpoint
- An attacker could intercept traffic to a revoked endpoint without detection

**Likelihood**: LOW (requires certificate revocation event + attacker positioning)
**Impact if Exploited**: CRITICAL (complete compromise of LLM API credentials)

---

## Sign-off

**Auditor**: Vigil (Security Engineer)
**Audit Date**: 2026-04-07
**Report Version**: 1.0

**Verdict**: **FAIL** - Cannot proceed to production

This audit recommends blocking any deployment or release until critical vulnerabilities are resolved.

---

## Future Security Work

- Implement Software Bill of Materials (SBOM) generation
- Add security scanning to GitHub Actions CI/CD
- Establish security incident response procedures
- Conduct architectural security review of LLM routing logic
- Penetration test network exposure after remediation
