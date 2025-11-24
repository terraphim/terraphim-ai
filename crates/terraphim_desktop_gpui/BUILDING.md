# Building Terraphim Desktop GPUI

## Current Status: Prototype/Framework

⚠️ **IMPORTANT:** This crate cannot be built yet because GPUI dependencies are not finalized.

## Dependency Issues

### GPUI Framework

GPUI is still in pre-1.0 development and is **not published to crates.io**. It exists as part of the Zed editor monorepo.

**Two options to proceed:**

### Option 1: Use Git Dependencies (When Ready)

Update `Cargo.toml`:

```toml
[dependencies]
gpui = { git = "https://github.com/zed-industries/zed", branch = "main" }
```

**Challenges:**
- Zed's GPUI is tightly coupled to the editor
- May require extracting GPUI as standalone crate
- Breaking changes expected as GPUI approaches 1.0

### Option 2: Wait for GPUI 1.0 Release

The Zed team plans to release GPUI as a standalone crate. Monitor:
- [Zed GitHub Repo](https://github.com/zed-industries/zed)
- [GPUI Tracking Issue](https://github.com/zed-industries/zed/issues/XXXX) (TBD)

### gpui-component Library

[Longbridge's gpui-component](https://github.com/longbridge/gpui-component) depends on GPUI. Same limitations apply.

```toml
[dependencies]
gpui-component = { git = "https://github.com/longbridge/gpui-component", branch = "main" }
```

## Current Workarounds

### 1. Keep Tauri/Svelte Version

While GPUI implementation is in progress, continue using the production-ready Tauri version:

```bash
cd desktop
yarn run tauri dev
```

### 2. Mock GPUI for Development

Create stub types to allow Rust code to compile without GPUI:

```rust
// Mock GPUI types for compilation
mod gpui {
    pub struct App;
    pub struct AppContext;
    pub struct ViewContext<T> { _phantom: std::marker::PhantomData<T> }
    // ... more mocks
}
```

### 3. Parallel Development

Develop the business logic (state management, search integration) separately:

```bash
# Build just the state and integration layers
cargo build -p terraphim_service
cargo build -p terraphim_automata
```

Then integrate with GPUI views when the framework is available.

## Build Timeline Estimate

| Milestone | ETA | Status |
|-----------|-----|--------|
| GPUI 1.0 release | Q2-Q3 2025? | ⏳ Waiting |
| gpui-component stable | Q3-Q4 2025? | ⏳ Waiting |
| Terraphim GPUI prototype | 2 weeks after GPUI 1.0 | ✅ Ready |
| Terraphim GPUI beta | 8-10 weeks after GPUI 1.0 | 📋 Planned |

## Alternative: Prototype with Other Frameworks

If GPUI timeline is too uncertain, consider:

### Dioxus (Rust → Native UI)
```toml
dioxus = "0.6"
dioxus-desktop = "0.6"
```
- More mature than GPUI
- Published to crates.io
- Similar declarative UI model

### Iced (Elm-inspired Rust GUI)
```toml
iced = "0.13"
```
- Stable and well-documented
- Native performance
- Different architecture from GPUI

### Keep Tauri, Replace Svelte with Leptos/Dioxus
- Maintain web tech stack
- Better Rust integration than Svelte
- Less risk than full GPUI migration

## Next Steps

1. **Monitor GPUI Progress**
   - Watch Zed repo for GPUI extraction
   - Join Zed Discord for updates

2. **Prepare for Integration**
   - Continue developing business logic
   - Create adapter layer for UI framework
   - Write comprehensive tests for state management

3. **Prototype When Ready**
   - Update dependencies to GPUI git
   - Build minimal search interface
   - Benchmark performance vs Tauri

4. **Decision Point**
   - If GPUI stable: Proceed with migration
   - If delayed >6 months: Consider alternatives
   - Keep Tauri version as fallback

## Getting Help

- **Zed Discord**: https://discord.gg/zed
- **GPUI Discussions**: https://github.com/zed-industries/zed/discussions
- **Terraphim Team**: See main [CONTRIBUTING.md](../../CONTRIBUTING.md)

## Conclusion

The GPUI framework shows great promise, but timing is uncertain. This crate establishes the **architecture and patterns** for the migration, which can be adapted to GPUI or alternative frameworks when dependencies are available.

**For production use**: Continue with the Tauri/Svelte version in `desktop/`.
