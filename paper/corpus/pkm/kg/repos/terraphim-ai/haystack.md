# Haystack

Data source descriptor for the knowledge graph search pipeline. Each haystack has a location (filesystem path or URL), service type (Ripgrep, JMAP, Quickwit, GrepApp, MCP, etc.), and optional parameters. HaystackProvider trait provides uniform async search interface over heterogeneous backends.

synonyms:: haystack, data source, search backend, service type, haystack_core, haystack provider
