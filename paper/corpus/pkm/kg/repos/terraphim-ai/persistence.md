# Persistence

Unified storage abstraction over OpenDAL operators with cache hierarchy. Operators ordered by latency (slowest to fastest). Supports Memory (DashMap), SQLite, ReDB, S3 backends. Objects over 1MB compressed with zstd. Persistable trait for async save/load with schema evolution detection.

synonyms:: storage, persistence, opendal, Persistable, cache, terraphim_persistence
