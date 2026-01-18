# Design & Implementation Plan: Knowledge Graph Schema Linter

**Status:** Ready for Implementation
**Priority:** Medium
**Origin:** PR #294 (conflicting, extract KG linter only)
**Date:** 2025-12-31

---

## 1. Summary of Target Behavior

A CLI tool and library to validate Knowledge Graph markdown schemas:

1. **Validate KG markdown files** against schema rules
2. **Report lint issues** with severity, code, and message
3. **JSON output** for CI/CD integration
4. **Auto-fix capability** for common issues (future)
5. **Skill integration** for agentic loop validation

---

## 2. Key Components from PR #294

### New Crate: `terraphim_kg_linter`

```
crates/terraphim_kg_linter/
├── Cargo.toml
├── src/
│   ├── lib.rs      # Core linting logic
│   └── main.rs     # CLI binary
└── tests/
    └── basic.rs    # Integration tests
```

### Schema Structures

| Structure | Purpose |
|-----------|---------|
| `CommandDef` | Command definitions with args, permissions |
| `CommandArg` | Argument with name, type, required, default |
| `TypesBlock` | Type definitions (name -> field -> type) |
| `RolePermissions` | Role with allow/deny permission rules |
| `LintIssue` | Issue report with path, severity, code, message |

### Lint Rules

| Code | Severity | Description |
|------|----------|-------------|
| `E001` | Error | Missing required field |
| `E002` | Error | Invalid type reference |
| `E003` | Error | Undefined command reference |
| `W001` | Warning | Unused type definition |
| `W002` | Warning | Missing description |

---

## 3. Implementation Plan

### Step 1: Create Crate Structure

```bash
cargo new --lib crates/terraphim_kg_linter
```

**Cargo.toml dependencies:**
```toml
[dependencies]
regex = "1"
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1"
thiserror = "1"
walkdir = "2"
clap = { version = "4", features = ["derive"] }
terraphim_automata = { path = "../terraphim_automata" }

[dev-dependencies]
tempfile = "3"
```

### Step 2: Implement Core Types

- `LintError` enum with IO, YAML, Schema, Automata variants
- `CommandDef`, `CommandArg`, `TypesBlock`, `RolePermissions`
- `LintIssue` with severity levels
- `SchemaFragments` aggregating parsed schemas

### Step 3: Implement Linter

```rust
pub struct KgLinter {
    strict: bool,
    fragments: SchemaFragments,
}

impl KgLinter {
    pub fn new(strict: bool) -> Self;
    pub fn lint_directory(&mut self, path: &Path) -> Result<Vec<LintIssue>>;
    pub fn lint_file(&mut self, path: &Path) -> Result<Vec<LintIssue>>;
    fn validate_command(&self, cmd: &CommandDef) -> Vec<LintIssue>;
    fn validate_types(&self, types: &TypesBlock) -> Vec<LintIssue>;
    fn validate_permissions(&self, role: &RolePermissions) -> Vec<LintIssue>;
}
```

### Step 4: Implement CLI

```rust
#[derive(Parser)]
struct Cli {
    /// Path to KG directory
    #[arg(short, long, default_value = "docs/src/kg")]
    path: PathBuf,

    /// Output format
    #[arg(short, long, default_value = "text")]
    output: OutputFormat,

    /// Strict mode (warnings become errors)
    #[arg(long)]
    strict: bool,
}
```

### Step 5: Add to Workspace

Update root `Cargo.toml`:
```toml
members = [
    # ...
    "crates/terraphim_kg_linter",
]
```

### Step 6: Create Skill File

```yaml
# docs/src/skills/kg-schema-lint.skill.yaml
name: kg-schema-lint
description: Validate KG markdown schemas
steps:
  - run: cargo run -p terraphim_kg_linter -- --path $kg_path -o json --strict
  - parse: json
  - plan: minimal edits for issues
  - apply: edits
  - rerun: until exit code 0
```

### Step 7: CI Integration

Add to `.github/workflows/ci-native.yml`:
```yaml
- name: Lint KG schemas
  run: cargo run -p terraphim_kg_linter -- --path docs/src/kg --strict
```

---

## 4. Testing Strategy

| Test | Type | Location |
|------|------|----------|
| Valid schema passes | Unit | `tests/basic.rs` |
| Missing field detected | Unit | `tests/basic.rs` |
| Invalid type detected | Unit | `tests/basic.rs` |
| Directory scan works | Integration | `tests/basic.rs` |
| JSON output format | Integration | `tests/basic.rs` |
| CLI arguments | Integration | `tests/cli.rs` |

---

## 5. Risk Assessment

| Risk | Mitigation | Residual |
|------|------------|----------|
| PR #294 conflicts | Fresh implementation from extracted code | None |
| Schema format changes | Version schema format | Low |
| Performance on large KG | Lazy loading, parallel lint | Low |

---

## 6. Files to Create

| File | Action | Purpose |
|------|--------|---------|
| `crates/terraphim_kg_linter/Cargo.toml` | Create | Dependencies |
| `crates/terraphim_kg_linter/src/lib.rs` | Create | Core logic |
| `crates/terraphim_kg_linter/src/main.rs` | Create | CLI |
| `crates/terraphim_kg_linter/tests/basic.rs` | Create | Tests |
| `docs/src/skills/kg-schema-lint.skill.yaml` | Create | Skill def |
| `docs/src/kg/schema-linter.md` | Create | Documentation |

---

## 7. Implementation Timeline

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| Step 1-2 | 1 day | Crate structure, types |
| Step 3-4 | 2 days | Linter implementation, CLI |
| Step 5-7 | 1 day | Workspace, skill, CI |
| **Total** | **4 days** | Production-ready KG linter |

---

## 8. CLI Usage Examples

```bash
# Basic usage
cargo run -p terraphim_kg_linter -- --path docs/src/kg

# JSON output for CI
cargo run -p terraphim_kg_linter -- --path docs/src/kg -o json

# Strict mode (warnings become errors)
cargo run -p terraphim_kg_linter -- --path docs/src/kg --strict

# Single file
cargo run -p terraphim_kg_linter -- --file docs/src/kg/commands.md
```

---

## 9. Next Steps

1. Close PR #294 with comment linking to this plan
2. Create GitHub issue for KG linter implementation
3. Extract clean implementation from PR #294 branch
4. Implement following this plan

---

**Plan Status:** Ready for Implementation
