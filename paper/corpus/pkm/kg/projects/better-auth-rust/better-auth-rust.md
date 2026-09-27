# better-auth-rust

Rust authentication library with AI agent identity, SPIFFE, x402 payments, and enterprise SSO. Evolution planning repo inspired by better-auth (TypeScript) but built idiomatically for Rust.

## Key Facts

- **Language:** Rust (100%)
- **GitHub:** https://github.com/terraphim/better-auth-rust
- **Gitea:** https://git.terraphim.cloud/terraphim/better-auth-rust
- **Gitea Description:** Rust auth library with AI agent identity, SPIFFE, x402 payments, enterprise SSO. Evolution planning repo.
- **Created:** 2026-03-26
- **Default Branch:** main
- **ZDP Stage:** Discovery (PVVH, Wardley Map, Business Scenarios complete)
- **Docs:** https://github.com/terraphim/better-auth-rust/tree/main/docs

## Architecture

- Unified auth API: email/password, OAuth2/social login, sessions, API keys, 2FA/TOTP, multi-tenant organisations
- AI agent identity: Agent entities with API keys, capabilities, token vault
- Plugin architecture for extensible auth components
- Database abstraction via sqlx (SQLite/PostgreSQL)
- Built on tower-sessions, oauth2 crate
- Type-safe throughout, zero-cost abstractions

## Target Users

- Rust web developers (Axum/Actix)
- Full-stack Rust developers
- SaaS builders needing multi-tenancy
- API service developers needing API key auth
- AI agent developers needing machine-to-machine auth

## Key Differentiators

- First comprehensive "batteries-included" auth library for Rust
- AI agent identity as a first-class entity (not bolted on)
- SPIFFE/SPIRE integration for workload identity
- x402 payment protocol integration
- Enterprise SSO support
- Type-safe auth flows catching errors at compile time

## ZDP Artefacts (in docs/)

- PVVH (Product Vision and Value Hypothesis)
- Domain Model (User, Agent, ApiKey, Session, Organization, etc.)
- Wardley Map (Rust Auth Landscape)
- Business Scenarios (developer setup, M2M auth, SSO, 2FA, etc.)
- Parity Roadmap (5 phases: PostgreSQL, Passkeys, Enterprise SSO, CLI, Adapters)
- Risk Scan, LCA Gate Status
- Right-side-of-V Evaluation Reports (v1, v2, v3)
- Gap Remediation Research and Design
- Budget and Timeline
