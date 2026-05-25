# Session: Implement config validation with field-level errors (#1334)

**Started**: 2026-05-25T08:00
**Issue**: #1334 - feat(config): validate role config JSON against schema at startup and emit field-level errors
**Agent**: implementation-swarm-A
**Branch**: task/1334-config-schema-validation

## Context

The `terraphim_config` crate already has `schemars` as a dependency and `JsonSchema` derives on `Role`, `Haystack`, `Config`, and related structs. But there is NO validation function — config loading goes directly to `serde_json::from_str` which produces cryptic errors on malformed input.

## Plan

1. Add `ValidationError` struct and `validate_config()` function to `terraphim_config`
2. Use `schemars::schema_for!` + JSON Schema validation to produce field-level errors
3. Wire validation into `Config::load_from_json_file()` and the config load path
4. Add comprehensive unit tests

## Key Files
- `crates/terraphim_config/src/lib.rs` — main config types and load path
- `crates/terraphim_config/Cargo.toml` — dependencies
