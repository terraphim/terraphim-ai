# Session

Persistent session management in better-auth-rust with secure rotation and multi-device support.

## Features

- Server-side session storage (database-backed)
- Session rotation on privilege escalation
- Concurrent session management per user
- Session revocation (individual and bulk)
- Configurable expiry and idle timeout
- Cookie-based session binding

## Architecture

Built on tower-sessions with Rust-specific extensions for:
- Type-safe session data
- Integration with auth middleware
- Database persistence via sqlx
