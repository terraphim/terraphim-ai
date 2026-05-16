# Summary: terraphim_persistence/src/lib.rs

**Purpose:** Unified storage abstraction layer over multiple OpenDAL-backed operators with cache hierarchy.

**Architecture:**
- Operators ordered by latency; slowest to fastest
- Transparent write-back to fastest operator via fire-and-forget tokio::spawn
- Objects >1MB compressed with zstd before storage
- Schema evolution detection: failed deserialization triggers cache eviction

**Key Types:**
- **`DeviceStorage`**: Singleton wrapping HashMap<String, (Operator, u128)> with latency tracking
- **`Persistable`**: blanket trait for serializable types
- **`ConversationPersistence`**: trait for storing/loading conversations

**Supported Storage Backends:**
- Memory ( DashMap)
- SQLite
- ReDB (embedded key-value store)
- S3 and other cloud storage via OpenDAL

**Cache Write-back Behavior:**
- Non-blocking via `tokio::spawn`
- Best-effort (failures logged at debug level)
- Compressed transfer for large objects
- Schema evolution: If cached data fails to deserialize, cache is cleared and refetched

**Loading Strategy:**
1. Try fastest operator first
2. On cache miss/error, try all operators in speed order
3. On success from fallback, asynchronously write to fastest operator

**Concurrency:**
- `AsyncOnceCell` for process-wide singleton initialization
- `load_documents_by_ids()` uses `tokio::task::JoinSet` for parallel loading

**Compression:**
- `maybe_compress()`: zstd compression for objects >1MB
- `maybe_decompress()`: Automatic decompression on read

**Error Handling:**
- `Error` enum with variants: NotFound, Json, OpenDal, Profile, NoOperator
- Result type alias for ergonomic error propagation