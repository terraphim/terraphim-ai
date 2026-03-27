# terraphim_agent/src/service.rs Summary

## Purpose
Provides the TUI service layer that manages application state, configuration, and business logic for the Terraphim agent's text-based user interface, coordinating between configuration persistence and the core TerraphimService.

## Key Functionality
- Manages configuration loading with priority: CLI flag → settings.toml → persistence → embedded defaults
- Resolves role strings to RoleName objects by searching configuration (name first, then shortname)
- Provides access to thesaurus data for roles via embedded automata
- Handles chat interactions with LLM providers based on role configuration
- Implements document search, extraction, summarization, and connectivity checking
- Manages role graph operations and topological analysis
- Supports configuration persistence and reloading from JSON files
- Provides checklist validation functionality for various domains (code review, security, etc.)

## Important Details
- Role resolution returns error if role not found in config (line 251), unlike client.rs which falls back to raw string
- Uses Arc<Mutex<TerraphimService>> for shared state access across async contexts
- Implements bootstrap-then-persistence pattern for role_config in settings.toml
- Supports multiple search modes: simple term search and complex SearchQuery with operators
- Provides thesaurus-backed text operations: extraction, finding matches, autocomplete, fuzzy suggestions
- Includes connectivity checking for knowledge graph relationships between matched terms
- Handles device settings with fallback to embedded defaults in sandboxed environments
- Manages configuration updates and persistence through save_config() method