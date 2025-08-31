# Test KG Auto-linking Demo

This document demonstrates the KG auto-linking functionality.

When using the Terraphim Engineer role with `terraphim_it: true`, terms from the knowledge graph should automatically become clickable links:

- The **graph** concept is central to our architecture
- Our **haystack** system indexes documents efficiently
- The **service** layer provides core functionality
- **graph embeddings** improve semantic search

These terms should be converted to markdown links with the `kg:` protocol when processed by the system.

## Testing

To test this functionality:
1. Load this document with the Terraphim Engineer role
2. Check if KG terms become `[term](kg:term)` links
3. Verify the links are clickable in the frontend

The preprocessing should convert:
- `graph` → `[graph](kg:graph)`
- `haystack` → `[haystack](kg:haystack)`
- `service` → `[service](kg:service)`
- `graph embeddings` → `[graph embeddings](kg:graph-embeddings)`
