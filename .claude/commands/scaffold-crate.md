---
description: Generate Rust crate scaffold with tests, types, and docs
argument-hint: <crate-name>
---
# Scaffold Crate: $ARGUMENTS

1. Create crate directory: `crates/terraphim_$ARGUMENTS/`

2. **Generate Core Files:**

```
crates/terraphim_$ARGUMENTS/
├── Cargo.toml          # Crate manifest
├── src/
│   ├── lib.rs          # Main library entry
│   ├── error.rs        # Error types (thiserror)
│   ├── types.rs        # Type definitions
│   └── tests.rs        # Unit tests module
└── README.md           # Crate documentation
```

3. **Cargo.toml Template:**
- Edition 2024
- Workspace dependencies inheritance
- Feature flags for optional functionality
- Dev dependencies for testing

4. **Library Template (lib.rs):**
- Module declarations with rustdoc
- Public API exports
- Error type re-exports
- `#[cfg(test)]` test module

5. **Error Handling (error.rs):**
- Custom error enum with `thiserror`
- `Result<T>` type alias
- Error conversion traits

6. **Test Template (tests.rs):**
- `#[tokio::test]` for async tests
- Integration test structure
- No mocks - real implementations only
- Feature-gated live tests with `#[ignore]`

7. **Documentation:**
- Crate-level rustdoc in `lib.rs`
- README.md with:
  - Crate overview
  - Usage examples
  - Configuration options
  - API reference link

8. **Workspace Integration:**
- Add to `Cargo.toml` workspace members
- Update dependent crates if needed

9. **Git Integration:**
- Stage all files: `git add crates/terraphim_$ARGUMENTS/`
- Create feature branch: `git checkout -b feature/$ARGUMENTS`
