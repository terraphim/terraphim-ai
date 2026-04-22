# Summary: terraphim_persistence/src/lib.rs

**Purpose:** Unified persistence layer with multiple backend support.

**Key Details:**
- Modules: `compression`, `conversation`, `document`, `error`, `memory`, `settings`, `thesaurus`
- Core trait: `Persistable` for async save/load operations
- Uses `opendal::Operator` for abstraction over storage backends
- `DeviceStorage`: singleton pattern with `AsyncOnceCell` for lazily-initialized storage
- Storage backends supported: filesystem, memory, redis, s3, redb, ipfs (feature-gated)
- `init_memory_only()`: for tests that don't need filesystem
- `arc_instance()`: safe alternative to unsafe pointer operations
- Tildef expansion helper for home directory paths
