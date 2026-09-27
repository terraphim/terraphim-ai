# CXDB

AI Context Store (Apache-2.0). Three-tier architecture: React/Next.js frontend (port 3000), Go gateway with OAuth proxy (port 8080), Rust storage engine (binary protocol 9009, HTTP 9010). Turn DAG stores immutable conversation nodes with BLAKE3 content hashes, parent references, and depth tracking. Blob CAS provides content-addressed storage with Zstd compression. Forking is O(1) -- new contexts point to existing turns without history duplication. Type registry enables forward-compatible schema evolution (Go writers -> numeric field tags -> Rust msgpack projection -> typed JSON). JavaScript renderers with ESM/CDN loading and CSP sandboxing. Built entirely by agents.

synonyms:: ai context store, context database, agent context dag, cx database, conversation history store, agent interaction log, turn-based context management
