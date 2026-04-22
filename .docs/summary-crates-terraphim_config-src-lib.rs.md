# Summary: terraphim_config/src/lib.rs

**Purpose:** Configuration management and role-based knowledge graph setup.

**Key Details:**
- Core type: `ConfigState` - manages role configurations and their associated graphs
- Uses `terraphim_automata` for thesaurus loading and building
- Uses `terraphim_rolegraph` for `RoleGraph` and `RoleGraphSync`
- Uses `terraphim_persistence::Persistable` for saving/loading config
- Uses `terraphim_settings::DeviceSettings` for device-specific settings
- Error type: `TerraphimConfigError` with variants for NotFound, NoRoles, Persistence, Json, etc.
- LLM router configuration in `llm_router` module
- Shell-like variable expansion in paths (`expand_tilde`)
- Async trait implementations for config loading
- TypeScript definitions available with `typescript` feature (tsify)
