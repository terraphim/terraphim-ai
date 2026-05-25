# Phase 3+ Plan: Monetisation, Auth, and Premium Skills

## Context

Phase 1–2 (ADF agent validation) is complete: all 37 agents probed, 32/37 runnable, all terraphim-ai ADF agents fully validated across all modes. The orchestrator runs live at PID 579991.

Four greenfield workstreams remain:
1. **Stripe integration** — subscription billing ($20 Individual / $300 + $100/seat Enterprise)
2. **better-auth-rust backend** — Gitea OAuth, API keys, agent identity
3. **Custom domain routing** — `terraphim-skills.md` marketplace
4. **Premium skill definitions** — tiered skill marketplace

**Constraint:** MSRV 1.80, subscription-only models (C1 invariant), disciplined phases (research → design → implementation → verification → validation).

---

## Workstream 1 — Stripe Integration

### 1.1 Research

**Owner:** research agent
**Artifacts:** `.docs/research-stripe-integration.md`

**Questions to answer:**
- How does Terraphim currently identify users? (Existing user model? Gitea OAuth already in use?)
- What events trigger billing? (Agent execution? API calls? Seat provisioning?)
- Enterprise tier: $300 base + $100/seat — how is "seat" defined and counted?
- Stripe webhook idempotency — how to handle duplicate events?
- Free tier: what limits apply, how enforced?
- Existing `terraphim_usage` crate tracks internal AI costs — does this map to user billing?

**Existing code to study:**
- `crates/terraphim_usage/src/pricing.rs` — `PricingTable` (internal AI cost model)
- `crates/terraphim_usage/src/store.rs` — `AgentMetricsRecord`, `BudgetSnapshotRecord`
- `crates/terraphim_onepassword_cli/src/lib.rs` — `SecretLoader` trait (Stripe keys via `op://`)
- `crates/terraphim_service/` — main server; study existing user/session model
- `crates/terraphim_settings/` — twelf config layer; secrets injection

### 1.2 Design

**Owner:** architecture agent
**Artifacts:** `.docs/design-stripe-integration.md`

**Decisions to make:**
- Stripe product/pricing IDs for Individual ($20/mo) and Enterprise ($300/mo + $100/seat)
- Webhook endpoint path and event types to handle (`customer.subscription.*`, `invoice.*`, `checkout.session.*`)
- Database schema for: `users`, `subscriptions`, `seats`, `api_keys`, `usage_records`
- How "seat" counting works (real-time vs. monthly snapshot)
- Free tier limits and enforcement layer (rate limiting? feature flags?)
- Stripe customer → local user mapping
- Refund/cancellation flow
- Test environment (Stripe test mode vs. live)

### 1.3 Implementation

**New crate:** `crates/terraphim_billing/`
**Module structure:**
```
terraphim_billing/
├── Cargo.toml
└── src/
    ├── lib.rs           # public exports
    ├── stripe_client.rs # Stripe API calls (subscribe, cancel, update seats)
    ├── webhook.rs       # Stripe webhook handler (axum endpoint)
    ├── models.rs        # Subscription, Seat, Invoice, UsageRecord
    ├── enforcement.rs   # Feature gates / rate limits per tier
    └── db.rs            # PostgreSQL schema + migrations (sqlx)
```

**Integrations:**
- `terraphim_service/src/` — mount webhook endpoint, inject subscription context into request
- `terraphim_settings/` — `op://` references for `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`
- `terraphim_usage/` — may extend existing usage tracking for per-user billing

**Key impl steps (TDD):**
1. `StripeClient::create_checkout_session()` test
2. `StripeClient::cancel_subscription()` test
3. `WebhookHandler::handle_event()` — route `customer.subscription.*` events
4. `SeatManager::count_seats()` / `update_seats()`
5. `TierEnforcer::check_tier()` feature gate tests
6. Integration test: full checkout → webhook → DB update flow

### 1.4 Verification

- `cargo test -p terraphim_billing`
- `cargo clippy -p terraphim_billing`
- Stripe test mode: trigger full checkout flow end-to-end
- Webhook idempotency: send duplicate event, verify no double-charge

---

## Workstream 2 — better-auth-rust Backend

### 2.1 Research

**Owner:** research agent
**Artifacts:** `.docs/research-better-auth-rust.md`

**Questions:**
- Is `better-auth` an existing Rust crate? What does it provide vs. `axum-auth` or `jsonwebtoken`?
- Gitea OAuth: existing implementation? OAuth1 vs. OAuth2? Token refresh flow?
- API key model: per-user static keys? Per-agent ephemeral keys? Key rotation?
- Agent identity: how is an agent authenticated to the orchestrator? Certificate? Token?
- Relationship to Stripe: does auth happen before or after payment?

**Existing code to study:**
- `crates/terraphim_onepassword_cli/src/lib.rs` — `SecretLoader` trait
- `crates/terraphim_service/src/` — existing auth middleware?
- `crates/terraphim_settings/src/` — twelf layer
- `.opencode/skills/` — any auth-related skills

