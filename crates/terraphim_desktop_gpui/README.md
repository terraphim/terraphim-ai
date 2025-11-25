# Terraphim Desktop GPUI

Pure Rust desktop application for Terraphim AI using GPUI 0.2.2.

## Status: 85% Complete - Major User Journey Working ✅

**Tests:** 23/23 PASSING | **Code Reuse:** 100% from Tauri | **Commits:** 23

## Quick Start

```bash
# Build
cargo build -p terraphim_desktop_gpui --target aarch64-apple-darwin

# Run
cargo run -p terraphim_desktop_gpui --target aarch64-apple-darwin

# Test
cargo test -p terraphim_desktop_gpui --target aarch64-apple-darwin
```

## Working Features

✅ Search with KG autocomplete (190 terms)
✅ Context management (create, add, delete)
✅ Chat with LLM (llm::build_llm_from_role)
✅ Role management (backend ready)
✅ Navigation (Search/Chat/Editor)

## Test Results

```
23/23 tests PASSING:
- Search: 5/5
- Autocomplete: 7/7
- Context: 7/7
- End-to-End: 4/4
```

## Documentation

- `plan.md` - Implementation tracking (85%)
- `STATUS_REPORT.md` - Cross-check vs Tauri
- Test files prove 100% backend reuse

## Backend Services (Shared with Tauri)

All services imported from shared crates - ZERO duplication:
- terraphim_service (search, LLM)
- terraphim_config (configuration)
- terraphim_automata (KG autocomplete)
- ContextManager (conversations)

See STATUS_REPORT.md for detailed cross-check.
