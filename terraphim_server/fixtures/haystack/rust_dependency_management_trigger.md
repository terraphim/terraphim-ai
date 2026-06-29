# Rust Dependency Management

A knowledge-graph entry covering how Rust projects declare, lock, and audit
third-party crates using Cargo. Trigger- and pinned-based retrieval relies on
the directives below; the body text is ordinary prose so the entry also
participates in normal synonym (Aho-Corasick) matching.

synonyms:: cargo package management, rust dependency, dependency management in rust, cargo crates
trigger:: when managing rust cargo dependencies and crate versions
pinned:: true

## Topics

- `Cargo.toml` manifest and the `[dependencies]` table
- `Cargo.lock` for reproducible, locked builds
- `cargo update` to advance crate versions within semver bounds
- Auditing the dependency tree with `cargo audit`

## See also

- `rust_example.md` for general Rust programming patterns
- `testing_strategies.md` for test-organisation practices