### 2.2 Design

**Owner:** architecture agent
**Artifacts:** `.docs/design-better-auth-rust.md`

**Decisions:**
- Auth backend architecture: pure Rust library vs. external auth service
- Gitea OAuth2 flow: authorisation code grant, PKCE, token storage
- API key format and storage: hashed in DB? AES-encrypted at rest?
- Agent-to-orchestrator auth: service account tokens, mTLS, or shared secret
- Integration with existing `SecretLoader` pattern from `terraphim_onepassword_cli`
- Session management: JWT access tokens + refresh tokens, expiry policy
- Password recovery / Gitea OAuth only

### 2.3 Implementation

**New crate:** `crates/terraphim_auth/`
**Module structure:**
```
terraphim_auth/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── gitea_oauth.rs    # OAuth2 authorisation code flow with Gitea
    ├── api_keys.rs       # API key generation, hashing, validation
    ├── agent_identity.rs # Agent credentials (service accounts)
    ├── middleware.rs     # axum middleware (authenticate_request)
    ├── jwt.rs            # JWT minting + validation
    └── db.rs             # Schema: users, api_keys, sessions
```

**Integrations:**
- `crates/terraphim_service/src/` — mount auth routes (`/auth/login`, `/auth/apikey`, `/auth/logout`)
- `crates/terraphim_onepassword_cli/` — reuse `SecretLoader` for OAuth client secret
- `crates/terraphim_billing/` — auth must precede billing (user identity from auth)

**Key impl steps (TDD):**
1. `GiteaOAuthProvider::authorisation_url()` + PKCE test
2. `GiteaOAuthProvider::exchange_code()` test
3. `ApiKeyManager::generate()` / `validate()` tests
4. `AgentCredentials::mint_service_token()` test
5. `AuthMiddleware::layer()` integration with axum Router
6. `JwtManager::issue_access_token()` / `refresh()` tests

### 2.4 Verification

- `cargo test -p terraphim_auth`
- `cargo clippy -p terraphim_auth`
- E2E: OAuth login flow with Gitea test instance
- E2E: API key creation → usage → revocation

---

## Workstream 3 — Custom Domain terraphim-skills.md

### 3.1 Research

**Owner:** research agent
**Artifacts:** `.docs/research-custom-domain-skills.md`

**Questions:**
- What is the "custom domain" exactly? `skills.terraphim.ai`? Subdomain per user/org?
- How does `gitea_skill_loader.rs` currently work? Can it be extended for domain routing?
- Is the skill store multi-tenant? Can organisations have private skill forks?
- CDN / Cloudflare integration already exists (`scripts/add-custom-domains.sh`) — does this help?

**Existing code to study:**
- `crates/terraphim_orchestrator/src/gitea_skill_loader.rs` — existing remote skill loader
- `crates/terraphim_orchestrator/src/control_plane/routing.rs` — routing engine
- `scripts/add-custom-domains.sh` — Cloudflare Pages DNS
- `data/kg/meridian/product-strategy.md` — skill marketplace notes

### 3.2 Design

**Owner:** architecture agent
**Artifacts:** `.docs/design-custom-domain-skills.md`

