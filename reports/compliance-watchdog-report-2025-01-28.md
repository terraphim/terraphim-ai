# Compliance Watchdog Report
**Date**: 2025-01-28
**Status**: FAIL
**Scope**: terraphim-ai project - License compliance, supply chain security, GDPR/data handling

---

## Executive Summary

Compliance violations detected across three categories:
- **3 License violations** (2 unlicensed internal crates, 1 unapproved dependency license)
- **0 Active security advisories** (5 historical advisories properly ignored)
- **8 GDPR/data handling gaps** (no retention policy, incomplete erasure/portability)

**Recommendation**: DO NOT MERGE until license issues resolved and GDPR gaps documented.

---

## 1. License Compliance (FAIL)

### Critical Findings

| Severity | Issue | Location | Evidence |
|----------|-------|----------|----------|
| HIGH | Unlicensed internal crate | `crates/terraphim_ccusage/Cargo.toml` | No `license` field declared |
| HIGH | Unlicensed internal crate | `crates/terraphim_usage/Cargo.toml` | No `license` field declared |
| MEDIUM | Unapproved license | `libbz2-rs-sys v0.2.2` | License "bzip2-1.0.6" not in allow-list |
| LOW | Deprecated SPDX | `html2md v0.2.15` | Uses deprecated "GPL-3.0+" identifier |

### Remediation Required

**terraphim_ccusage and terraphim_usage**:
```toml
# Add to both Cargo.toml files:
license = "MIT OR Apache-2.0"
```

**libbz2-rs-sys license**:
Either:
- Add "bzip2-1.0.6" to deny.toml allow-list (if license reviewed), OR
- Replace with alternative compression library

**html2md**:
```toml
# In deny.toml, add license clarification:
[[licenses.clarify]]
name = "html2md"
expression = "GPL-3.0-or-later"
license-files = [{ path = "LICENSE", hash = 0x... }]
```

---

## 2. Supply Chain Security (PASS)

### Advisory Status

| Check | Result | Details |
|-------|--------|---------|
| cargo-deny advisories | PASS | No active vulnerabilities detected |
| Yanked crates | WARN | None currently yanked |
| Unmaintained deps | MONITORING | 6 unmaintained crates properly tracked in deny.toml |

### Unmaintained Dependencies (Documented)

All properly tracked with TODOs in deny.toml:
- `RUSTSEC-2023-0071` - RSA Marvin Attack (awaiting upstream fix)
- `RUSTSEC-2021-0145` - atty unaligned read (Windows only)
- `RUSTSEC-2024-0375` - atty unmaintained (migration planned)
- `RUSTSEC-2025-0141` - bincode unmaintained (redb backend)
- `RUSTSEC-2021-0141` - dotenv unmaintained (replace with dotenvy)
- `RUSTSEC-2020-0163` - term_size unmaintained (replace with terminal_size)

### External Git Dependencies

Properly allow-listed in deny.toml:
- `https://github.com/terraphim/rust-genai.git`
- `https://github.com/AlexMikhalev/self_update.git`

---

## 3. GDPR/Data Handling Audit (PARTIAL COMPLIANCE)

### Critical Gaps (High Risk)

| GDPR Article | Requirement | Status | Evidence |
|--------------|-------------|--------|----------|
| Art. 5(1)(e) | Storage limitation | FAIL | No retention policy; data stored indefinitely |
| Art. 17 | Right to erasure | PARTIAL | Delete endpoints exist but no automated complete purge |
| Art. 20 | Data portability | PARTIAL | Export exists for conversations only |
| Art. 25 | Privacy by design | FAIL | No anonymization/pseudonymization |
| Art. 32 | Security of processing | PARTIAL | API tokens in plaintext; no encryption at rest |

### Data Collection Inventory

