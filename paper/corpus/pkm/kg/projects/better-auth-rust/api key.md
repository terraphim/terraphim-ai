# API Key

Machine-to-machine authentication mechanism in better-auth-rust, used by both services and AI agents.

## Entity Structure

```rust
pub struct ApiKey {
    pub id: Uuid,
    pub key_hash: String,       // SHA-256 hash
    pub key_prefix: String,     // First 8 chars for identification
    pub name: String,
    pub agent_id: Uuid,
    pub expires_at: Option<DateTime>,
    pub last_used: Option<DateTime>,
    pub revoked: bool,
}
```

## Lifecycle

1. Created for an Agent or User
2. Key shown once at creation (only hash stored)
3. Validated on each request via hash comparison
4. Optional expiry and automatic rotation
5. Revocable by owner or admin

## Security

- SHA-256 hashed at rest (never stored in plaintext)
- Prefix-based identification without exposing key
- Scoped to agent capabilities
- Audit-logged on creation, use, and revocation