**Decisions:**
- Domain model: `https://{org}.terraphim.ai/skills/{skill}` vs. `https://skills.terraphim.ai/{org}/{skill}`
- Skill manifest: `SKILL.md` already exists — what additional metadata needed? (`domain`, `pricing_tier`, `org`, `version`)
- Custom domain TLS: cert management (Let's Encrypt? Cloudflare origin cert?)
- Skill versioning: immutable tags? Semantic versioning?
- Private skills: authentication required to fetch? Org-gated?

### 3.3 Implementation

**Extensions to existing crates:**
- `crates/terraphim_orchestrator/src/gitea_skill_loader.rs` — add domain-aware loader
- `crates/terraphim_orchestrator/src/control_plane/routing.rs` — domain routing layer

**New module:** `crates/terraphim_skill_store/`
```
terraphim_skill_store/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── manifest.rs   # SkillManifest: domain, pricing_tier, org, version
    ├── domain.rs     # Custom domain resolution (nginx/Caddy config)
    ├── registry.rs   # SkillRegistry: publish, unpublish, list, search
    └── cloudflare.rs # DNS/SSL automation via Cloudflare API
```

**Key impl steps (TDD):**
1. `SkillManifest::parse()` from `SKILL.md` frontmatter test
2. `DomainResolver::resolve(skill_domain)` test
3. `SkillRegistry::publish()` / `unpublish()` tests
4. `CloudflareProvider::add_domain()` integration test
5. `gitea_skill_loader.rs` — extend with domain-header injection

### 3.4 Verification

- `cargo test -p terraphim_skill_store`
- `cargo clippy -p terraphim_skill_store`
- Custom subdomain resolves correctly (e.g., `acme.skills.terraphim.ai`)
- Private skill requires auth header

---

## Workstream 4 — Premium Skill Definitions

### 4.1 Research

**Owner:** research agent
**Artifacts:** `.docs/research-premium-skills.md`

**Questions:**
- What makes a skill "premium"? Higher LLM cost? Special capabilities? SLA?
- How do premium skills integrate with Stripe billing? Per-execution? Monthly subscription?
- Existing skill chain mechanism (`AgentDefinition.skill_chain`) — can it gate skills by tier?
- Skill marketplace: discovery, rating, versioning — build or integrate?

**Existing code to study:**
- `skills/*/skill.md` — 5 existing skills (smart-commit, quickwit-search, learning-capture, pre-llm-validate, post-llm-check)
- `crates/terraphim_orchestrator/src/config.rs` — `AgentDefinition.skill_chain` field
- `crates/terraphim_usage/src/pricing.rs` — `PricingTable` (could extend with skill pricing)

### 4.2 Design

**Owner:** architecture agent
**Artifacts:** `.docs/design-premium-skills.md`

**Decisions:**
- Skill tiers: Free, Pro ($20), Enterprise ($300+). What capabilities per tier?
- Skill manifest metadata: `tier`, `credits_per_execution`, `monthly_fee`
- Skill gating: `TierEnforcer::check_tier()` from billing workstream applied to skill chain
- Premium skill examples: which of the 5 existing skills become premium?
- Skill bundle: can multiple premium skills be purchased together?

### 4.3 Implementation

**Extensions:**
- `crates/terraphim_billing/src/enforcement.rs` — extend with `check_skill_tier()`
- `crates/terraphim_skill_store/src/manifest.rs` — add `tier`, `credits_per_execution` fields

**New module:** `crates/terraphim_skill_premium/`
```
terraphim_skill_premium/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── tiers.rs    # TierDefinition: Free, Pro, Enterprise; limits per tier
    ├── gating.rs   # SkillGate: evaluate skill chain against user tier
    └── registry.rs # PremiumSkillRegistry: metadata for all premium skills
```

**Skill definitions to create/update:**
1. `skills/premium-llm-gateway/skill.md` — Pro-tier LLM gateway with volume discounts
2. `skills/priority-queue/skill.md` — Enterprise-tier priority agent scheduling
3. `skills/advanced-analytics/skill.md` — Enterprise-tier usage analytics dashboard
4. Update existing `skills/post-llm-check/skill.md` — mark as Pro-tier

**Key impl steps (TDD):**
1. `TierDefinition::from_str("pro")` / `from_str("enterprise")` tests
2. `SkillGate::evaluate(skill_chain, user_tier)` tests
3. `PremiumSkillRegistry::register()` / `get_tier()` tests
4. Integration: `AgentDefinition::validate_skill_chain()` gated by user subscription

### 4.4 Verification

- `cargo test -p terraphim_skill_premium`
- `cargo clippy -p terraphim_skill_premium`
- E2E: Pro user can execute Pro skill; Free user blocked with 403
- E2E: Enterprise user can execute all skills

---

## Cross-Cutting Concerns

### Dependency Order
```
Workstream 2 (auth) ──→ Workstream 1 (billing) ──→ Workstream 4 (premium skills)
         │                      │
         └──────────────────────┴──→ Workstream 3 (skill store) ──→ Skill definitions
```

**Rationale:** Auth (user identity) needed before billing (who to charge). Billing needed before premium skill gating. Custom domain skill store is largely independent but builds on skill metadata from WS4.

### Shared Crate

**`crates/terraphim_user/`** — shared user model used by all workstreams:
```rust
pub enum SubscriptionTier { Free, Pro, Enterprise }
pub struct User { id, email, tier, seats, created_at, ... }
pub struct ApiKey { id, user_id, name, hash, created_at, ... }
```

### Secrets Management
All secrets via `op://` references through `terraphim_onepassword_cli::SecretLoader`:
- `op://TerraphimPlatform/stripe/secret_key`
- `op://TerraphimPlatform/stripe/webhook_secret`
- `op://TerraphimPlatform/gitea-oauth/client_secret`
- `op://TerraphimPlatform/jwt/signing_key`

### Testing Strategy
- Unit tests for all new crates
- Integration tests against live services (Stripe test mode, Gitea test instance)
- Feature flags: `#[cfg(feature = "stripe")]`, `#[cfg(feature = "gitea-oauth")]`

---

## Phase Gate: Research Complete

Before implementation begins, all four research documents must be approved. Each research document should answer:
1. What exists today (inventory)
2. What must be built (gap analysis)
3. Recommended approach with trade-offs
4. Risk register

**Gate criterion:** All four `.docs/research-*.md` files exist and reviewed.

---

## Next Steps (Immediate)

1. Create four research issues on Gitea
2. Run four parallel research agents
3. Upon research completion, gate for architecture design
4. Upon design approval, implement in dependency order