**Sessions Crate** (`terraphim_sessions`):
- **Collected**: Full message content, file paths, project paths, timestamps, tool inputs
- **Storage**: Imported from `~/.claude/projects/`, stored in-memory
- **Retention**: Indefinite (no TTL)
- **Risk**: HIGH - Complete conversation history retained

**Persistence Crate** (`terraphim_persistence`):
- **Backends**: Dashmap, Redb, SQLite, S3, Redis, Memory
- **Data**: Conversations, documents, settings
- **Keys**: `conversations/{id}.json`, `document_{id}.json`
- **Retention**: Indefinite (no automatic purging)
- **Risk**: HIGH - No data lifecycle management

**Service Crate** (`terraphim_service`):
- **APIs**: Full CRUD on conversations, chat completion, search
- **Logging**: User prompts logged (secret redaction implemented)
- **External**: Data sent to LLM providers
- **Risk**: MEDIUM - External data sharing

### Positive Privacy Controls

1. **Secret Redaction** - Implemented in `terraphim_agent/src/learnings/redaction.rs`
   - Covers: AWS keys, OpenAI keys, Slack tokens, GitHub tokens
   - Gaps: Azure principals, GCP keys, JWT tokens, PEM keys

2. **Offline-First Architecture** - Local processing capability
3. **Export Functionality** - `export_conversation` endpoint (GDPR Art. 20 partial)
4. **Delete Functionality** - `delete_conversation` endpoint (GDPR Art. 17 partial)

### Recommendations

**Immediate (Block Release)**:
1. Add license declarations to terraphim_ccusage and terraphim_usage
2. Document GDPR compliance gaps in README
3. Add data retention policy configuration

**Short-term (Next Sprint)**:
1. Implement automatic data purging with configurable TTL
2. Add comprehensive user data export across all systems
3. Encrypt API tokens at rest (keyring integration)
4. Expand secret redaction coverage

**Long-term (Quarterly)**:
1. Implement anonymization for analytics
2. Add consent management for data collection
3. Privacy impact assessment for LLM data sharing

---

## Compliance Verdict

**OVERALL: FAIL**

- License Compliance: FAIL (3 violations)
- Supply Chain: PASS (monitored)
- GDPR/Data Handling: PARTIAL (8 gaps)

### Blockers for Merge

1. Unlicensed internal crates (terraphim_ccusage, terraphim_usage)
2. No documented data retention policy
3. API tokens stored in plaintext

### Risk Assessment

| Risk Category | Level | Justification |
|---------------|-------|---------------|
| License | MEDIUM | Internal crates easy to fix; external dep needs review |
| Supply Chain | LOW | No active vulnerabilities; unmaintained deps tracked |
| Data Protection | HIGH | Indefinite data retention, plaintext secrets |

---

## Evidence

### Commands Executed
```bash
source ~/.profile
cd /home/alex/terraphim-ai
cargo deny check licenses
cargo deny check advisories
grep -r "license" crates/*/Cargo.toml
```

### Files Examined
- `/home/alex/terraphim-ai/deny.toml` - cargo-deny configuration
- `/home/alex/terraphim-ai/crates/terraphim_ccusage/Cargo.toml` - Unlicensed
- `/home/alex/terraphim-ai/crates/terraphim_usage/Cargo.toml` - Unlicensed
- `/home/alex/terraphim-ai/crates/terraphim_sessions/src/model.rs` - Session data structures
- `/home/alex/terraphim-ai/crates/terraphim_persistence/src/conversation.rs` - Persistence layer
- `/home/alex/terraphim-ai/crates/terraphim_service/src/conversation_service.rs` - Service layer

---

## Next Steps

1. **License fixes** (est. 30 min): Add license fields to Cargo.toml files
2. **GDPR documentation** (est. 2 hours): Create compliance roadmap
3. **Review bzip2 license** (est. 1 hour): Determine if acceptable for project

---

*Report generated by Vigil, Security Engineer*
*Shield-lock: Trust but verify. Verify but trust only after proof.*
