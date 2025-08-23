# Database Backend Features

This document describes the optional database backend features available in Terraphim AI.

## Overview

By default, Terraphim AI uses lightweight, dependency-free storage backends:
- **Memory**: In-memory storage (no persistence)
- **DashMap**: Fast concurrent hash map storage 
- **ReDB**: Pure Rust embedded database (default for persistence)

Heavy database backends like SQLite and RocksDB are now **optional features** to:
- Reduce compilation time for development
- Minimize binary size for lightweight deployments
- Avoid heavy native dependencies when not needed

## Available Features

### Default Features
- `services-dashmap`: Fast in-memory concurrent storage
- `services-redb`: Pure Rust embedded database
- `services-atomicserver`: Atomic Server integration

### Optional Features  
- `services-sqlite`: SQLite database support
- `services-rocksdb`: RocksDB database support  
- `services-redis`: Redis backend support
- `full-db`: Enable all database backends

## Enabling Optional Features

### For the Server
```bash
# Enable SQLite only
cargo build --features sqlite

# Enable RocksDB only  
cargo build --features rocksdb

# Enable Redis only
cargo build --features redis

# Enable all database backends
cargo build --features full-db

# Enable with OpenRouter
cargo build --features "full-db,openrouter"
```

### For the Desktop App
```bash
# Enable SQLite support
cargo build --features sqlite

# Enable all database features
cargo build --features full-db
```

### Cargo.toml Configuration
Add to your `Cargo.toml` dependencies:

```toml
[dependencies]
terraphim_persistence = { path = "../crates/terraphim_persistence", features = ["services-sqlite", "services-rocksdb"] }
```

## Configuration Files

When optional database features are disabled, the corresponding profiles in configuration files are commented out:

### Default Configuration (ReDB)
```toml
[profiles.memory]
type = "memory"

[profiles.redb]
type = "redb"
datadir = "/tmp/terraphim_redb"

[profiles.dashmap]
type = "dashmap"
root = "/tmp/terraphim_dashmap"
```

### With Optional Backends Enabled
Uncomment the desired profiles:
```toml
# Uncomment if SQLite feature is enabled
[profiles.sqlite]
type = "sqlite"
datadir = "/tmp/terraphim_sqlite"

# Uncomment if RocksDB feature is enabled
[profiles.rocksdb]
type = "rocksdb"
datadir = "/tmp/terraphim_rocksdb"
```

## Performance Characteristics

| Backend | Speed | Memory Usage | Disk Usage | Dependencies |
|---------|-------|--------------|------------|--------------|
| Memory  | Fastest | High | None | None |
| DashMap | Very Fast | Medium | None | None |
| ReDB    | Fast | Low | Low | None |
| SQLite  | Medium | Low | Medium | libsqlite3-sys |
| RocksDB | Fast | Medium | Low | librocksdb-sys |

## Migration Guide

### From Full Dependencies to Optional
If you were previously relying on SQLite or RocksDB being available by default:

1. **Update build commands** to include the required features
2. **Update configuration files** to uncomment the database profiles
3. **Update CI/CD pipelines** to build with appropriate features

### Example CI Configuration
```yaml
# Fast builds without heavy databases
- name: Quick Build
  run: cargo build

# Full build with all features
- name: Full Build  
  run: cargo build --features full-db
```

## Troubleshooting

### "Scheme not supported" Error
If you see an error like:
```
Error: Unsupported scheme: sqlite
```

This means the SQLite feature is not enabled. Either:
1. Remove SQLite profiles from your configuration, or
2. Build with `--features sqlite`

### "Profile not found" Error
If you see:
```
Error: Profile 'rocksdb' not found
```

This means your configuration references a RocksDB profile but the feature isn't enabled. Either:
1. Comment out RocksDB profiles in configuration files, or
2. Build with `--features rocksdb`

## Recommendations

- **Development**: Use default features (ReDB/DashMap) for fast builds
- **Production**: Use ReDB for reliability, SQLite for compatibility, RocksDB for performance
- **CI/CD**: Test both with and without optional features
- **Lightweight deployments**: Use default features only