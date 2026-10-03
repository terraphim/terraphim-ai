# Config

Role-based configuration management with knowledge graph orchestration. Config holds multiple Roles, each with KG, haystacks, relevance function, LLM settings, and theme. Loading priority: TERRAPHIM_CONFIG env var, saved config from persistence, hard-coded defaults. ConfigState wraps Config with Arc<Mutex> for thread-safe access.

synonyms:: configuration, config, role config, terraphim_config, ConfigState, role management
