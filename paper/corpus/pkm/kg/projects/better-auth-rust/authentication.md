# Authentication

The core capability provided by better-auth-rust: comprehensive, type-safe authentication for Rust web applications.

## Components

- Email/password with bcrypt hashing
- OAuth2/social login (GitHub, Google, etc.)
- Session management with rotation
- API key generation, management, revocation
- Two-factor authentication (TOTP)
- Email OTP
- Passkey/WebAuthn support (parity phase 2)
- Enterprise SSO (SAML, OIDC) (parity phase 3)
- CLI authentication (parity phase 4)
- Adapters for various frameworks (parity phase 5)

## Design Philosophy

- "Just works" experience -- developer gets full auth in < 30 minutes
- Type safety catches auth errors at compile time, not runtime
- Unified API surface instead of stitching multiple crates
- Plugin architecture for custom auth flows
