# Data Proxy

Cache-first data access pattern implemented in the Data Hub. Reads check KV cache first with Atomic Server fallback on miss. Writes update Atomic Server (source of truth) then KV cache. Feature flag USE_KV_AS_SOURCE_OF_TRUTH enables KV-first writes with async Atomic sync.

synonyms:: dataproxy, cache proxy
