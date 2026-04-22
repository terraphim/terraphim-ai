# Summary: .pre-commit-config.yaml

**Purpose:** Pre-commit hooks for code quality.

**Key Details:**
- Python version: 3.9
- Hooks:
  1. `trailing-whitespace`, `end-of-file-fixer`, `check-yaml`, `check-toml`, `check-json`
  2. `check-case-conflict`, `check-merge-conflict`, `debug-statements`, `detect-private-key`
  3. `check-added-large-files` (max 1000KB)
  4. `cargo-fmt` (system): `cargo fmt --all -- --check`
  5. `cargo-clippy` (system): `cargo clippy --workspace --all-targets -- -D warnings`
  6. `cargo-test` (system, manual stage): `cargo test --workspace`
  7. `cargo-audit` (system, manual stage): `cargo audit`
  8. `biome-check` (system): `cd desktop && npx @biomejs/biome check --no-errors-on-unmatched`
  9. `biome-format` (system, manual stage): `cd desktop && npx @biomejs/biome format --write --no-errors-on-unmatched`
  10. `detect-secrets` with baseline `.secrets.baseline`
- Global exclusions: target/, desktop/node_modules/, desktop/dist/, desktop/src-tauri/target/, vendor/
