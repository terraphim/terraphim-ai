# ADR-0001: Ollama Service Trust Boundary

**Status**: Accepted  
**Date**: 2026-05-07  
**Deciders**: Security (Vigil), Architecture  
**Refs**: #1313, #1318, #1317  
**GDPR Basis**: Article 32 — Technical and organisational measures  

---

## Context

Terraphim AI embeds Ollama as a local LLM inference engine. Ollama currently binds its HTTP API on all network interfaces (`*:11434` / `0.0.0.0:11434`) with no authentication layer. This creates an unauthenticated, network-accessible LLM endpoint.

### Threat Model

| Threat | Likelihood | Impact |
|--------|------------|--------|
| Adjacent-network host queries Ollama without authorisation | High (LAN/VPN) | High — arbitrary model inference, data exfiltration via prompt |
| Ollama used as relay to exfiltrate documents indexed by Terraphim | Medium | Critical — contains personal/proprietary knowledge graph data |
| Container escape or compromised container pivots to Ollama | Low | High — full model access |
| Ollama SSRF via crafted model pull URL | Medium | High — internal network reconnaissance |

### GDPR Article 32 Relevance

Terraphim processes documents that may contain personal data (emails via JMAP haystack, Confluence pages, personal knowledge bases). Ollama receives this content as prompt context. An unauthenticated binding means any process or host on the network segment can submit arbitrary prompts containing this data to the LLM endpoint, constituting unauthorised processing under GDPR Article 4(2).

---

## Decision Drivers

1. Ollama has no built-in authentication mechanism as of version 0.x
2. Terraphim runs as a local-first privacy-preserving assistant — wide network exposure contradicts the product's threat model
3. GDPR Article 32 requires appropriate technical measures proportionate to the risk
4. Development environments must remain ergonomic (no complex auth setup for local use)
5. A formal architecture decision is required as evidence artefact for the Article 32 compliance audit

---

## Considered Options

### Option A: Bind Ollama to localhost only (127.0.0.1:11434)

Restrict the Ollama bind address to loopback. Only processes on the same host may reach the API.

**Pros**:
- Eliminates network exposure entirely
- Zero operational complexity — no credentials to manage
- Compatible with all current Terraphim integration code (same port, same protocol)
- Directly satisfies GDPR Article 32

**Cons**:
- Prevents multi-host setups where Ollama runs on a dedicated GPU server

### Option B: Restrict via firewall rules

Keep `0.0.0.0:11434` binding; add `iptables`/`nftables` rules to restrict access to localhost only.

**Pros**: Supports future multi-host topology without code change

**Cons**:
- Firewall rules are fragile, host-specific, not version-controlled
- Rule removal or flush leaves Ollama exposed with no application-level defence
- Does not satisfy defence-in-depth principle

### Option C: Proxy with authentication (nginx/caddy in front of Ollama)

Introduce an authenticating reverse proxy on port 11434; Ollama binds loopback only.

**Pros**: Supports multi-host; provides audit log; authentication layer

**Cons**:
- Significant operational complexity for a local-first tool
- Adds a dependency and failure mode
- Overkill for the current single-host deployment model

### Option D: Accept risk with compensating controls

Document the risk, bind remains wide, require network segmentation as compensating control.

**Pros**: Zero development effort

**Cons**:
- Not acceptable under GDPR Article 32 — risk is disproportionate to the (absent) compensating measure
- Contradicts Terraphim's privacy-first positioning
- No evidence artefact of active control

---

## Decision

**Chosen option: Option A — Bind Ollama to localhost (127.0.0.1:11434)**

### Rationale

Terraphim is a local-first, privacy-preserving system. The single-host deployment model is the primary and current production topology. Binding to localhost eliminates the attack surface with zero operational overhead and directly satisfies Article 32.

Multi-host GPU setups are a future concern. When that topology is required, it should be addressed via a follow-on ADR (Option C) with a proper authentication mechanism. Accepting wide network exposure now to pre-empt a future requirement violates the principle of proportionality.

---

## Consequences

### Immediate Actions Required

| Action | Owner | Target | Tracking |
|--------|-------|--------|----------|
| Rebind Ollama on live host to `127.0.0.1:11434` | Ops/Platform | 2026-05-08 | #1313 |
| Update `docker-compose.yml` Ollama port mapping to `127.0.0.1:11434:11434` | Developer | Next PR | #1313 |
| Update `docker/docker-compose.yml` and any CI fixtures | Developer | Next PR | #1313 |
| Verify Terraphim config points to `http://127.0.0.1:11434` (already correct) | Developer | Next PR | #1313 |

### Positive Consequences

- Ollama API is no longer reachable from adjacent network hosts
- Personal data submitted as LLM context is processed only by local processes
- GDPR Article 32 technical measure is documented and enforceable
- This ADR serves as the evidence artefact required by the Article 32 audit (#1318)

### Negative Consequences / Accepted Limitations

- Multi-host Ollama setups require a follow-on ADR and additional configuration
- Users running Ollama on a remote GPU server must configure an SSH tunnel or VPN — this is a known limitation and is acceptable for v1 deployments

### Follow-on Decisions Required

- ADR-0002: Ollama authentication for multi-host deployments (deferred, tracked in #1318 comments)
- ADR-0003: Artefact integrity for `deserialize_unchecked` in sharded extractor (#1322)

---

## Implementation Notes

### Docker Compose

```yaml
# Before (INSECURE — exposes to all interfaces)
ollama:
  ports:
    - "11434:11434"

# After (SECURE — localhost only)
ollama:
  ports:
    - "127.0.0.1:11434:11434"
```

### Host-level (systemd / direct)

```bash
# /etc/systemd/system/ollama.service (or override)
[Service]
Environment="OLLAMA_HOST=127.0.0.1:11434"
```

### Verification

```bash
# Verify binding after change
ss -tlnp | grep 11434
# Expected: 127.0.0.1:11434 (NOT 0.0.0.0:11434)

# Confirm remote access is blocked (from another host)
curl -m 3 http://<host-ip>:11434/api/tags
# Expected: connection refused / timeout
```

---

## Compliance Mapping

| Requirement | Control | Evidence |
|-------------|---------|----------|
| GDPR Art. 32(1)(b) — confidentiality | Localhost binding prevents network access | This ADR + host binding verification |
| GDPR Art. 32(1)(a) — pseudonymisation/encryption | Not applicable (loopback traffic) | N/A |
| GDPR Art. 32(2) — proportionality | Risk assessed; control proportionate | Threat model above |
| Terraphim privacy-first principle | Local inference only; no data leaves host via Ollama | Architecture constraint |

---

*This ADR was created by Vigil (Security Engineer) in response to the compliance gate blocker documented in #1318 and the security findings in #1313.*
